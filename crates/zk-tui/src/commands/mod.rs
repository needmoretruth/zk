//! The slash commands: their names, their arguments and how a typed line becomes one.
//!
//! The same table feeds `/help`, the completion popup and the parser, so a new command is one
//! entry here plus one arm where commands are carried out.

mod activities;

use zk_core::ExampleId;
use zk_core::catalog::Shelf;
use zk_i18n::Language;

use crate::activities::cave::CaveMode;
use crate::activities::ceremony::CeremonyPart;
use crate::activities::forge::ForgeTarget;
use crate::activities::pool::PoolAction;
use crate::activities::trio::TrioMode;
use crate::phrases::fill;
use crate::phrases::ui::Msg;

/// What an argument position accepts, which decides its completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arg {
    System,
    SystemOrAll,
    Example,
    Shelf,
    Language,
    CaveMode,
    TrioMode,
    /// A cheat after `cheat`, otherwise an example.
    TrioDetail,
    CeremonyPart,
    ForgeTarget,
    PoolAction,
    /// What follows a pool action: a system, an attack, `new`.
    PoolDetail,
    /// A name or an amount: nothing to offer.
    Free,
}

/// One command as the popup and `/help` show it.
#[derive(Debug)]
pub(crate) struct Spec {
    pub(crate) name: &'static str,
    pub(crate) usage: &'static str,
    /// Argument kinds in order, with whether each is required.
    pub(crate) args: &'static [(Arg, bool)],
    /// Flags it takes; `--yes` stands alone, every other flag takes a value.
    pub(crate) flags: &'static [&'static str],
    pub(crate) summary: Msg,
}

pub(crate) const SPECS: &[Spec] = &[
    Spec { name: "/help", usage: "/help", args: &[], flags: &[], summary: Msg::CmdHelp },
    Spec {
        name: "/list",
        usage: "/list [shelf]",
        args: &[(Arg::Shelf, false)],
        flags: &[],
        summary: Msg::CmdList,
    },
    Spec {
        name: "/about",
        usage: "/about <system>",
        args: &[(Arg::System, true)],
        flags: &[],
        summary: Msg::CmdAbout,
    },
    Spec {
        name: "/examples",
        usage: "/examples",
        args: &[],
        flags: &[],
        summary: Msg::CmdExamples,
    },
    Spec {
        name: "/run",
        usage: "/run <system|all> <example>",
        args: &[(Arg::SystemOrAll, true), (Arg::Example, true)],
        flags: &[],
        summary: Msg::CmdRun,
    },
    Spec {
        name: "/cave",
        usage: "/cave [mode] [example] [--scenes N]",
        args: &[(Arg::CaveMode, false), (Arg::Example, false)],
        flags: &["--scenes"],
        summary: Msg::CmdCave,
    },
    Spec {
        name: "/trio",
        usage: "/trio [mode] [example] [--rounds N]",
        args: &[(Arg::TrioMode, false), (Arg::TrioDetail, false), (Arg::Example, false)],
        flags: &["--rounds"],
        summary: Msg::CmdTrio,
    },
    Spec {
        name: "/pool",
        usage: "/pool [action] …",
        args: &[
            (Arg::PoolAction, false),
            (Arg::PoolDetail, false),
            (Arg::Free, false),
            (Arg::Free, false),
        ],
        flags: &["--yes"],
        summary: Msg::CmdPool,
    },
    Spec {
        name: "/ceremony",
        usage: "/ceremony [part] [--participants N]",
        args: &[(Arg::CeremonyPart, false)],
        flags: &["--participants"],
        summary: Msg::CmdCeremony,
    },
    Spec {
        name: "/forge",
        usage: "/forge bctv14",
        args: &[(Arg::ForgeTarget, true)],
        flags: &[],
        summary: Msg::CmdForge,
    },
    Spec {
        name: "/lang",
        usage: "/lang <en|ko>",
        args: &[(Arg::Language, true)],
        flags: &[],
        summary: Msg::CmdLang,
    },
    Spec { name: "/clear", usage: "/clear", args: &[], flags: &[], summary: Msg::CmdClear },
    Spec { name: "/quit", usage: "/quit", args: &[], flags: &[], summary: Msg::CmdQuit },
];

/// Which systems a run covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    All,
    System(String),
}

/// A parsed command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    List(Option<Shelf>),
    About(String),
    Examples,
    Run { target: Target, example: ExampleId },
    Cave { mode: CaveMode, example: ExampleId, scenes: u32 },
    Trio { mode: TrioMode, example: ExampleId, rounds: u32 },
    Pool(PoolAction),
    Ceremony { part: CeremonyPart, participants: usize },
    Forge(ForgeTarget),
    Lang(Language),
    Clear,
    Quit,
}

/// Why a line is not a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    Empty,
    NoSlash,
    Unknown(String),
    Usage(&'static str),
    Shelf(String),
    Example(String),
    Language(String),
    /// A word that is none of the choices a position offers.
    Choice {
        value: String,
        choices: String,
    },
    /// A flag's value is not a whole number from 1 to `max`.
    Count {
        flag: &'static str,
        value: String,
        max: u32,
    },
    /// An amount is not a whole number from 0 to 65,535.
    Amount(String),
}

/// Flags found on a line: the switches given and each valued flag with its value.
pub(crate) type Flags<'a> = Vec<(&'static str, Option<&'a str>)>;

