//! What Trio shows: one beat per round with its card, what it opened and every check.

use serde_json::json;
use zk_i18n::Language;
use zk_trio::{Card, CheckKind, ClaimSource, FalseClaim, Opened, RoundEvent, Session};

use crate::activities::one_in;
use crate::activities::trio::{TrioCheat, TrioMode, TrioRun};
use crate::doc::{Doc, Hue, Span, Tone, plain, span};
use crate::phrases::fill;
use crate::phrases::trio::Msg as M;

/// The top of the story cell: title, the homemade caution and what is about to happen.
pub(crate) fn opening(run: &TrioRun) -> Doc {
    let language = run.language;
    let mode = match run.mode {
        TrioMode::Play => M::ModePlay.text(language).to_string(),
        TrioMode::Cheat(cheat) => {
            fill(M::ModeCheat.text(language), &[("cheat", cheat_name(cheat).text(language))])
        }
        TrioMode::Simulate => M::ModeSimulate.text(language).to_string(),
    };
    let title = fill(
        M::Title.text(language),
        &[("mode", &mode), ("example", run.example.id()), ("rounds", &run.rounds.to_string())],
    );
    let intro = match run.mode {
        TrioMode::Play => M::IntroPlay,
        TrioMode::Cheat(_) => M::IntroCheat,
        TrioMode::Simulate => M::IntroSimulate,
    };
    let mut doc = Doc::new();
    doc.line(vec![span(title, Tone::BODY.bold())]);
    for line in super::cautions(&zk_trio::META, language) {
        doc.line(line);
    }
    doc.line(vec![plain(intro.text(language))]);
    doc
}

/// A cheat's display name.
pub(crate) fn cheat_name(cheat: TrioCheat) -> M {
    match cheat {
        TrioCheat::BadCard => M::CheatBadCard,
        TrioCheat::BadComputation => M::CheatBadComputation,
        TrioCheat::RewriteHidden => M::CheatRewriteHidden,
    }
}

/// The lie a cheat tells, and where its false claim comes from.
pub(crate) fn lie(cheat: TrioCheat, claim: &FalseClaim, language: Language) -> Doc {
    let how = match (cheat, claim.rigged) {
        (TrioCheat::BadCard, Some(rigged)) => fill(
            M::LieBadCard.text(language),
            &[("index", &(rigged.multiplication + 1).to_string())],
        ),
        (TrioCheat::BadCard, None) => String::new(),
        (TrioCheat::BadComputation, _) => M::LieBadComputation.text(language).to_string(),
        (TrioCheat::RewriteHidden, _) => M::LieRewriteHidden.text(language).to_string(),
    };
    let source = match claim.source {
        ClaimSource::ExampleFalseClaim => M::SourceExample.text(language).to_string(),
        ClaimSource::BumpedPublicInput(index) => {
            fill(M::SourceBumped.text(language), &[("index", &(index + 1).to_string())])
        }
    };
    let mut doc = Doc::new();
    doc.line(super::caution(format!("{source} {how}").trim_end()));
    doc
}

fn short(card: Card, language: Language) -> String {
    match card {
        Card::DealerA => M::CardShortDealerA.text(language).to_string(),
        Card::DealerB => M::CardShortDealerB.text(language).to_string(),
        Card::Peek(friend) => {
            fill(M::CardShortPeek.text(language), &[("friend", &friend.number().to_string())])
        }
    }
}

/// The cards the simulator was told before the first round.
pub(crate) fn plan(cards: &[Card], language: Language) -> Doc {
    let list: Vec<String> = cards.iter().map(|card| short(*card, language)).collect();
    let text = fill(M::PlannedCards.text(language), &[("cards", &list.join(" "))]);
    let mut doc = Doc::new();
    doc.line(vec![plain(text)]);
    doc
}

fn mark(good: bool) -> Span {
    if good { span("✓", Tone::of(Hue::Success)) } else { span("✗", Tone::of(Hue::Failure)) }
}

