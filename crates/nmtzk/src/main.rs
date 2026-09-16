//! `nmtzk`: the museum's program.
//!
//! Without a command on a terminal it opens the full-screen program. With a command it prints the
//! same cells the full-screen program would show, or JSON lines with `--json`, and exits: non-zero
//! when an honest proof was rejected, an attack was accepted or a run failed.

mod activity;
mod cli;
mod print;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use zk_tui::plain::Printer;
use zk_tui::{Look, Museum, Settings};

use crate::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let look = Look::detect(cli.ascii);
    let museum = Museum { systems: zk_registry::systems(), page: zk_registry::page };
    let context = print::Context {
        museum,
        language: cli.lang,
        json: cli.json,
        printer: Printer::stdout(look),
        data_dir: cli.data_dir.clone(),
    };
    match cli.command {
        Some(command) => print::command(command, &context),
        None if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() => {
            let settings = Settings {
                language: cli.lang,
                look,
                data_dir: cli.data_dir,
                pace: zk_tui::activities::Pace::Story,
            };
            match zk_tui::run(museum, settings) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("nmtzk: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            print::write_out(&context.printer.entry(&zk_tui::views::welcome(&museum, cli.lang)));
            ExitCode::SUCCESS
        }
    }
}
