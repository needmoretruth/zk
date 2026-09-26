//! The hands-on activities as the command line asks for them, parsed by the same grammar as the
//! screen's slash commands so both accept the same words and say the same things about mistakes.

use std::path::{Path, PathBuf};

use zk_i18n::Language;

use super::Pace;
use super::cave::CaveRun;
use super::ceremony::CeremonyRun;
use super::forge::ForgeRun;
use super::pool::store::{chosen_system, data_root};
use super::pool::{PoolAction, PoolCache, PoolRun};
use super::trio::TrioRun;
use crate::activity::Activity;
use crate::commands::{self, Command};
use crate::phrases::fill;
use crate::phrases::pool::Msg as P;

/// One activity, ready to run.
#[derive(Clone, Debug)]
pub enum Request {
    /// `cave …`
    Cave(CaveRun),
    /// `trio …`
    Trio(TrioRun),
    /// `pool …`
    Pool(PoolRun),
    /// `ceremony …`
    Ceremony(CeremonyRun),
    /// `forge …`
    Forge(ForgeRun),
}

impl Request {
    /// The activity to hand to [`super::collect`].
    pub fn into_activity(self) -> Box<dyn Activity> {
        match self {
            Request::Cave(run) => Box::new(run),
            Request::Trio(run) => Box::new(run),
            Request::Pool(run) => Box::new(run),
            Request::Ceremony(run) => Box::new(run),
            Request::Forge(run) => Box::new(run),
        }
    }

    /// Whether this is a pool reset nobody has confirmed yet.
    pub fn unconfirmed_reset(&self) -> bool {
        matches!(self, Request::Pool(run) if run.action == PoolAction::Reset { confirmed: false })
    }

    /// The same request with a reset confirmed.
    pub fn confirm_reset(mut self) -> Request {
        if let Request::Pool(run) = &mut self
            && let PoolAction::Reset { confirmed } = &mut run.action
        {
            *confirmed = true;
        }
        self
    }
}

/// Parses `command` (`cave`, `trio`, `pool`, `ceremony`, `forge`) with its words, flags included,
/// into a request that runs without pauses. The error is the sentence the screen would show, with
/// its commands spelled the way a shell takes them.
pub fn request(
    command: &str,
    words: &[String],
    language: Language,
    data_dir: Option<PathBuf>,
) -> Result<Request, String> {
    let line = format!("/{command} {}", words.join(" "));
    let parsed = commands::parse(&line)
        .map_err(|error| cli_spelling(&commands::error_text(error, language)))?;
    let pace = Pace::Instant;
    Ok(match parsed {
        Command::Cave { mode, example, scenes } => {
            Request::Cave(CaveRun { mode, example, scenes, language, pace })
        }
        Command::Trio { mode, example, rounds } => {
            Request::Trio(TrioRun { mode, example, rounds, language, pace })
        }
        Command::Pool(action) => {
            Request::Pool(PoolRun { action, data_dir, cache: PoolCache::default(), language })
        }
        Command::Ceremony { part, participants } => {
            Request::Ceremony(CeremonyRun { part, participants, language, pace })
        }
        Command::Forge(target) => Request::Forge(ForgeRun { target, language, pace }),
        _ => return Err(commands::error_text(commands::ParseError::Unknown(line), language)),
    })
}

/// `text` with every slash command written as the command line takes it: `/pool send …` becomes
/// `nmtzk pool send …`, `/help` becomes `nmtzk --help`.
fn cli_spelling(text: &str) -> String {
    const COMMANDS: [&str; 10] =
        ["list", "about", "examples", "run", "cave", "trio", "pool", "ceremony", "forge", "help"];
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(slash) = rest.find('/') {
        let (before, after) = rest.split_at(slash);
        out.push_str(before);
        let starts_word = out.is_empty() || out.ends_with([' ', '(', '\'', '"']);
        let command = COMMANDS.iter().find(|command| {
            after[1..].strip_prefix(**command).is_some_and(|tail| {
                !tail.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-')
            })
        });
        match command {
            Some(&"help") if starts_word => {
                out.push_str("nmtzk --help");
                rest = &after[1 + "help".len()..];
            }
            Some(command) if starts_word => {
                out.push_str("nmtzk ");
                out.push_str(command);
                rest = &after[1 + command.len()..];
            }
            _ => {
                out.push('/');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The question the command line asks before a reset, naming the pool it would empty.
pub fn reset_question(data_dir: Option<&Path>, language: Language) -> String {
    let system = data_root(data_dir).and_then(|root| chosen_system(&root).ok()).unwrap_or_default();
    fill(P::ResetPrompt.text(language), &[("system", system.name())])
}

/// What the command line says when a reset was not confirmed.
pub fn reset_declined(language: Language) -> &'static str {
    P::ResetKept.text(language)
}

#[cfg(test)]
mod tests {
    use super::cli_spelling;

    #[test]
    fn slash_commands_are_spelled_for_the_shell() {
        assert_eq!(
            cli_spelling("Usage: /pool send <from> <to> <amount>"),
            "Usage: nmtzk pool send <from> <to> <amount>"
        );
        assert_eq!(
            cli_spelling("Unknown example 'x'. Type /examples for the list."),
            "Unknown example 'x'. Type nmtzk examples for the list."
        );
        assert_eq!(cli_spelling("Type /help."), "Type nmtzk --help.");
        assert_eq!(cli_spelling("a/b and /poolside and 1/2"), "a/b and /poolside and 1/2");
    }
}