/// Parses a typed line. A known command name without its slash is accepted too.
pub(crate) fn parse(line: &str) -> Result<Command, ParseError> {
    let mut words = line.split_whitespace();
    let Some(first) = words.next() else { return Err(ParseError::Empty) };
    let name = if first.starts_with('/') { first.to_string() } else { format!("/{first}") };
    let Some(spec) = SPECS.iter().find(|spec| spec.name == name) else {
        return Err(if first.starts_with('/') {
            ParseError::Unknown(first.to_string())
        } else {
            ParseError::NoSlash
        });
    };
    let (args, flags) = split_flags(spec, words)?;
    let required = spec.args.iter().filter(|(_, required)| *required).count();
    if args.len() < required || args.len() > spec.args.len() {
        return Err(ParseError::Usage(spec.usage));
    }
    build(spec.name, &args, &flags)
}

/// Separates `--flags` from positional words; a flag the command does not take, or a valued flag
/// with nothing after it, is a usage error.
fn split_flags<'a>(
    spec: &Spec,
    mut words: impl Iterator<Item = &'a str>,
) -> Result<(Vec<&'a str>, Flags<'a>), ParseError> {
    let (mut args, mut flags) = (Vec::new(), Vec::new());
    while let Some(word) = words.next() {
        if !word.starts_with("--") {
            args.push(word);
            continue;
        }
        let flag = spec.flags.iter().find(|flag| **flag == word);
        let flag = flag.ok_or(ParseError::Usage(spec.usage))?;
        let value = if *flag == "--yes" {
            None
        } else {
            Some(words.next().ok_or(ParseError::Usage(spec.usage))?)
        };
        flags.push((*flag, value));
    }
    Ok((args, flags))
}

fn build(name: &str, args: &[&str], flags: &Flags<'_>) -> Result<Command, ParseError> {
    let first = args.first().copied();
    Ok(match name {
        "/help" => Command::Help,
        "/list" => Command::List(match first {
            Some(key) => Some(shelf(key).ok_or_else(|| ParseError::Shelf(key.to_string()))?),
            None => None,
        }),
        "/about" => Command::About(first.unwrap_or_default().to_string()),
        "/examples" => Command::Examples,
        "/run" => {
            let target = match first.unwrap_or_default() {
                "all" => Target::All,
                id => Target::System(id.to_string()),
            };
            Command::Run { target, example: example(args.get(1).copied().unwrap_or_default())? }
        }
        "/cave" => activities::cave(args, flags)?,
        "/trio" => activities::trio(args, flags)?,
        "/pool" => activities::pool(args, flags)?,
        "/ceremony" => activities::ceremony(args, flags)?,
        "/forge" => activities::forge(args)?,
        "/lang" => {
            let code = first.unwrap_or_default();
            Command::Lang(
                Language::from_code(code).ok_or_else(|| ParseError::Language(code.to_string()))?,
            )
        }
        "/clear" => Command::Clear,
        _ => Command::Quit,
    })
}

/// What is wrong with a line, in words.
pub(crate) fn error_text(error: ParseError, language: Language) -> String {
    let text = |msg: Msg, values: &[(&str, &str)]| fill(msg.text(language), values);
    match error {
        ParseError::Empty | ParseError::NoSlash => Msg::NoSlash.text(language).to_string(),
        ParseError::Unknown(command) => text(Msg::UnknownCommand, &[("command", &command)]),
        ParseError::Usage(usage) => text(Msg::Usage, &[("usage", usage)]),
        ParseError::Shelf(shelf) => {
            let keys: Vec<&str> = Shelf::ALL.iter().map(|s| s.key()).collect();
            text(Msg::UnknownShelf, &[("shelf", &shelf), ("shelves", &keys.join(", "))])
        }
        ParseError::Example(example) => text(Msg::UnknownExample, &[("example", &example)]),
        ParseError::Language(code) => {
            let codes: Vec<&str> = Language::ALL.iter().map(|l| l.code()).collect();
            text(Msg::UnknownLanguage, &[("language", &code), ("languages", &codes.join(", "))])
        }
        ParseError::Choice { value, choices } => {
            text(Msg::UnknownChoice, &[("value", &value), ("choices", &choices)])
        }
        ParseError::Count { flag, value, max } => {
            text(Msg::BadCount, &[("flag", flag), ("value", &value), ("max", &max.to_string())])
        }
        ParseError::Amount(value) => text(Msg::BadAmount, &[("value", &value)]),
    }
}

/// An example by its ID.
pub(crate) fn example(id: &str) -> Result<ExampleId, ParseError> {
    ExampleId::from_id(id).ok_or_else(|| ParseError::Example(id.to_string()))
}

/// A shelf by its command key.
pub(crate) fn shelf(key: &str) -> Option<Shelf> {
    Shelf::ALL.into_iter().find(|shelf| shelf.key() == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_command_shape() {
        assert_eq!(parse("/help"), Ok(Command::Help));
        assert_eq!(parse("  /list zcash "), Ok(Command::List(Some(Shelf::Zcash))));
        assert_eq!(parse("/list"), Ok(Command::List(None)));
        assert_eq!(
            parse("/run all one-plus-one"),
            Ok(Command::Run { target: Target::All, example: ExampleId::OnePlusOne })
        );
        assert_eq!(parse("/lang ko"), Ok(Command::Lang(Language::KOREAN)));
        assert_eq!(parse("examples"), Ok(Command::Examples));
    }

    #[test]
    fn explains_what_is_wrong() {
        assert_eq!(parse("/nope"), Err(ParseError::Unknown("/nope".to_string())));
        assert_eq!(parse("hello"), Err(ParseError::NoSlash));
        assert_eq!(parse("/run groth16"), Err(ParseError::Usage("/run <system|all> <example>")));
        assert_eq!(parse("/run groth16 two"), Err(ParseError::Example("two".to_string())));
        assert_eq!(parse("/list moon"), Err(ParseError::Shelf("moon".to_string())));
        assert_eq!(parse("/lang xx"), Err(ParseError::Language("xx".to_string())));
        assert_eq!(parse("/list --scenes 3"), Err(ParseError::Usage("/list [shelf]")));
    }
}
