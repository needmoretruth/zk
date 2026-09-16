//! The command line: global flags and the subcommands that print and exit.

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

    /// Print JSON lines (run reports, system metadata) instead of text.
    #[arg(long, global = true)]
    pub(crate) json: bool,

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
