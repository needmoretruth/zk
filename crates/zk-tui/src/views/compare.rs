//! `/run all`: every system on one example, as a table that fills in row by row.

use zk_core::catalog::{SystemMeta, ZeroKnowledge};
use zk_core::{ExampleId, ProofSystem, RunError, RunReport, SecretScan, Verdict};
use zk_i18n::Language;

use crate::doc::{Align, Block, Doc, Entry, Hue, Kind, Span, Table, Tone, plain, span};
use crate::format;
use crate::phrases::fill;
use crate::phrases::run::Msg as R;

/// Where one row of the comparison is.
#[derive(Clone, Debug)]
enum RowState {
    Waiting,
    Running,
    Finished(Box<Result<RunReport, RunError>>),
    Skipped,
}

/// The comparison of every system on one example, built up as runs finish.
#[derive(Clone, Debug)]
pub struct Comparison {
    example: ExampleId,
    rows: Vec<(&'static SystemMeta, RowState)>,
    closed: bool,
}

impl Comparison {
    /// A table with a waiting row for each system.
    pub fn new(systems: &[&'static dyn ProofSystem], example: ExampleId) -> Comparison {
        let rows = systems.iter().map(|system| (system.meta(), RowState::Waiting)).collect();
        Comparison { example, rows, closed: false }
    }

    /// Marks row `index` as running.
    pub fn start(&mut self, index: usize) {
        if let Some(row) = self.rows.get_mut(index) {
            row.1 = RowState::Running;
        }
    }

    /// Records how row `index` ended.
    pub fn finish(&mut self, index: usize, result: Result<RunReport, RunError>) {
        if let Some(row) = self.rows.get_mut(index) {
            row.1 = RowState::Finished(Box::new(result));
        }
    }

    /// Marks every row that has not started as skipped and ends the table.
    pub fn close(&mut self) {
        for row in &mut self.rows {
            if matches!(row.1, RowState::Waiting | RowState::Running) {
                row.1 = RowState::Skipped;
            }
        }
        self.closed = true;
    }

    /// Every finished result, in row order.
    pub fn results(&self) -> impl Iterator<Item = &Result<RunReport, RunError>> {
        self.rows.iter().filter_map(|(_, state)| match state {
            RowState::Finished(result) => Some(result.as_ref()),
            _ => None,
        })
    }

    /// The table as a cell: running until closed, then a result.
    pub fn entry(&self, language: Language) -> Entry {
        let mut doc = Doc::new();
        doc.line(vec![span(
            fill(R::CompareTitle.text(language), &[("example", self.example.id())]),
            Tone::BODY.bold(),
        )]);
        doc.push(Block::Table(self.table(language)));
        if self.closed {
            self.footer(&mut doc, language);
        }
        Entry::new(if self.closed { Kind::Result } else { Kind::Running }, doc)
    }

    fn table(&self, language: Language) -> Table {
        let header = [
            R::ColSystem,
            R::ColShelf,
            R::ColSetup,
            R::ColProve,
            R::ColVerify,
            R::ColProof,
            R::ColVerdict,
            R::ColAttacks,
            R::ColSecrets,
        ]
        .iter()
        .map(|msg| vec![plain(msg.text(language))])
        .collect();
        let rows = self.rows.iter().map(|(meta, state)| row(meta, state, language)).collect();
        let mut align = vec![Align::Left; 9];
        for column in [2, 3, 4, 5, 7] {
            align[column] = Align::Right;
        }
        Table { header, align, rows, wrap: false, optional: vec![1] }
    }

    fn footer(&self, doc: &mut Doc, language: Language) {
        let secondary = Tone::of(Hue::Secondary);
        let finished: Vec<&RunReport> =
            self.results().filter_map(|result| result.as_ref().ok()).collect();
        let sound = finished.iter().filter(|report| report.sound()).count();
        let summary = fill(
            R::CompareSummary.text(language),
            &[("sound", &sound.to_string()), ("total", &self.rows.len().to_string())],
        );
        let tone = if sound == self.rows.len() { Tone::of(Hue::Success) } else { Tone::BODY };
        doc.line(vec![span(summary, tone)]);
        doc.line(vec![span(R::CompareTimes.text(language), secondary)]);
        doc.line(vec![span(R::CompareSecrets.text(language), secondary)]);
        let details = fill(R::CompareDetails.text(language), &[("example", self.example.id())]);
        doc.line(vec![span(details, secondary)]);
    }
}

fn row(meta: &SystemMeta, state: &RowState, language: Language) -> Vec<Vec<Span>> {
    let secondary = Tone::of(Hue::Secondary);
    let mut cells = vec![
        vec![plain(meta.name)],
        vec![span(super::shelf_name(meta.shelf, language), Tone::of(Hue::Shelf))],
    ];
    let dash = || vec![span("–", secondary)];
    let (verdict, report) = match state {
        RowState::Waiting => (vec![span(R::RowWaiting.text(language), secondary)], None),
        RowState::Running => {
            (vec![span(R::RowRunning.text(language), Tone::of(Hue::Accent))], None)
        }
        RowState::Skipped => (vec![span(R::RowSkipped.text(language), secondary)], None),
        RowState::Finished(result) => match result.as_ref() {
            Ok(report) => (verdict_cell(&report.verdict, language), Some(report)),
            Err(error) => (error_cell(error, language), None),
        },
    };
    match report {
        Some(report) => {
            let t = report.timings;
            cells.push(vec![plain(format::duration(t.setup_micros))]);
            cells.push(vec![plain(format::duration(t.prove_micros))]);
            cells.push(if t.verify_micros == 0 {
                dash()
            } else {
                vec![plain(format::duration(t.verify_micros))]
            });
            cells.push(vec![plain(format::size(report.proof_bytes))]);
            cells.push(verdict);
            cells.push(attacks_cell(report));
            cells.push(secrets_cell(meta, &report.secret_scan, language));
        }
        None => {
            cells.extend([dash(), dash(), dash(), dash(), verdict, dash(), dash()]);
        }
    }
    cells
}

fn verdict_cell(verdict: &Verdict, language: Language) -> Vec<Span> {
    match verdict {
        Verdict::Accepted => mark("✓", Hue::Success, R::RowAccepted.text(language)),
        Verdict::Rejected | Verdict::Malformed(_) => {
            mark("✗", Hue::Failure, R::RowRejected.text(language))
        }
    }
}

fn error_cell(error: &RunError, language: Language) -> Vec<Span> {
    match error {
        RunError::Unsupported { .. } => mark("–", Hue::Secondary, R::RowUnsupported.text(language)),
        RunError::Cancelled { .. } => mark("–", Hue::Caution, R::RowStopped.text(language)),
        RunError::Failed { .. } | RunError::Randomness(_) => {
            mark("✗", Hue::Failure, R::RowFailed.text(language))
        }
    }
}

fn attacks_cell(report: &RunReport) -> Vec<Span> {
    if report.attacks.is_empty() {
        return vec![span("–", Tone::of(Hue::Secondary))];
    }
    let held = report.attacks.iter().filter(|attack| attack.outcome.held()).count();
    let hue = if held == report.attacks.len() { Hue::Success } else { Hue::Failure };
    vec![span(format!("{held}/{}", report.attacks.len()), Tone::of(hue))]
}

fn secrets_cell(meta: &SystemMeta, scan: &SecretScan, language: Language) -> Vec<Span> {
    match scan {
        SecretScan::NotFound => mark("✓", Hue::Success, R::SecretsNone.text(language)),
        SecretScan::Found(_) if meta.zero_knowledge == ZeroKnowledge::No => {
            mark("⚠", Hue::Caution, R::SecretsFound.text(language))
        }
        SecretScan::Found(_) => mark("✗", Hue::Failure, R::SecretsFound.text(language)),
        SecretScan::Inconclusive => mark("–", Hue::Secondary, R::SecretsUnknown.text(language)),
    }
}

fn mark(glyph: &str, hue: Hue, word: &str) -> Vec<Span> {
    vec![span(format!("{glyph} "), Tone::of(hue)), plain(word)]
}