fn card_name(card: Card, language: Language) -> String {
    match card {
        Card::DealerA => M::CardDealerA.text(language).to_string(),
        Card::DealerB => M::CardDealerB.text(language).to_string(),
        Card::Peek(friend) => {
            fill(M::CardPeek.text(language), &[("friend", &friend.number().to_string())])
        }
    }
}

fn opened_name(opened: Opened, language: Language) -> String {
    let (msg, friend) = match opened {
        Opened::CardSeed(friend) => (M::OpenedCardSeed, Some(friend)),
        Opened::InputSeed(friend) => (M::OpenedInputSeed, Some(friend)),
        Opened::CardCorrection => (M::OpenedCardCorrection, None),
        Opened::InputCorrection => (M::OpenedInputCorrection, None),
        Opened::ViewKey(friend) => (M::OpenedViewKey, Some(friend)),
        Opened::Broadcasts(friend) => (M::OpenedBroadcasts, Some(friend)),
    };
    let number = friend.map_or_else(String::new, |friend| friend.number().to_string());
    fill(msg.text(language), &[("friend", &number)])
}

fn check_name(kind: CheckKind, language: Language) -> String {
    let (msg, friend) = match kind {
        CheckKind::OpeningFitsCard => (M::CheckFits, None),
        CheckKind::CardsMultiply => (M::CheckMultiply, None),
        CheckKind::CardCommitment(friend) => (M::CheckCardCommitment, Some(friend)),
        CheckKind::ViewCommitment(friend) => (M::CheckViewCommitment, Some(friend)),
        CheckKind::AssertionsVanish => (M::CheckSums, None),
    };
    let number = friend.map_or_else(String::new, |friend| friend.number().to_string());
    fill(msg.text(language), &[("friend", &number)])
}

/// One round: the card, what it opened, and each check with its result.
pub(crate) fn round(event: &RoundEvent, language: Language) -> Doc {
    let secondary = Tone::of(Hue::Secondary);
    let label = fill(M::RoundLabel.text(language), &[("round", &event.round.to_string())]);
    let mut doc = Doc::new();
    doc.led(
        vec![mark(event.passed), plain(" ")],
        vec![
            span(label, Tone::BODY.bold()),
            span(" · ", secondary),
            plain(card_name(event.card, language)),
        ],
    );
    let opened: Vec<String> = event.opened.iter().map(|o| opened_name(*o, language)).collect();
    doc.led(
        vec![plain("  ")],
        vec![
            span(format!("{}: ", M::OpenedLabel.text(language)), secondary),
            plain(opened.join(", ")),
        ],
    );
    let mut checks = vec![span(format!("{}: ", M::ChecksLabel.text(language)), secondary)];
    for (index, check) in event.checks.iter().enumerate() {
        if index > 0 {
            checks.push(span(" · ", secondary));
        }
        checks.push(plain(format!("{} ", check_name(check.kind, language))));
        checks.push(mark(check.passed));
    }
    doc.led(vec![plain("  ")], checks);
    doc
}

/// A round as a JSON record.
pub(crate) fn round_record(event: &RoundEvent) -> serde_json::Value {
    let friend = |kind: CheckKind| match kind {
        CheckKind::CardCommitment(friend) | CheckKind::ViewCommitment(friend) => {
            Some(friend.number())
        }
        _ => None,
    };
    let checks: Vec<serde_json::Value> = event
        .checks
        .iter()
        .map(|check| {
            json!({
                "check": check_key(check.kind),
                "friend": friend(check.kind),
                "passed": check.passed,
                "first_failure": check.first_failure,
            })
        })
        .collect();
    let opened: Vec<String> = event.opened.iter().map(|o| opened_key(*o)).collect();
    json!({
        "round": event.round,
        "of": event.of,
        "card": match event.card {
            Card::DealerA => "dealer-a",
            Card::DealerB => "dealer-b",
            Card::Peek(_) => "peek",
        },
        "hidden": event.hidden.map(|friend| friend.number()),
        "opened": opened,
        "checks": checks,
        "passed": event.passed,
    })
}

