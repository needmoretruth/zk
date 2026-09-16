//! One run in full: verdict, circuit, timings, sizes, proof bytes, attacks and the secret scan.

use zk_core::catalog::{SystemMeta, ZeroKnowledge};
use zk_core::{
    AttackKind, AttackOutcome, AttackReport, ExampleId, ProofSystem, RunError, RunMode, RunReport,
    SecretScan, ShapeForm, Support, Verdict,
};
use zk_i18n::Language;

use crate::doc::{Doc, Entry, Hue, Kind, Span, Tone, plain, span};
use crate::format;
use crate::phrases::run::Msg as R;
use crate::phrases::{fill, humanise};

/// The cell shown while a single run is going.
pub fn running(meta: &SystemMeta, example: ExampleId, language: Language) -> Entry {
    let text = fill(R::Running.text(language), &[("system", meta.name), ("example", example.id())]);
    Entry::text(Kind::Running, text)
}

/// The finished cell for one run, or the warning or error that ended it.
pub fn report(
    system: &dyn ProofSystem,
    example: ExampleId,
    result: &Result<RunReport, RunError>,
    language: Language,
) -> Entry {
    let meta = system.meta();
    match result {
        Ok(report) => {
            Entry::new(Kind::Result, report_doc(meta, &system.support(example), report, language))
        }
        Err(error) => run_error(meta, example, error, language),
    }
}

/// A run that produced no report, said as a warning (unsupported, stopped) or an error (failed).
pub(crate) fn run_error(
    meta: &SystemMeta,
    example: ExampleId,
    error: &RunError,
    language: Language,
) -> Entry {
    let names = [("system", meta.name), ("example", example.id())];
    match error {
        RunError::Unsupported { reason } => {
            let text = fill(
                R::Unsupported.text(language),
                &[names[0], names[1], ("reason", &humanise(reason))],
            );
            Entry::text(Kind::Warning, text)
        }
        RunError::Cancelled { after: Some(stage) } => {
            let stage = super::stage::stage_name(*stage, language);
            Entry::text(
                Kind::Warning,
                fill(R::StoppedAfter.text(language), &[names[0], ("stage", &stage)]),
            )
        }
        RunError::Cancelled { after: None } => {
            Entry::text(Kind::Warning, fill(R::StoppedBefore.text(language), &names))
        }
        RunError::Failed { stage, error } => {
            let stage = super::stage::stage_name(*stage, language);
            let error = super::stage::system_error(error, language);
            let text =
                fill(R::Failed.text(language), &[names[0], ("stage", &stage), ("error", &error)]);
            Entry::text(Kind::Error, text)
        }
        RunError::Randomness(why) => {
            Entry::text(Kind::Error, fill(R::NoRandomness.text(language), &[("why", why)]))
        }
    }
}

fn report_doc(meta: &SystemMeta, support: &Support, report: &RunReport, language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(verdict(meta, report, language));
    for line in super::cautions(meta, language) {
        doc.line(line);
    }
    if let Support::Adapted { note } = support {
        doc.line(super::caution(fill(R::Adapted.text(language), &[("note", &humanise(note))])));
    }
    circuit(&mut doc, report, language);
    timings(&mut doc, report, language);
    sizes(&mut doc, report, language);
    head(&mut doc, report, language);
    attacks(&mut doc, report, language);
    scan(&mut doc, meta, report, language);
    doc
}

fn heading(doc: &mut Doc, title: &str, note: Option<String>) {
    let mut line = vec![span(title, Tone::BODY.bold())];
    if let Some(note) = note {
        line.push(span(format!(" · {note}"), Tone::of(Hue::Secondary)));
    }
    doc.line(line);
}

