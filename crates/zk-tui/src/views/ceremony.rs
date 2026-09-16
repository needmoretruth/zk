//! What the ceremony shows: the forged proof, every turn with its fingerprint, every check, and
//! the colluders' opening.

use zk_ceremony::tau::Check;
use zk_i18n::Language;

use crate::activities::ceremony::{COEFFICIENTS, CeremonyPart, CeremonyRun, LIE, POINT};
use crate::activities::{hex, short_hex};
use crate::doc::{Doc, Hue, Span, Tone, plain, span};
use crate::phrases::catalog::Msg as C;
use crate::phrases::ceremony::Msg as M;
use crate::phrases::fill;

/// Bytes of a fingerprint or a point shown on screen.
const SHOWN_BYTES: usize = 6;

/// The top of the story cell: title, the teaching caution and what is about to happen.
pub(crate) fn opening(run: &CeremonyRun) -> Doc {
    let language = run.language;
    let participants = run.participants.to_string();
    let (title, intro) = match run.part {
        CeremonyPart::Toxic => (M::TitleToxic.text(language).to_string(), M::IntroToxic),
        CeremonyPart::Tau => {
            (fill(M::TitleTau.text(language), &[("participants", &participants)]), M::IntroTau)
        }
        CeremonyPart::Collude => (
            fill(M::TitleCollude.text(language), &[("participants", &participants)]),
            M::IntroCollude,
        ),
    };
    let mut doc = Doc::new();
    doc.line(vec![span(title, Tone::BODY.bold())]);
    doc.line(super::caution(C::CautionTeaching.text(language)));
    doc.line(vec![plain(intro.text(language))]);
    doc
}

fn mark(good: bool) -> Span {
    if good { span("✓ ", Tone::of(Hue::Success)) } else { span("✗ ", Tone::of(Hue::Failure)) }
}

/// A line led by `✓` or `✗`; a bad outcome is red.
fn marked(good: bool, text: impl Into<String>) -> Doc {
    let tone = if good { Tone::BODY } else { Tone::of(Hue::Failure) };
    let mut doc = Doc::new();
    doc.led(vec![mark(good)], vec![span(text, tone)]);
    doc
}

