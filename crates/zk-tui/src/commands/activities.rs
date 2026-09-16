//! The arguments of the hands-on commands: the cave, Trio, the pool, the ceremony and the forgery.

use zk_core::ExampleId;

use super::{Command, Flags, ParseError, example};
use crate::activities::cave::{CaveMode, DEFAULT_SCENES, MAX_SCENES};
use crate::activities::ceremony::{CeremonyPart, DEFAULT_PARTICIPANTS, MAX_PARTICIPANTS};
use crate::activities::forge::ForgeTarget;
use crate::activities::pool::{PoolAction, PoolSystem, attack_from_key, attack_key, parse_amount};
use crate::activities::trio::{DEFAULT_ROUNDS, MAX_ROUNDS, TrioCheat, TrioMode};

const TRIO_CHEAT_USAGE: &str = "/trio cheat <bad-card|bad-computation|rewrite-hidden> [example]";
const POOL_ACTIONS: [&str; 10] = [
    "use", "wallet", "wallets", "faucet", "shield", "send", "unshield", "ledger", "attack", "reset",
];

/// The value of a numeric flag, `default` when it is absent.
fn count(flags: &Flags<'_>, flag: &'static str, default: u32, max: u32) -> Result<u32, ParseError> {
    let Some((_, value)) = flags.iter().rev().find(|(name, _)| *name == flag) else {
        return Ok(default);
    };
    let value = value.unwrap_or_default();
    match value.parse::<u32>() {
        Ok(number) if (1..=max).contains(&number) => Ok(number),
        _ => Err(ParseError::Count { flag, value: value.to_string(), max }),
    }
}

fn choice<T>(value: &str, found: Option<T>, keys: &[&str]) -> Result<T, ParseError> {
    found.ok_or_else(|| ParseError::Choice { value: value.to_string(), choices: keys.join(", ") })
}

fn example_or_default(words: &[&str], usage: &'static str) -> Result<ExampleId, ParseError> {
    match words {
        [] => Ok(ExampleId::OnePlusOne),
        [id] => example(id),
        _ => Err(ParseError::Usage(usage)),
    }
}

/// `/cave [mode] [example] [--scenes N]`; a lone example means the demonstration.
pub(super) fn cave(args: &[&str], flags: &Flags<'_>) -> Result<Command, ParseError> {
    let scenes = count(flags, "--scenes", DEFAULT_SCENES, MAX_SCENES)?;
    let usage = "/cave [mode] [example] [--scenes N]";
    let (mode, rest) = match args.split_first() {
        None => (CaveMode::Demonstration, args),
        Some((word, rest)) => match CaveMode::from_key(word) {
            Some(mode) => (mode, rest),
            None if rest.is_empty() && ExampleId::from_id(word).is_some() => {
                (CaveMode::Demonstration, args)
            }
            None => {
                let keys: Vec<&str> = CaveMode::ALL.iter().map(|mode| mode.key()).collect();
                return Err(ParseError::Choice {
                    value: word.to_string(),
                    choices: keys.join(", "),
                });
            }
        },
    };
    Ok(Command::Cave { mode, example: example_or_default(rest, usage)?, scenes })
}

/// `/trio [play|cheat <kind>|simulate] [example] [--rounds N]`; a lone example means play.
pub(super) fn trio(args: &[&str], flags: &Flags<'_>) -> Result<Command, ParseError> {
    let rounds = count(flags, "--rounds", DEFAULT_ROUNDS, MAX_ROUNDS)?;
    let usage = "/trio [mode] [example] [--rounds N]";
    let (mode, rest) = match args {
        [] => (TrioMode::Play, args),
        ["play", rest @ ..] => (TrioMode::Play, rest),
        ["simulate", rest @ ..] => (TrioMode::Simulate, rest),
        ["cheat"] => return Err(ParseError::Usage(TRIO_CHEAT_USAGE)),
        ["cheat", kind, rest @ ..] => {
            let keys: Vec<&str> = TrioCheat::ALL.iter().map(|cheat| cheat.key()).collect();
            (TrioMode::Cheat(choice(kind, TrioCheat::from_key(kind), &keys)?), rest)
        }
        [word] if ExampleId::from_id(word).is_some() => (TrioMode::Play, args),
        [word, ..] => {
            return Err(ParseError::Choice {
                value: word.to_string(),
                choices: "play, cheat, simulate".to_string(),
            });
        }
    };
    Ok(Command::Trio { mode, example: example_or_default(rest, usage)?, rounds })
}