fn check_key(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::OpeningFitsCard => "opening-fits-card",
        CheckKind::CardsMultiply => "cards-multiply",
        CheckKind::CardCommitment(_) => "card-commitment",
        CheckKind::ViewCommitment(_) => "view-commitment",
        CheckKind::AssertionsVanish => "assertions-vanish",
    }
}

fn opened_key(opened: Opened) -> String {
    match opened {
        Opened::CardSeed(friend) => format!("card-seed-{}", friend.number()),
        Opened::InputSeed(friend) => format!("input-seed-{}", friend.number()),
        Opened::CardCorrection => "card-correction".to_string(),
        Opened::InputCorrection => "input-correction".to_string(),
        Opened::ViewKey(friend) => format!("view-key-{}", friend.number()),
        Opened::Broadcasts(friend) => format!("broadcasts-{}", friend.number()),
    }
}

/// Where a stop left the conversation.
pub(crate) fn stopped(done: u32, total: u32, language: Language) -> Doc {
    let text = fill(
        M::Stopped.text(language),
        &[("done", &done.to_string()), ("total", &total.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc
}

/// Why the first failed check in `event` failed, in words.
fn caught_by(event: &RoundEvent, language: Language) -> String {
    let Some(check) = event.checks.iter().find(|check| !check.passed) else {
        return String::new();
    };
    let index = check.first_failure.map_or_else(String::new, |i| (i + 1).to_string());
    let (msg, friend) = match check.kind {
        CheckKind::OpeningFitsCard => (M::CaughtFits, None),
        CheckKind::CardsMultiply => (M::CaughtMultiply, None),
        CheckKind::CardCommitment(friend) => (M::CaughtCardCommitment, Some(friend)),
        CheckKind::ViewCommitment(friend) => (M::CaughtViewCommitment, Some(friend)),
        CheckKind::AssertionsVanish => (M::CaughtSums, None),
    };
    let friend = friend.map_or_else(String::new, |friend| friend.number().to_string());
    fill(msg.text(language), &[("index", &index), ("friend", &friend)])
}

/// How the conversation ended, and the odds of a cheat getting that far.
pub(crate) fn verdict(
    run: &TrioRun,
    session: &Session<'_>,
    last: Option<&RoundEvent>,
    language: Language,
) -> Doc {
    let rounds = run.rounds.to_string();
    let mut doc = Doc::new();
    let good = |doc: &mut Doc, text: String| {
        doc.led(vec![mark(true), plain(" ")], vec![span(text, Tone::BODY.bold())]);
    };
    match (session.caught_at(), last, run.mode) {
        (Some(round), Some(event), _) => {
            let why = caught_by(event, language);
            let text = fill(
                M::VerdictCaught.text(language),
                &[("round", &round.to_string()), ("why", &why)],
            );
            let tone =
                if matches!(run.mode, TrioMode::Cheat(_)) { Hue::Success } else { Hue::Failure };
            doc.led(
                vec![mark(tone == Hue::Success), plain(" ")],
                vec![span(text, Tone::of(tone).bold())],
            );
        }
        (_, _, TrioMode::Cheat(_)) => {
            doc.line(super::caution(fill(
                M::VerdictEscaped.text(language),
                &[("rounds", &rounds)],
            )));
        }
        (_, _, TrioMode::Simulate) => {
            good(&mut doc, fill(M::VerdictSimulated.text(language), &[("rounds", &rounds)]));
            doc.line(vec![plain(M::SimulatedNote.text(language))]);
        }
        _ => good(&mut doc, fill(M::VerdictAccepted.text(language), &[("rounds", &rounds)])),
    }
    let odds = one_in(zk_trio::escape_probability(run.rounds));
    let text = fill(M::Odds.text(language), &[("rounds", &rounds), ("odds", &odds)]);
    doc.line(vec![span(text, Tone::of(Hue::Secondary))]);
    doc
}
