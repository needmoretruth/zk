//! The hands-on activities on the command line: the same cells as the screen, without pauses.

use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

use zk_core::Control;
use zk_tui::activities::{self, Request};

use crate::print::{Context, USAGE, write_out};

/// Runs `command` with `words` and prints its cells, or its records as JSON lines.
///
/// Exits 2 when the words do not parse or a reset was not confirmed, 1 when a cell reports an
/// error, and 0 otherwise.
pub(crate) fn run(command: &str, words: &[String], context: &Context) -> ExitCode {
    let language = context.language;
    let request = match activities::request(command, words, language, context.data_dir.clone()) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(USAGE);
        }
    };
    let (request, declined) = confirm(request, context);
    if declined {
        eprintln!("{}", activities::reset_declined(language));
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
    } else if collected.failed() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Asks before a reset when someone is at the terminal. Returns the request, confirmed if they
/// typed `yes`, and whether they declined. Without a terminal the reset stays unconfirmed, and
/// the activity says how to confirm it.
fn confirm(request: Request, context: &Context) -> (Request, bool) {
    if !request.unconfirmed_reset() || !std::io::stdin().is_terminal() {
        return (request, false);
    }
    let question = activities::reset_question(context.data_dir.as_deref(), context.language);
    eprint!("{question} ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    let read = std::io::stdin().lock().read_line(&mut answer);
    if read.is_ok() && answer.trim().eq_ignore_ascii_case("yes") {
        (request.confirm_reset(), false)
    } else {
        (request, true)
    }
}

/// Shows what the activity is doing on standard error while it is a terminal.
fn progress(status: &str) {
    if std::io::stderr().is_terminal() {
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K{status}");
        let _ = err.flush();
    }
}

fn clear_progress() {
    if std::io::stderr().is_terminal() {
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K");
        let _ = err.flush();
    }
}