fn amount(text: &str) -> Result<u16, ParseError> {
    parse_amount(text).ok_or_else(|| ParseError::Amount(text.to_string()))
}

/// `/pool <action> …`; `/pool` alone lists the actions.
pub(super) fn pool(args: &[&str], flags: &Flags<'_>) -> Result<Command, ParseError> {
    let name = |word: &str| word.to_string();
    let action = match args {
        [] => PoolAction::Help,
        ["use", system] => {
            let keys: Vec<&str> = PoolSystem::ALL.iter().map(|s| s.key()).collect();
            PoolAction::Use(choice(system, PoolSystem::from_key(system), &keys)?)
        }
        ["wallet", "new", wallet] => PoolAction::NewWallet(name(wallet)),
        ["wallet", wallet] if *wallet != "new" => PoolAction::Wallet(name(wallet)),
        ["wallets"] => PoolAction::Wallets,
        ["faucet", wallet, coins] => {
            PoolAction::Faucet { name: name(wallet), amount: amount(coins)? }
        }
        ["shield", wallet, coins] => {
            PoolAction::Shield { name: name(wallet), amount: amount(coins)? }
        }
        ["unshield", wallet, coins] => {
            PoolAction::Unshield { name: name(wallet), amount: amount(coins)? }
        }
        ["send", from, to, coins] => {
            PoolAction::Send { from: name(from), to: name(to), amount: amount(coins)? }
        }
        ["ledger"] => PoolAction::Ledger,
        ["attack", attack] => {
            let keys: Vec<&str> = zk_pool::Attack::ALL.iter().map(|a| attack_key(*a)).collect();
            PoolAction::Attack(choice(attack, attack_from_key(attack), &keys)?)
        }
        ["reset"] => {
            PoolAction::Reset { confirmed: flags.iter().any(|(flag, _)| *flag == "--yes") }
        }
        [action, ..] => return Err(pool_usage(action)),
    };
    Ok(Command::Pool(action))
}

/// The usage of one pool action, or the list of actions for a word that is none of them.
fn pool_usage(action: &str) -> ParseError {
    let usage = match action {
        "use" => "/pool use <groth16|halo2>",
        "wallet" => "/pool wallet new <name> · /pool wallet <name>",
        "wallets" => "/pool wallets",
        "faucet" => "/pool faucet <name> <amount>",
        "shield" => "/pool shield <name> <amount>",
        "unshield" => "/pool unshield <name> <amount>",
        "send" => "/pool send <from> <to> <amount>",
        "ledger" => "/pool ledger",
        "attack" => "/pool attack <double-spend|steal|counterfeit|unbound-nullifier>",
        "reset" => "/pool reset --yes",
        other => {
            let choices = POOL_ACTIONS.join(", ");
            return ParseError::Choice { value: other.to_string(), choices };
        }
    };
    ParseError::Usage(usage)
}

/// `/ceremony [toxic|tau|collude] [--participants N]`.
pub(super) fn ceremony(args: &[&str], flags: &Flags<'_>) -> Result<Command, ParseError> {
    let default = u32::try_from(DEFAULT_PARTICIPANTS).unwrap_or(MAX_PARTICIPANTS);
    let participants = count(flags, "--participants", default, MAX_PARTICIPANTS)? as usize;
    let part = match args.first() {
        None => CeremonyPart::Tau,
        Some(word) => {
            let keys: Vec<&str> = CeremonyPart::ALL.iter().map(|part| part.key()).collect();
            choice(word, CeremonyPart::from_key(word), &keys)?
        }
    };
    Ok(Command::Ceremony { part, participants })
}

