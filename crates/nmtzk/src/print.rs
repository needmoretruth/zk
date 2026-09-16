//! Subcommands that print and exit.

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use serde_json::json;
use zk_core::{Control, ExampleId, ProofSystem, RunError, RunOptions, RunReport};
use zk_i18n::Language;
use zk_tui::Museum;
use zk_tui::doc::Entry;
use zk_tui::plain::Printer;
use zk_tui::views::{self, Comparison};

use crate::cli::Command;

/// Exit code for a command line that names something this binary does not have.
const USAGE: u8 = 2;

/// What every subcommand needs.
pub(crate) struct Context {
    pub(crate) museum: Museum,
    pub(crate) language: Language,
    pub(crate) json: bool,
    pub(crate) printer: Printer,
}

impl Context {
    fn show(&self, entry: &Entry) {
        write_out(&self.printer.entry(entry));
    }

    fn fail(&self, entry: &Entry) -> ExitCode {
        eprint!("{}", self.printer.entry(entry));
        ExitCode::from(USAGE)
    }

    fn json_line(&self, value: &impl serde::Serialize) {
        match serde_json::to_string(value) {
            Ok(line) => write_out(&format!("{line}\n")),
            Err(error) => eprintln!("nmtzk: {error}"),
        }
    }
}

/// Writes to standard output. A reader that stops reading (`nmtzk list | head`) ends the program
/// quietly instead of with a panic; any other write failure is reported.
pub(crate) fn write_out(text: &str) {
    let mut out = std::io::stdout().lock();
    if let Err(error) = out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("nmtzk: {error}");
        std::process::exit(1);
    }
}

/// Runs one subcommand and returns the process exit code.
pub(crate) fn command(command: Command, context: &Context) -> ExitCode {
    let language = context.language;
    match command {
        Command::List { shelf } => {
            if context.json {
                let metas = context.museum.systems.iter().map(|system| system.meta());
                metas
                    .filter(|meta| shelf.is_none_or(|wanted| meta.shelf == wanted))
                    .for_each(|meta| context.json_line(meta));
            } else {
                context.show(&views::list(&context.museum, shelf, language));
            }
            ExitCode::SUCCESS
        }
        Command::About { system } => about(&system, context),
        Command::Examples => {
            if context.json {
                for example in ExampleId::ALL {
                    context.json_line(&json!({
                        "id": example.id(),
                        "public_inputs": example.public_input_names(),
                        "private_inputs": example.private_input_names(),
                    }));
                }
            } else {
                context.show(&views::examples(language));
            }
            ExitCode::SUCCESS
        }
        Command::Run { target, example, seed, no_attacks } => {
            let options = RunOptions { seed, attacks: !no_attacks };
            run(&target, example, &options, context)
        }
    }
}

fn about(id: &str, context: &Context) -> ExitCode {
    let (entry, page) = views::about(&context.museum, id, context.language);
    if context.json {
        return match context.museum.system(id) {
            Some(system) => {
                context.json_line(system.meta());
                ExitCode::SUCCESS
            }
            None => context.fail(&views::unknown_system(id, context.language)),
        };
    }
    if context.museum.system(id).is_none() && page.is_none() {
        return context.fail(&entry);
    }
    context.show(&entry);
    if let Some(page) = page {
        write_out("\n");
        write_out(&context.printer.page(page.markdown));
    }
    ExitCode::SUCCESS
}

fn run(target: &str, example: ExampleId, options: &RunOptions, context: &Context) -> ExitCode {
    let language = context.language;
    let systems: Vec<&'static dyn ProofSystem> = match target {
        "all" => context.museum.systems.to_vec(),
        id => match context.museum.system(id) {
            Some(system) => vec![system],
            None => return context.fail(&views::unknown_system(id, language)),
        },
    };
    if systems.is_empty() {
        context.show(&views::no_systems(language));
        return ExitCode::SUCCESS;
    }
    let mut table = Comparison::new(&systems, example);
    let mut failed = false;
    for (index, system) in systems.iter().enumerate() {
        let control = progress(system.meta().name, language);
        let result = system.run(example, options, &control);
        clear_progress();
        failed |= run_failed(&result);
        if context.json {
            context.json_line(&result_json(system.meta().id, example, &result));
        } else if target != "all" {
            context.show(&views::report(*system, example, &result, language));
        }
        table.finish(index, result);
    }
    if target == "all" && !context.json {
        table.close();
        context.show(&table.entry(language));
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

/// Whether a run means the command should exit non-zero: an honest proof rejected, an attack
/// accepted, or the run failing outright. A system that declares an example unsupported did not fail.
pub(crate) fn run_failed(result: &Result<RunReport, RunError>) -> bool {
    match result {
        Ok(report) => !report.sound(),
        Err(RunError::Unsupported { .. }) => false,
        Err(_) => true,
    }
}

fn result_json(
    system: &str,
    example: ExampleId,
    result: &Result<RunReport, RunError>,
) -> serde_json::Value {
    match result {
        Ok(report) => json!(report),
        Err(error) => {
            let (kind, detail) = match error {
                RunError::Unsupported { reason } => ("unsupported", (*reason).to_string()),
                RunError::Cancelled { after } => ("cancelled", format!("{after:?}")),
                RunError::Failed { stage, error } => ("failed", format!("{stage:?}: {error}")),
                RunError::Randomness(why) => ("randomness", why.clone()),
            };
            json!({ "system": system, "example": example.id(), "error": { "kind": kind, "detail": detail } })
        }
    }
}

/// A control that shows the stage on standard error while it is a terminal.
fn progress(system: &'static str, language: Language) -> Control {
    if !std::io::stderr().is_terminal() {
        return Control::new();
    }
    Control::with_progress(move |stage| {
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K{}", views::status(Some(stage), system, language));
        let _ = err.flush();
    })
}

fn clear_progress() {
    if std::io::stderr().is_terminal() {
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K");
        let _ = err.flush();
    }
}

#[cfg(test)]
mod tests {
    use zk_core::{AttackKind, AttackOutcome, AttackReport};
    use zk_core::{
        CircuitShape, RunMode, SecretScan, ShapeForm, Stage, SystemError, Timings, Verdict,
    };

    use super::*;

    fn report(verdict: Verdict, attack: AttackOutcome) -> RunReport {
        RunReport {
            system: "stand-in".into(),
            example: "one-plus-one".into(),
            field: "toy".into(),
            mode: RunMode::NonInteractive,
            shape: CircuitShape { form: ShapeForm::R1cs, counts: vec![] },
            timings: Timings::default(),
            setup_bytes: None,
            proof_bytes: 0,
            proof_head: vec![],
            rounds: None,
            verdict,
            attacks: vec![AttackReport {
                kind: AttackKind::BumpPublicInput,
                outcome: attack,
                offset: None,
            }],
            secret_scan: SecretScan::NotFound,
        }
    }

    #[test]
    fn exit_code_follows_rejected_proofs_accepted_attacks_and_failures() {
        assert!(!run_failed(&Ok(report(Verdict::Accepted, AttackOutcome::Rejected))));
        assert!(run_failed(&Ok(report(Verdict::Rejected, AttackOutcome::Rejected))));
        assert!(run_failed(&Ok(report(Verdict::Accepted, AttackOutcome::Accepted))));
        assert!(!run_failed(&Err(RunError::Unsupported { reason: "no" })));
        assert!(run_failed(&Err(RunError::Failed {
            stage: Stage::Prove,
            error: SystemError::Failed("x".into())
        })));
    }
}