/// Where a stop left the ceremony.
pub(crate) fn stopped(done: usize, total: usize, language: Language) -> Doc {
    let text = fill(
        M::Stopped.text(language),
        &[("done", &done.to_string()), ("total", &total.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc
}

/// The keys, made from numbers everyone can see here.
pub(crate) fn toxic_keys(language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(super::caution(M::ToxicKeys.text(language)));
    doc
}

/// The claim that is about to be proved.
pub(crate) fn toxic_claim(language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(vec![plain(M::ToxicClaim.text(language))]);
    doc
}

/// The forged proof's three points and the ordinary verifier's answer. Accepting it is what the
/// lesson shows, so it is drawn as the break it is.
pub(crate) fn toxic_forged(points: &[String; 3], accepted: bool, language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(vec![plain(M::ToxicForged.text(language))]);
    let items = ["A", "B", "C"]
        .iter()
        .zip(points)
        .map(|(name, bytes)| {
            let text = fill(
                M::ToxicPoint.text(language),
                &[("name", name), ("bytes", &short_hex(bytes, SHOWN_BYTES))],
            );
            vec![span(text, Tone::of(Hue::Secondary))]
        })
        .collect();
    doc.tree(items);
    let verdict = if accepted { M::ToxicAccepted } else { M::ToxicNotAccepted };
    doc.append(marked(false, verdict.text(language)));
    doc
}

/// The forgery again with a δ nobody knows, and what the lesson is.
pub(crate) fn toxic_guessed(accepted: bool, language: Language) -> Doc {
    let verdict = if accepted { M::ToxicGuessAccepted } else { M::ToxicGuessRejected };
    let mut doc = marked(!accepted, verdict.text(language));
    doc.line(vec![span(M::ToxicLesson.text(language), Tone::BODY.bold())]);
    doc
}

fn fingerprint(bytes: &[u8; 32]) -> String {
    short_hex(&hex(bytes), SHOWN_BYTES)
}

/// The string before anyone's turn.
pub(crate) fn tau_start(digest: &[u8; 32], language: Language) -> Doc {
    let text = fill(M::TauStart.text(language), &[("fingerprint", &fingerprint(digest))]);
    let mut doc = Doc::new();
    doc.line(vec![span(text, Tone::of(Hue::Secondary))]);
    doc
}

/// One participant's turn and the string's fingerprint after it.
pub(crate) fn turn(number: usize, digest: &[u8; 32], kept: bool, language: Language) -> Doc {
    let msg = if kept { M::TurnKept } else { M::TurnDiscarded };
    let text = fill(
        msg.text(language),
        &[("number", &number.to_string()), ("fingerprint", &fingerprint(digest))],
    );
    let mut doc = Doc::new();
    if kept {
        doc.line(super::caution(text));
    } else {
        doc.led(vec![mark(true)], vec![plain(text)]);
    }
    doc
}

/// Every check, in the order the verifier runs them.
const CHECKS: [Check; 5] = [
    Check::WellFormed,
    Check::KnowledgeProof,
    Check::SameSecret,
    Check::BuildsOnPrevious,
    Check::ConsistentPowers,
];

fn check_name(check: Check, language: Language) -> &'static str {
    match check {
        Check::WellFormed => M::CheckWellFormed,
        Check::KnowledgeProof => M::CheckKnowledgeProof,
        Check::SameSecret => M::CheckSameSecret,
        Check::BuildsOnPrevious => M::CheckBuildsOnPrevious,
        Check::ConsistentPowers => M::CheckConsistentPowers,
    }
    .text(language)
}

/// A check's name in a JSON record.
pub(crate) fn check_key(check: Check) -> &'static str {
    match check {
        Check::WellFormed => "well-formed",
        Check::KnowledgeProof => "knowledge-proof",
        Check::SameSecret => "same-secret",
        Check::BuildsOnPrevious => "builds-on-previous",
        Check::ConsistentPowers => "consistent-powers",
    }
}

/// One turn checked: every check passed up to `failed`, which failed; later ones did not run.
pub(crate) fn checked(number: usize, failed: Option<Check>, language: Language) -> Doc {
    let secondary = Tone::of(Hue::Secondary);
    let label = fill(M::CheckedTurn.text(language), &[("number", &number.to_string())]);
    let mut body = vec![span(format!("{label} "), Tone::BODY.bold())];
    let ran =
        CHECKS.iter().position(|check| Some(*check) == failed).map_or(CHECKS.len(), |i| i + 1);
    for (index, check) in CHECKS.iter().take(ran).enumerate() {
        if index > 0 {
            body.push(span(" · ", secondary));
        }
        body.push(plain(format!("{} ", check_name(*check, language))));
        let good = Some(*check) != failed;
        body.push(span(
            if good { "✓" } else { "✗" },
            Tone::of(if good { Hue::Success } else { Hue::Failure }),
        ));
    }
    let mut doc = Doc::new();
    doc.led(vec![mark(failed.is_none())], body);
    doc
}

/// Whether the whole chain checks out.
pub(crate) fn chain_verdict(participants: usize, verified: bool, language: Language) -> Doc {
    let text = if verified {
        fill(M::ChainVerified.text(language), &[("participants", &participants.to_string())])
    } else {
        M::ChainRejected.text(language).to_string()
    };
    let mut doc = Doc::new();
    doc.led(vec![mark(verified)], vec![span(text, Tone::BODY.bold())]);
    doc
}

fn caught_line(caught: Option<Check>, language: Language) -> Doc {
    match caught {
        Some(check) => marked(
            true,
            fill(M::CaughtBy.text(language), &[("check", check_name(check, language))]),
        ),
        None => marked(false, M::NotCaught.text(language)),
    }
}

/// A participant who throws away every earlier turn, and the check that catches it.
pub(crate) fn cheat_ignored(number: usize, caught: Option<Check>, language: Language) -> Doc {
    let text = fill(M::CheatIgnored.text(language), &[("number", &number.to_string())]);
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc.append(caught_line(caught, language));
    doc
}

/// A participant who swaps one power, and the check that catches it.
pub(crate) fn cheat_replaced(
    number: usize,
    power: usize,
    caught: Option<Check>,
    language: Language,
) -> Doc {
    let text = fill(
        M::CheatReplaced.text(language),
        &[("number", &number.to_string()), ("power", &power.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc.append(caught_line(caught, language));
    doc
}

/// `p(POINT)` for the committed polynomial, and the value the colluders claim instead.
fn values() -> (u64, u64) {
    let value = COEFFICIENTS.iter().rev().fold(0u64, |acc, c| acc * POINT + c);
    (value, value + LIE)
}

/// The honest opening of the commitment.
pub(crate) fn honest_opening(accepted: bool, language: Language) -> Doc {
    let point = POINT.to_string();
    let (value, _) = values();
    let msg = if accepted { M::HonestOpening } else { M::HonestOpeningRejected };
    marked(accepted, fill(msg.text(language), &[("point", &point), ("value", &value.to_string())]))
}

/// A forged opening made with `kept` of `total` secrets, and the ordinary verifier's answer.
pub(crate) fn forged_opening(kept: usize, total: usize, accepted: bool, language: Language) -> Doc {
    let (_, claimed) = values();
    let values = [
        ("kept", kept.to_string()),
        ("total", total.to_string()),
        ("point", POINT.to_string()),
        ("claimed", claimed.to_string()),
    ];
    let values: Vec<(&str, &str)> = values.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let all = kept == total;
    let msg = match (all, accepted) {
        (true, true) => M::ForgedAll,
        (true, false) => M::ForgedAllRejected,
        (false, false) => M::ForgedMissing,
        (false, true) => M::ForgedMissingAccepted,
    };
    // Every secret kept makes the forgery work: that acceptance is the break being shown.
    let mut doc = marked(!accepted, fill(msg.text(language), &values));
    if !all {
        doc.line(vec![span(M::ColludeLesson.text(language), Tone::BODY.bold())]);
    }
    doc
}