/// `/forge bctv14`.
pub(super) fn forge(args: &[&str]) -> Result<Command, ParseError> {
    let word = args.first().copied().unwrap_or_default();
    let keys: Vec<&str> = ForgeTarget::ALL.iter().map(|target| target.key()).collect();
    Ok(Command::Forge(choice(word, ForgeTarget::from_key(word), &keys)?))
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::*;

    #[test]
    fn the_cave_takes_a_mode_an_example_and_a_scene_count() {
        let cave = |mode, example, scenes| Ok(Command::Cave { mode, example, scenes });
        assert_eq!(parse("/cave"), cave(CaveMode::Demonstration, ExampleId::OnePlusOne, 40));
        assert_eq!(
            parse("/cave jealous-edit --scenes 8"),
            cave(CaveMode::JealousEdit, ExampleId::OnePlusOne, 8)
        );
        assert_eq!(
            parse("/cave --scenes 3 court sudoku"),
            cave(CaveMode::Court, ExampleId::Sudoku, 3)
        );
        assert_eq!(parse("/cave age"), cave(CaveMode::Demonstration, ExampleId::Age, 40));
        assert!(matches!(parse("/cave ring"), Err(ParseError::Choice { .. })));
        assert!(matches!(parse("/cave --scenes 0"), Err(ParseError::Count { max: 1000, .. })));
        assert_eq!(
            parse("/cave --scenes"),
            Err(ParseError::Usage("/cave [mode] [example] [--scenes N]"))
        );
    }

    #[test]
    fn trio_takes_a_mode_a_cheat_and_rounds() {
        assert_eq!(
            parse("/trio cheat bad-card sudoku --rounds 6"),
            Ok(Command::Trio {
                mode: TrioMode::Cheat(TrioCheat::BadCard),
                example: ExampleId::Sudoku,
                rounds: 6
            })
        );
        assert_eq!(
            parse("/trio"),
            Ok(Command::Trio { mode: TrioMode::Play, example: ExampleId::OnePlusOne, rounds: 55 })
        );
        assert_eq!(parse("/trio cheat"), Err(ParseError::Usage(TRIO_CHEAT_USAGE)));
        assert!(matches!(parse("/trio cheat lie"), Err(ParseError::Choice { .. })));
    }

    #[test]
    fn pool_actions_parse_with_their_arguments() {
        let pool = |action| Ok(Command::Pool(action));
        assert_eq!(parse("/pool"), pool(PoolAction::Help));
        assert_eq!(parse("/pool wallet new alice"), pool(PoolAction::NewWallet("alice".into())));
        assert_eq!(parse("/pool wallet alice"), pool(PoolAction::Wallet("alice".into())));
        assert_eq!(
            parse("/pool send alice bob 120"),
            pool(PoolAction::Send { from: "alice".into(), to: "bob".into(), amount: 120 })
        );
        assert_eq!(parse("/pool use halo2"), pool(PoolAction::Use(PoolSystem::Halo2)));
        assert_eq!(parse("/pool reset"), pool(PoolAction::Reset { confirmed: false }));
        assert_eq!(parse("/pool reset --yes"), pool(PoolAction::Reset { confirmed: true }));
        assert_eq!(parse("/pool shield alice lots"), Err(ParseError::Amount("lots".into())));
        assert_eq!(
            parse("/pool send alice 5"),
            Err(ParseError::Usage("/pool send <from> <to> <amount>"))
        );
        assert!(matches!(parse("/pool mint"), Err(ParseError::Choice { .. })));
    }

    #[test]
    fn the_ceremony_and_the_forgery_parse() {
        assert_eq!(
            parse("/ceremony"),
            Ok(Command::Ceremony { part: CeremonyPart::Tau, participants: 5 })
        );
        assert_eq!(
            parse("/ceremony collude --participants 3"),
            Ok(Command::Ceremony { part: CeremonyPart::Collude, participants: 3 })
        );
        assert_eq!(parse("/forge bctv14"), Ok(Command::Forge(ForgeTarget::Bctv14)));
        assert!(matches!(parse("/forge groth16"), Err(ParseError::Choice { .. })));
        assert_eq!(parse("/forge"), Err(ParseError::Usage("/forge bctv14")));
    }
}
