//! An attack on the pool, step by step, in scratch pools.

use zk_core::Verdict;
use zk_i18n::Language;
use zk_pool::{Attack, AttackAction, AttackReport, AttackStep, SpendCircuit};

use super::{coins, disclaimer, heading, reason};
use crate::activities::pool::PoolSystem;
use crate::doc::{Doc, Entry, Hue, Kind, Span, Tone, plain, span};
use crate::phrases::fill;
use crate::phrases::pool::Msg as M;

/// An attack's display name.
pub(crate) fn name(attack: Attack) -> M {
    match attack {
        Attack::DoubleSpend => M::AttackDoubleSpend,
        Attack::Steal => M::AttackSteal,
        Attack::Counterfeit => M::AttackCounterfeit,
        Attack::UnboundNullifier => M::AttackUnboundNullifier,
    }
}

fn intro(attack: Attack) -> M {
    match attack {
        Attack::DoubleSpend => M::IntroDoubleSpend,
        Attack::Steal => M::IntroSteal,
        Attack::Counterfeit => M::IntroCounterfeit,
        Attack::UnboundNullifier => M::IntroUnboundNullifier,
    }
}

fn mark(good: bool) -> Span {
    if good { span("✓ ", Tone::of(Hue::Success)) } else { span("✗ ", Tone::of(Hue::Failure)) }
}

/// Every step of an attack, and whether the honest ledger held.
pub(crate) fn attack(report: &AttackReport, system: PoolSystem, language: Language) -> Entry {
    let title = fill(
        M::AttackTitle.text(language),
        &[("attack", name(report.attack).text(language)), ("system", system.name())],
    );
    let mut doc = Doc::new();
    heading(&mut doc, title);
    doc.line(vec![plain(intro(report.attack).text(language))]);
    doc.line(vec![span(M::AttackScratch.text(language), Tone::of(Hue::Secondary))]);
    for (index, step) in report.steps.iter().enumerate() {
        doc.append(step_doc(index + 1, step, language));
    }
    let (held, msg) =
        if report.honest_ledger_held { (true, M::AttackHeld) } else { (false, M::AttackBroken) };
    let tone = if held { Tone::BODY.bold() } else { Tone::of(Hue::Failure).bold() };
    doc.led(vec![mark(held)], vec![span(msg.text(language), tone)]);
    disclaimer(&mut doc, language);
    Entry::new(if held { Kind::Result } else { Kind::Error }, doc)
}

fn action_text(step: &AttackStep, language: Language) -> String {
    let v_pub_in = step.receipt.public.shielded.as_ref().map_or(0, |s| s.v_pub_in);
    match step.action {
        AttackAction::AttackerShields => {
            fill(M::StepAttackerShields.text(language), &[("amount", &coins(v_pub_in))])
        }
        AttackAction::VictimShields => {
            fill(M::StepVictimShields.text(language), &[("amount", &coins(v_pub_in))])
        }
        AttackAction::SpendNote => M::StepSpendNote.text(language).to_string(),
        AttackAction::RespendSpentNote => M::StepRespend.text(language).to_string(),
        AttackAction::SpendNoteWithoutKey => M::StepSteal.text(language).to_string(),
        AttackAction::SpendWithNegativeOutput { outputs: [first, second] } => fill(
            M::StepNegative.text(language),
            &[("first", &first.to_string()), ("second", &second.to_string())],
        ),
        AttackAction::UnshieldEverything { amount } => {
            fill(M::StepUnshieldAll.text(language), &[("amount", &coins(amount))])
        }
        AttackAction::RespendWithFreshNullifier => M::StepFreshNullifier.text(language).to_string(),
    }
}

fn circuit_name(circuit: SpendCircuit, language: Language) -> &'static str {
    match circuit {
        SpendCircuit::Honest => M::CircuitHonest,
        SpendCircuit::WithoutRangeChecks => M::CircuitNoRange,
        SpendCircuit::WithoutNullifierBinding => M::CircuitUnbound,
    }
    .text(language)
}

/// What the ledger did with a step: recorded, or refused with every reason.
fn outcome_text(step: &AttackStep, language: Language) -> String {
    match &step.receipt.decision {
        Some(decision) if !decision.accepted => {
            let (before, after) = (decision.pool_balance_before, decision.pool_balance_after);
            let reasons: Vec<String> =
                decision.rejections.iter().map(|r| reason(*r, before, after, language)).collect();
            fill(M::StepRefused.text(language), &[("reasons", &reasons.join("; "))])
        }
        _ => M::StepRecorded.text(language).to_string(),
    }
}

/// One step: what was tried, against which circuit, what the verifier and the ledger said, which
/// assertions the proved values break, and the scratch pool afterwards.
fn step_doc(number: usize, step: &AttackStep, language: Language) -> Doc {
    let secondary = Tone::of(Hue::Secondary);
    let mut doc = Doc::new();
    doc.led(
        vec![plain(format!("{number}. "))],
        vec![
            span(action_text(step, language), Tone::BODY.bold()),
            span(format!(" · {}", circuit_name(step.circuit, language)), secondary),
        ],
    );
    let indent = || vec![plain(" ".repeat(number.to_string().len() + 2))];
    let recorded = step.receipt.accepted();
    // A forgery that gets recorded is a break; a scene-setting step that gets refused is a fault.
    let good = recorded != step.action.is_forgery();
    let tone = if good { Tone::BODY } else { Tone::of(Hue::Failure) };
    let mut line = indent();
    line.push(mark(good));
    doc.led(line, vec![span(outcome_text(step, language), tone)]);
    let mut details = Vec::new();
    if let Some(decision) = &step.receipt.decision {
        let verdict = match &decision.proof {
            Verdict::Accepted => M::WordAccepted,
            Verdict::Rejected => M::WordRejected,
            Verdict::Malformed(_) => M::WordUnreadable,
        };
        details.push(fill(M::StepVerifier.text(language), &[("verdict", verdict.text(language))]));
    }
    if !step.violated.is_empty() {
        details.push(fill(M::StepBroken.text(language), &[("labels", &step.violated.join("; "))]));
    }
    details.push(fill(
        M::StepAfter.text(language),
        &[
            ("pool", &coins(step.pool_balance)),
            ("attacker", &coins(step.attacker_balance)),
            ("nullifiers", &step.nullifiers.to_string()),
        ],
    ));
    for detail in details {
        doc.led(indent(), vec![span(detail, secondary)]);
    }
    doc
}
