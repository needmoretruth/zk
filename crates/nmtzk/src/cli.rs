//! The command line: global flags and the subcommands that print and exit.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use zk_core::ExampleId;
use zk_core::catalog::Shelf;
use zk_i18n::Language;

/// A working museum of zero-knowledge proof systems. Run without a command for the full-screen program.
#[derive(Debug, Parser)]
#[command(name = "nmtzk", version, long_about = None)]
pub(crate) struct Cli {
    /// Language of everything shown: en or ko.
    #[arg(long, global = true, value_name = "CODE", value_parser = parse_language, default_value = "en")]
    pub(crate) lang: Language,

    /// Draw with ASCII symbols only, for consoles without box-drawing glyphs.
    #[arg(long, global = true)]
    pub(crate) ascii: bool,

    /// Print JSON lines (run reports, system metadata, activity records) instead of text.
    #[arg(long, global = true)]
    pub(crate) json: bool,

    /// Where the Toy Shielded Pool keeps its ledgers and wallets. Default: $XDG_DATA_HOME/nmtzk or
    /// ~/.local/share/nmtzk, and ~/Library/Application Support/nmtzk on macOS.
    #[arg(long, global = true, value_name = "DIR")]
    pub(crate) data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// List the systems built into this binary, on every shelf or on one.
    List {
        /// zcash, aztec, polygon, others or homemade.
        #[arg(value_parser = parse_shelf)]
        shelf: Option<Shelf>,
    },
    /// What one system declares about itself, followed by its page.
    About {
        /// A system ID, as `list` shows it.
        system: String,
    },
    /// The seven statements every system proves.
    Examples,
    /// Prove, verify and attack an example with one system or with every system.
    Run {
        /// A system ID, or `all`.
        target: String,
        /// An example ID, as `examples` shows it.
        #[arg(value_parser = parse_example)]
        example: ExampleId,
        /// Seed for the example's fresh secrets, as 64 hex digits; drawn from the OS when omitted.
        #[arg(long, value_name = "HEX", value_parser = parse_seed)]
        seed: Option<[u8; 32]>,
        /// Skip the three attacks after the honest proof.
        #[arg(long)]
        no_attacks: bool,
    },
    /// Film Ali Baba's cave scene by scene: [MODE] [EXAMPLE].
    ///
    /// MODE is demonstration (the default), impostor, jealous-edit, court, prior-agreement or
    /// apartment; EXAMPLE is an example ID, one-plus-one by default.
    Cave {
        /// Mode, then example.
        #[arg(value_name = "WORDS")]
        words: Vec<String>,
        /// Scenes to film, 1 to 1000 (40 by default); floors in the apartment building.
        #[arg(long, value_name = "N")]
        scenes: Option<String>,
    },
    /// Play Trio's rounds: [play | cheat KIND | simulate] [EXAMPLE].
    ///
    /// KIND is bad-card, bad-computation or rewrite-hidden; EXAMPLE is one-plus-one by default.
    Trio {
        /// Mode (with the cheat's kind), then example.
        #[arg(value_name = "WORDS")]
        words: Vec<String>,
        /// Rounds the verifier plans, 1 to 1000 (55 by default).
        #[arg(long, value_name = "N")]
        rounds: Option<String>,
    },
    /// Use the Toy Shielded Pool: ACTION [ARGUMENTS].
    ///
    /// use <groth16|halo2> · wallet new <name> · wallet <name> · wallets · faucet <name> <amount> ·
    /// shield <name> <amount> · send <from> <to> <amount> · unshield <name> <amount> · ledger ·
    /// attack <double-spend|steal|counterfeit|unbound-nullifier> · reset [--yes]. Without an
    /// action, lists them.
    Pool {
        /// The action and its arguments.
        #[arg(value_name = "WORDS")]
        words: Vec<String>,
        /// Reset without asking first.
        #[arg(long)]
        yes: bool,
    },
    /// Run a trusted-setup ceremony: [toxic | tau | collude], tau by default.
    Ceremony {
        /// The part to run.
        #[arg(value_name = "PART")]
        words: Vec<String>,
        /// Participants taking turns, 1 to 100 (5 by default).
        #[arg(long, value_name = "N")]
        participants: Option<String>,
    },
    /// Replay a known forgery: bctv14 (CVE-2019-7167).
    Forge {
        /// What to forge.
        #[arg(value_name = "TARGET")]
        words: Vec<String>,
    },
}

