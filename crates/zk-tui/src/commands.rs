//! The slash commands: their names, their arguments and how a typed line becomes one.
//!
//! The same table feeds `/help`, the completion popup and the parser, so a new command (the cave,
//! Trio, the pool) is one entry here plus one arm where commands are carried out.

use zk_core::ExampleId;
use zk_core::catalog::Shelf;
use zk_i18n::Language;

use crate::phrases::ui::Msg;

/// What an argument position accepts, which decides its completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arg {
    System,
    SystemOrAll,
    Example,
    Shelf,
    Language,
}

/// One command as the popup and `/help` show it.
#[derive(Debug)]
pub(crate) struct Spec {
    pub(crate) name: &'static str,
    pub(crate) usage: &'static str,
    /// Argument kinds in order, with whether each is required.
    pub(crate) args: &'static [(Arg, bool)],
    pub(crate) summary: Msg,
}

pub(crate) const SPECS: &[Spec] = &[
    Spec { name: "/help", usage: "/help", args: &[], summary: Msg::CmdHelp },
    Spec {
        name: "/list",
        usage: "/list [shelf]",
        args: &[(Arg::Shelf, false)],
        summary: Msg::CmdList,
    },
    Spec {
        name: "/about",
        usage: "/about <system>",
        args: &[(Arg::System, true)],
        summary: Msg::CmdAbout,
    },
    Spec { name: "/examples", usage: "/examples", args: &[], summary: Msg::CmdExamples },
    Spec {
        name: "/run",
        usage: "/run <system|all> <example>",
        args: &[(Arg::SystemOrAll, true), (Arg::Example, true)],
        summary: Msg::CmdRun,
    },
    Spec {
        name: "/lang",
        usage: "/lang <en|ko>",
        args: &[(Arg::Language, true)],
        summary: Msg::CmdLang,
    },
    Spec { name: "/clear", usage: "/clear", args: &[], summary: Msg::CmdClear },
    Spec { name: "/quit", usage: "/quit", args: &[], summary: Msg::CmdQuit },
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
}

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
    let args: Vec<&str> = words.collect();
    let required = spec.args.iter().filter(|(_, required)| *required).count();
    if args.len() < required || args.len() > spec.args.len() {
        return Err(ParseError::Usage(spec.usage));
    }
    build(spec.name, &args)
}

fn build(name: &str, args: &[&str]) -> Result<Command, ParseError> {
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
            let id = args.get(1).copied().unwrap_or_default();
            let example =
                ExampleId::from_id(id).ok_or_else(|| ParseError::Example(id.to_string()))?;
            Command::Run { target, example }
        }
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
    }
}
