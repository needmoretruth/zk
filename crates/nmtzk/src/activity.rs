//! The hands-on activities on the command line: the same cells as the screen, without pauses.

use std::io::{BufRead, IsTerminal};
use std::process::ExitCode;

use zk_core::Control;
use zk_tui::activities::{self, Request};

use crate::console;
use crate::print::{Context, USAGE, write_out};

/// Runs `command` with `words` and prints its cells, or its records as JSON lines.
///
/// Exits 2 when the words do not parse or a reset was not confirmed, 1 when a cell reports an
/// error or the ledger refused a transaction the reader asked for, and 0 otherwise. An attack's
/// refusals are what it came to show, so they do not count.
pub(crate) fn run(command: &str, words: &[String], context: &Context) -> ExitCode {
    let language = context.language;
    let request = match activities::request(command, words, language, context.data_dir.clone()) {
        Ok(request) => request,
        Err(message) => {
            console::write_err(&format!("{}\n", context.printer.line(&message)));
            return ExitCode::from(USAGE);
        }
    };
    let (request, declined) = confirm(request, context);
    if declined {
        console::write_err(&format!("{}\n", activities::reset_declined(language)));
        return ExitCode::from(USAGE);
    }
    let unconfirmed = request.unconfirmed_reset();
    let collected =
        activities::collect(request.into_activity(), &Control::new(), language, progress);
    clear_progress();
    if context.json {
        for record in &collected.records {
            context.json_line(record);
        }
    } else {
        for (index, entry) in collected.entries.iter().enumerate() {
            if index > 0 {
                write_out("\n");
            }
            write_out(&context.printer.entry(entry));
        }
    }
    if unconfirmed {
        ExitCode::from(USAGE)
    } else if collected.failed() || refused(&collected.records) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Whether a pool receipt says the ledger refused the transaction. Attacks report their steps, not a
/// receipt, so their refusals never match.
fn refused(records: &[serde_json::Value]) -> bool {
    records.iter().any(|record| record["result"]["decision"]["accepted"] == false)
}

/// Asks before a reset when someone is at the terminal. Returns the request, confirmed if they
/// typed `yes`, and whether they declined. Without a terminal the reset stays unconfirmed, and
/// the activity says how to confirm it; so it does when standard error is not the terminal, since
/// the question would go where nobody reads it and the program would wait for an answer.
fn confirm(request: Request, context: &Context) -> (Request, bool) {
    let someone_asked = std::io::stdin().is_terminal() && console::err().is_terminal();
    if !request.unconfirmed_reset() || !someone_asked {
        return (request, false);
    }
    let question = activities::reset_question(context.data_dir.as_deref(), context.language);
    console::write_err(&format!("{question} "));
    let mut answer = String::new();
    let read = std::io::stdin().lock().read_line(&mut answer);
    if read.is_ok() && agrees(answer.trim(), context.language) {
        (request.confirm_reset(), false)
    } else {
        (request, true)
    }
}

/// Whether `answer` says yes: the English word in every language, and 예 or 네 in Korean.
fn agrees(answer: &str, language: zk_i18n::Language) -> bool {
    answer.eq_ignore_ascii_case("yes")
        || (language == zk_i18n::Language::KOREAN && matches!(answer, "예" | "네"))
}

/// Shows what the activity is doing on standard error while it is a terminal that can erase it.
fn progress(status: &str) {
    if console::live_status() {
        console::write_err(&format!("\r\x1b[2K{status}"));
    }
}

fn clear_progress() {
    if console::live_status() {
        console::write_err("\r\x1b[2K");
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use zk_i18n::Language;

    use super::{agrees, refused};

    #[test]
    fn yes_is_yes_in_every_language_and_korean_has_its_own() {
        assert!(agrees("YES", Language::ENGLISH) && agrees("yes", Language::KOREAN));
        assert!(agrees("예", Language::KOREAN) && agrees("네", Language::KOREAN));
        assert!(!agrees("예", Language::ENGLISH) && !agrees("y", Language::ENGLISH));
    }

    #[test]
    fn only_a_refused_receipt_counts_as_a_refusal() {
        let receipt = |accepted| json!({ "result": { "decision": { "accepted": accepted } } });
        assert!(refused(&[json!({ "result": { "wallets": [] } }), receipt(false)]));
        assert!(!refused(&[receipt(true)]));
        let faucet = json!({ "result": { "decision": null } });
        let attack = json!({ "result": { "steps": [{ "decision": { "accepted": false } }] } });
        assert!(!refused(&[faucet, attack]));
    }
}