impl Command {
    /// For an activity: its name and its words with flags written back the way the screen takes
    /// them, so both parse with one grammar. `None` for the other commands.
    pub(crate) fn activity(&self) -> Option<(&'static str, Vec<String>)> {
        let with = |words: &[String], flag: &str, value: &Option<String>| {
            let mut all = words.to_vec();
            if let Some(value) = value {
                all.extend([flag.to_string(), value.clone()]);
            }
            all
        };
        Some(match self {
            Command::Cave { words, scenes } => ("cave", with(words, "--scenes", scenes)),
            Command::Trio { words, rounds } => ("trio", with(words, "--rounds", rounds)),
            Command::Pool { words, yes } => {
                let mut all = words.clone();
                if *yes {
                    all.push("--yes".to_string());
                }
                ("pool", all)
            }
            Command::Ceremony { words, participants } => {
                ("ceremony", with(words, "--participants", participants))
            }
            Command::Forge { words } => ("forge", words.clone()),
            Command::List { .. }
            | Command::About { .. }
            | Command::Examples
            | Command::Run { .. } => {
                return None;
            }
        })
    }
}

fn parse_language(code: &str) -> Result<Language, String> {
    Language::from_code(code).ok_or_else(|| {
        let codes: Vec<&str> = Language::ALL.iter().map(|language| language.code()).collect();
        format!("expected one of: {}", codes.join(", "))
    })
}

fn parse_shelf(key: &str) -> Result<Shelf, String> {
    Shelf::ALL.into_iter().find(|shelf| shelf.key() == key).ok_or_else(|| {
        let keys: Vec<&str> = Shelf::ALL.iter().map(|shelf| shelf.key()).collect();
        format!("expected one of: {}", keys.join(", "))
    })
}

fn parse_example(id: &str) -> Result<ExampleId, String> {
    ExampleId::from_id(id).ok_or_else(|| {
        let ids: Vec<&str> = ExampleId::ALL.iter().map(|example| example.id()).collect();
        format!("expected one of: {}", ids.join(", "))
    })
}

/// 64 hex digits into 32 bytes.
pub(crate) fn parse_seed(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    if hex.len() != 64 || !hex.is_ascii() {
        return Err("expected 64 hex digits".to_string());
    }
    let mut seed = [0u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| "expected 64 hex digits".to_string())?;
    }
    Ok(seed)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn the_command_line_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn seeds_are_64_hex_digits() {
        let seed = parse_seed(&"ab".repeat(32)).expect("valid seed");
        assert_eq!(seed, [0xab; 32]);
        assert!(parse_seed("abc").is_err());
        assert!(parse_seed(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn flags_and_subcommands_parse() {
        let cli = Cli::try_parse_from([
            "nmtzk",
            "--lang",
            "ko",
            "run",
            "all",
            "one-plus-one",
            "--no-attacks",
        ])
        .expect("valid command line");
        assert_eq!(cli.lang, Language::KOREAN);
        assert!(matches!(
            cli.command,
            Some(Command::Run { ref target, example: ExampleId::OnePlusOne, seed: None, no_attacks: true })
                if target == "all"
        ));
        assert!(Cli::try_parse_from(["nmtzk", "run", "all", "two-plus-two"]).is_err());
        assert!(Cli::try_parse_from(["nmtzk", "list", "moon"]).is_err());
    }
}