fn verdict(meta: &SystemMeta, report: &RunReport, language: Language) -> Vec<Span> {
    let names = [("system", meta.name), ("example", report.example.as_str())];
    match &report.verdict {
        Verdict::Accepted => vec![
            span("✓ ", Tone::of(Hue::Success)),
            span(fill(R::Accepted.text(language), &names), Tone::BODY.bold()),
        ],
        Verdict::Rejected => vec![
            span("✗ ", Tone::of(Hue::Failure)),
            span(fill(R::Rejected.text(language), &names), Tone::of(Hue::Failure).bold()),
        ],
        Verdict::Malformed(why) => vec![
            span("✗ ", Tone::of(Hue::Failure)),
            span(
                fill(R::Malformed.text(language), &[names[0], names[1], ("why", why)]),
                Tone::of(Hue::Failure).bold(),
            ),
        ],
    }
}

fn circuit(doc: &mut Doc, report: &RunReport, language: Language) {
    heading(doc, R::Circuit.text(language), None);
    let form = match report.shape.form {
        ShapeForm::R1cs => R::FormR1cs,
        ShapeForm::Plonkish => R::FormPlonkish,
        ShapeForm::WideAir => R::FormWideAir,
        ShapeForm::Acir => R::FormAcir,
        ShapeForm::Program => R::FormProgram,
        ShapeForm::Native => R::FormNative,
    };
    let mut items = vec![vec![plain(fill(
        R::CircuitForm.text(language),
        &[("form", form.text(language)), ("field", &report.field)],
    ))]];
    let counts: Vec<String> = report
        .shape
        .counts
        .iter()
        .map(|(key, value)| format!("{} {}", count_name(key, language), format::count(*value)))
        .collect();
    if !counts.is_empty() {
        items.push(vec![plain(counts.join(" · "))]);
    }
    doc.tree(items);
}

/// A circuit count's name; keys no table knows are shown as words.
fn count_name(key: &str, language: Language) -> String {
    let msg = match key {
        "constraints" => R::CountConstraints,
        "variables" => R::CountVariables,
        "public-inputs" => R::CountPublicInputs,
        "private-inputs" => R::CountPrivateInputs,
        "rows" => R::CountRows,
        "columns" => R::CountColumns,
        "gates" => R::CountGates,
        "multiplications" => R::CountMultiplications,
        other => return humanise(other),
    };
    msg.text(language).to_string()
}

fn timings(doc: &mut Doc, report: &RunReport, language: Language) {
    heading(doc, R::Timings.text(language), Some(R::Measured.text(language).to_string()));
    let t = report.timings;
    let verify = match report.mode {
        RunMode::NonInteractive => format::duration(t.verify_micros),
        RunMode::Interactive => R::VerifyInConversation.text(language).to_string(),
    };
    let mut pairs = vec![
        (R::TimeSetup.text(language).to_string(), vec![plain(format::duration(t.setup_micros))]),
        (R::TimeProve.text(language).to_string(), vec![plain(format::duration(t.prove_micros))]),
        (R::TimeVerify.text(language).to_string(), vec![plain(verify)]),
    ];
    if let Some(rounds) = report.rounds {
        pairs.push((R::Rounds.text(language).to_string(), vec![plain(rounds.to_string())]));
    }
    doc.pairs(pairs);
}

fn sizes(doc: &mut Doc, report: &RunReport, language: Language) {
    heading(doc, R::Sizes.text(language), None);
    let setup = match report.setup_bytes {
        Some(bytes) => format::size(bytes),
        None => R::NoSetupMaterial.text(language).to_string(),
    };
    doc.pairs(vec![
        (R::SetupMaterial.text(language).to_string(), vec![plain(setup)]),
        (proof_word(report, language).to_string(), vec![plain(format::size(report.proof_bytes))]),
    ]);
}

fn proof_word(report: &RunReport, language: Language) -> &'static str {
    match report.mode {
        RunMode::NonInteractive => R::Proof.text(language),
        RunMode::Interactive => R::Transcript.text(language),
    }
}

