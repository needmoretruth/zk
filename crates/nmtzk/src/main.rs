//! `nmtzk`: the museum's program.
//!
//! Without a command on a terminal it opens the full-screen program. With a command it prints the
//! same cells the full-screen program would show, or JSON lines with `--json`, and exits: non-zero
//! when an honest proof was rejected, an attack was accepted or a run failed.
//!
//! What upstream proof-system crates print on their own is sent to `/dev/null` (see `console`);
//! `NMTZK_UPSTREAM_OUTPUT=1` shows it.

mod activity;
mod cli;
mod console;
mod print;

use std::io::IsTerminal;
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

use clap::Parser;
use zk_tui::plain::Printer;
use zk_tui::{Look, Museum, Settings};

use crate::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    console::start();
    // With descriptor 2 silenced the panic hook only keeps the message, so a panic that reaches
    // here is reported on the program's own standard error, with Rust's usual exit code.
    panic::catch_unwind(AssertUnwindSafe(|| run(cli))).unwrap_or_else(|_| {
        console::report_panic();
        ExitCode::from(console::PANICKED)
    })
}

fn run(cli: Cli) -> ExitCode {
    let look = Look::detect(cli.ascii);
    let museum = Museum { systems: zk_registry::systems(), page: zk_registry::page };
    let context = print::Context {
        museum,
        language: cli.lang,
        json: cli.json,
        printer: Printer::stdout(look, console::out().is_terminal()),
        data_dir: cli.data_dir.clone(),
    };
    match cli.command {
        Some(command) => print::command(command, &context),
        None if std::io::stdin().is_terminal() && console::out().is_terminal() => {
            let settings = Settings {
                language: cli.lang,
                look,
                data_dir: cli.data_dir,
                pace: zk_tui::activities::Pace::Story,
            };
            match console::screen().and_then(|screen| zk_tui::run(museum, settings, screen)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    console::write_err(&format!("nmtzk: {error}\n"));
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