fn head(doc: &mut Doc, report: &RunReport, language: Language) {
    if report.proof_head.is_empty() {
        return;
    }
    let count = fill(
        R::ProofHeadCount.text(language),
        &[
            ("shown", &report.proof_head.len().to_string()),
            ("total", &format::count(report.proof_bytes)),
        ],
    );
    heading(doc, R::ProofHead.text(language), Some(count));
    doc.tree(vec![vec![span(format::hex_groups(&report.proof_head), Tone::of(Hue::Secondary))]]);
}

fn attacks(doc: &mut Doc, report: &RunReport, language: Language) {
    if report.attacks.is_empty() {
        heading(doc, R::Attacks.text(language), Some(R::AttacksSkipped.text(language).to_string()));
        return;
    }
    let held = report.attacks.iter().filter(|attack| attack.outcome.held()).count();
    let total = report.attacks.len().to_string();
    let note =
        fill(R::AttacksHeld.text(language), &[("held", &held.to_string()), ("total", &total)]);
    heading(doc, R::Attacks.text(language), Some(note));
    doc.tree(report.attacks.iter().map(|attack| attack_line(attack, language)).collect());
}

fn attack_line(attack: &AttackReport, language: Language) -> Vec<Span> {
    let name = match (attack.kind, attack.offset) {
        (AttackKind::FlipProofByte, Some(offset)) => {
            fill(R::AttackFlipAt.text(language), &[("offset", &offset.to_string())])
        }
        (AttackKind::FlipProofByte, None) => R::AttackFlip.text(language).to_string(),
        (AttackKind::BumpPublicInput, _) => R::AttackBump.text(language).to_string(),
        (AttackKind::DishonestWitness, _) => R::AttackDishonest.text(language).to_string(),
    };
    let (glyph, hue, text) = match &attack.outcome {
        AttackOutcome::Rejected => {
            ("✓", Hue::Success, R::OutcomeRejected.text(language).to_string())
        }
        AttackOutcome::Malformed(why) => {
            ("✓", Hue::Success, fill(R::OutcomeMalformed.text(language), &[("why", why)]))
        }
        AttackOutcome::ProverRefused(why) => {
            ("✓", Hue::Success, fill(R::OutcomeRefused.text(language), &[("why", why)]))
        }
        AttackOutcome::Accepted if attack.kind == AttackKind::FlipProofByte => {
            ("⚠", Hue::Caution, R::OutcomeMalleable.text(language).to_string())
        }
        AttackOutcome::Accepted => {
            ("✗", Hue::Failure, R::OutcomeAccepted.text(language).to_string())
        }
        AttackOutcome::NotApplicable(why) => {
            ("–", Hue::Secondary, fill(R::OutcomeNotApplicable.text(language), &[("why", why)]))
        }
    };
    let text_tone = if hue == Hue::Success { Tone::BODY } else { Tone::of(hue) };
    vec![
        span(format!("{glyph} "), Tone::of(hue)),
        plain(format!("{name}: ")),
        span(text, text_tone),
    ]
}

fn scan(doc: &mut Doc, meta: &SystemMeta, report: &RunReport, language: Language) {
    heading(doc, R::Scan.text(language), None);
    let secondary = Tone::of(Hue::Secondary);
    let mut items = Vec::new();
    match &report.secret_scan {
        SecretScan::NotFound => {
            items.push(vec![
                span("✓ ", Tone::of(Hue::Success)),
                plain(R::ScanNotFound.text(language)),
            ]);
        }
        SecretScan::Found(names) => {
            let found = fill(R::ScanFound.text(language), &[("names", &names.join(", "))]);
            if meta.zero_knowledge == ZeroKnowledge::No {
                items.push(super::caution(found));
                items.push(vec![span(R::ScanFoundExpected.text(language), secondary)]);
            } else {
                let tone = Tone::of(Hue::Failure);
                items.push(vec![span("✗ ", tone), span(found, tone)]);
            }
        }
        SecretScan::Inconclusive => {
            items.push(vec![span("– ", secondary), plain(R::ScanInconclusive.text(language))]);
        }
    }
    items.push(vec![span(R::ScanSmoke.text(language), secondary)]);
    doc.tree(items);
}
