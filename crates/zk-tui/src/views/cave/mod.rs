//! What the cave shows: the forked cave drawn, one line per scene, the tape and the court's counts.

use serde_json::json;
use zk_cave::odds::{survival_one_in, survival_probability};
use zk_cave::{Coin, Demonstration, Edit, Scene, Side, WallOutcome};
use zk_i18n::Language;

use crate::activities::cave::{CaveMode, CaveRun};
use crate::activities::{grouped, one_in};
use crate::doc::{Block, Doc, Hue, Span, Tone, plain, span};
use crate::phrases::cave::Msg as M;
use crate::phrases::fill;
use crate::text::width;

mod court;

pub(crate) use court::{averages, court_beat, court_record, hearing};

/// The top of the story cell: title, cautions, the cave, the trust line and what happens next.
pub(crate) fn opening(run: &CaveRun) -> Doc {
    let language = run.language;
    let mode = mode_name(run.mode).text(language);
    let (template, count) = match run.mode {
        CaveMode::Apartment => (M::TitleFloors, "floors"),
        _ => (M::TitleScenes, "scenes"),
    };
    let title = fill(
        template.text(language),
        &[("mode", mode), ("example", run.example.id()), (count, &run.scenes.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(vec![span(title, Tone::BODY.bold())]);
    for line in super::cautions(&zk_cave::META, language) {
        doc.line(line);
    }
    doc.push(drawing(language));
    doc.line(super::caution(M::TrustInTheWall.text(language)));
    doc.line(vec![plain(intro(run.mode).text(language))]);
    doc
}

/// A mode's display name.
pub(crate) fn mode_name(mode: CaveMode) -> M {
    match mode {
        CaveMode::Demonstration => M::ModeDemonstration,
        CaveMode::Impostor => M::ModeImpostor,
        CaveMode::JealousEdit => M::ModeJealousEdit,
        CaveMode::Court => M::ModeCourt,
        CaveMode::PriorAgreement => M::ModePriorAgreement,
        CaveMode::Apartment => M::ModeApartment,
    }
}

fn intro(mode: CaveMode) -> M {
    match mode {
        CaveMode::Demonstration => M::IntroDemonstration,
        CaveMode::Impostor => M::IntroImpostor,
        CaveMode::JealousEdit => M::IntroJealousEdit,
        CaveMode::Court => M::IntroCourt,
        CaveMode::PriorAgreement => M::IntroPriorAgreement,
        CaveMode::Apartment => M::IntroApartment,
    }
}

/// The forked cave: an entrance, a fork, two bent passages and two dead ends with the wall between
/// them. Labels are measured so a translation keeps the passages lined up.
pub(crate) fn drawing(language: Language) -> Block {
    let [dead_end, wall, left, right, fork, entrance] = [
        M::DrawingDeadEnd,
        M::DrawingWall,
        M::DrawingLeft,
        M::DrawingRight,
        M::DrawingFork,
        M::DrawingEntrance,
    ]
    .map(|msg| msg.text(language));
    // `m` is the column of the left passage; the fork and the wall sit seven columns to its right.
    let m = 2 + (width(left) + 1).max(width(dead_end).saturating_sub(6)).max(width(entrance) / 2);
    let at = |column: usize, text: &str| format!("{}{text}", " ".repeat(column));
    let centred = |text: &str| at((m + 7).saturating_sub(width(text).saturating_sub(1) / 2), text);
    let rows = [
        at(m + 6 - width(dead_end), &format!("{dead_end} ┃ {dead_end}")),
        at(m, "╭──────┃──────╮"),
        format!("{}│{}│", " ".repeat(m), pad_centre(wall, 13)),
        format!("{}{left} │             │ {right}", " ".repeat(m - width(left) - 1)),
        at(m, "╰────╮   ╭────╯"),
        at(m + 5, "╰─┬─╯"),
        at(m + 7, &format!("│  {fork}")),
        at(m + 7, "│"),
        centred(entrance),
    ];
    let tone = Tone::of(Hue::Secondary);
    Block::Pre(rows.into_iter().map(|row| vec![span(row, tone)]).collect())
}

/// `text` centred in `columns` columns, or as it is when it is wider.
fn pad_centre(text: &str, columns: usize) -> String {
    let room = columns.saturating_sub(width(text));
    format!("{}{text}{}", " ".repeat(room / 2), " ".repeat(room - room / 2))
}

pub(super) fn side(side: Side, language: Language) -> &'static str {
    match side {
        Side::Left => M::SideLeft,
        Side::Right => M::SideRight,
    }
    .text(language)
}

fn letter(side: Side, language: Language) -> &'static str {
    match side {
        Side::Left => M::LetterLeft,
        Side::Right => M::LetterRight,
    }
    .text(language)
}

pub(super) fn mark(good: bool) -> Span {
    if good { span("✓ ", Tone::of(Hue::Success)) } else { span("✗ ", Tone::of(Hue::Failure)) }
}

/// One scene, take or floor on one line: the coin, the call, the exit and the wall. `kept` is
/// `Some` for a take of the jealous edit, saying whether it was kept.
pub(crate) fn scene_beat(
    label: M,
    scene: &Scene,
    kept: Option<Option<u32>>,
    language: Language,
) -> Doc {
    let number = if label == M::FloorLabel { scene.at.floor } else { scene.at.take };
    let coin = match scene.coin {
        Some(Coin::Heads) => M::CoinHeads,
        Some(Coin::Tails) => M::CoinTails,
        None => M::NoCoin,
    };
    let wall = match scene.wall {
        WallOutcome::NotNeeded => M::WallNotNeeded,
        WallOutcome::Opened => M::WallOpened,
        WallOutcome::StayedShut => M::WallStayedShut,
        WallOutcome::NothingToWhisper => M::WallNoWords,
    };
    let mut parts = vec![
        coin.text(language).to_string(),
        fill(M::Called.text(language), &[("side", side(scene.call, language))]),
        fill(M::CameOut.text(language), &[("side", side(scene.exit, language))]),
        wall.text(language).to_string(),
    ];
    match kept {
        Some(Some(number)) => {
            parts.push(fill(M::KeptAs.text(language), &[("number", &number.to_string())]));
        }
        Some(None) => parts.push(M::Cut.text(language).to_string()),
        None if !scene.succeeded() => parts.push(M::Caught.text(language).to_string()),
        None => {}
    }
    let label = fill(label.text(language), &[("number", &number.to_string())]);
    let mut body = vec![span(label, Tone::BODY.bold())];
    for part in parts {
        body.push(span(" · ", Tone::of(Hue::Secondary)));
        body.push(plain(part));
    }
    let mut doc = Doc::new();
    doc.led(vec![mark(scene.succeeded())], body);
    doc
}

/// A scene as a JSON record, including what only the prover knew.
pub(crate) fn scene_record(scene: &Scene) -> serde_json::Value {
    let side = |side: Side| match side {
        Side::Left => "left",
        Side::Right => "right",
    };
    json!({
        "take": scene.at.take,
        "floor": scene.at.floor,
        "entered": side(scene.entered),
        "coin": scene.coin.map(|coin| match coin {
            Coin::Heads => "heads",
            Coin::Tails => "tails",
        }),
        "call": side(scene.call),
        "exit": side(scene.exit),
        "wall": match scene.wall {
            WallOutcome::NotNeeded => "not-needed",
            WallOutcome::Opened => "opened",
            WallOutcome::StayedShut => "stayed-shut",
            WallOutcome::NothingToWhisper => "nothing-to-whisper",
        },
        "succeeded": scene.succeeded(),
    })
}

/// Where a stop cut the filming short.
pub(crate) fn stopped(done: usize, total: usize, language: Language) -> Doc {
    let text = fill(
        M::Stopped.text(language),
        &[("done", &done.to_string()), ("total", &total.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc
}

/// Said when a stop came while the court was averaging.
pub(crate) fn stopped_counting(language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(super::caution(M::StoppedCounting.text(language)));
    doc
}

/// Letters in groups of ten, so a reader can find scene 23.
fn letters(sides: impl Iterator<Item = Side>, language: Language) -> String {
    let all: Vec<&str> = sides.map(|s| letter(s, language)).collect();
    all.chunks(10).map(|group| group.concat()).collect::<Vec<_>>().join(" ")
}

/// After filming: the calls and exits the camera recorded, and the passages it could not see.
pub(crate) fn review(scenes: &[Scene], language: Language) -> Doc {
    let secondary = Tone::of(Hue::Secondary);
    let mut doc = Doc::new();
    doc.line(vec![span(M::ReviewHeading.text(language), Tone::BODY.bold())]);
    doc.pairs(vec![
        (
            M::ReviewCalled.text(language).to_string(),
            vec![plain(letters(scenes.iter().map(|s| s.call), language))],
        ),
        (
            M::ReviewCameOut.text(language).to_string(),
            vec![plain(letters(scenes.iter().map(|s| s.exit), language))],
        ),
        (
            M::ReviewWentIn.text(language).to_string(),
            vec![
                plain(letters(scenes.iter().map(|s| s.entered), language)),
                span(format!(" · {}", M::ReviewNotOnTape.text(language)), secondary),
            ],
        ),
    ]);
    doc
}

/// Who the reporter now believes, and the odds of getting that far by luck.
pub(crate) fn verdict(mode: CaveMode, played: &Demonstration, language: Language) -> Doc {
    let scenes = played.planned.to_string();
    let (good, text) = match (mode, played.caught_at) {
        (CaveMode::PriorAgreement, _) => {
            (true, fill(M::VerdictAgreed.text(language), &[("scenes", &scenes)]))
        }
        (_, Some(scene)) => {
            (true, fill(M::VerdictCaught.text(language), &[("scene", &scene.to_string())]))
        }
        (CaveMode::Impostor, None) => {
            (false, fill(M::VerdictEscaped.text(language), &[("scenes", &scenes)]))
        }
        (_, None) => (true, fill(M::VerdictConvinced.text(language), &[("scenes", &scenes)])),
    };
    let mut doc = Doc::new();
    if good {
        doc.led(vec![mark(true)], vec![span(text, Tone::BODY.bold())]);
    } else {
        doc.line(super::caution(text));
    }
    doc.line(vec![span(odds(played.planned, language), Tone::of(Hue::Secondary))]);
    doc
}

fn odds(scenes: u32, language: Language) -> String {
    let count = scenes.to_string();
    match survival_one_in(scenes) {
        Some(n) => fill(M::OddsExact.text(language), &[("scenes", &count), ("odds", &grouped(n))]),
        None => fill(
            M::OddsApprox.text(language),
            &[("scenes", &count), ("odds", &one_in(survival_probability(scenes)))],
        ),
    }
}

/// The calls the reporter and the double settled before filming.
pub(crate) fn agreed(calls: &[Side], language: Language) -> Doc {
    let text = fill(
        M::AgreedCalls.text(language),
        &[("calls", &letters(calls.iter().copied(), language))],
    );
    let mut doc = Doc::new();
    doc.line(vec![plain(text)]);
    doc
}

/// How many takes the edit cost and what it kept.
pub(crate) fn edit_summary(edit: &Edit, keep: u32, language: Language) -> Doc {
    let summary = fill(
        M::EditSummary.text(language),
        &[
            ("takes", &edit.takes.len().to_string()),
            ("kept", &edit.kept.len().to_string()),
            ("cut", &edit.discarded().to_string()),
        ],
    );
    let expected = fill(
        M::EditExpected.text(language),
        &[
            ("kept", &keep.to_string()),
            ("expected", &zk_cave::odds::expected_takes(keep).to_string()),
        ],
    );
    let mut doc = Doc::new();
    doc.led(vec![mark(true)], vec![span(summary, Tone::BODY.bold())]);
    doc.line(vec![span(expected, Tone::of(Hue::Secondary))]);
    doc.line(vec![plain(M::EditNote.text(language))]);
    doc
}

/// Whether every floor came out right, and the odds of passing them all by luck.
pub(crate) fn building_verdict(failed: &[u32], floors: usize, language: Language) -> Doc {
    let count = floors.to_string();
    let mut doc = Doc::new();
    if failed.is_empty() {
        let text = fill(M::BuildingConvinced.text(language), &[("floors", &count)]);
        doc.led(vec![mark(true)], vec![span(text, Tone::BODY.bold())]);
    } else {
        let list: Vec<String> = failed.iter().map(u32::to_string).collect();
        let text = fill(M::BuildingFailed.text(language), &[("floors", &list.join(", "))]);
        doc.led(vec![mark(false)], vec![span(text, Tone::of(Hue::Failure).bold())]);
    }
    let floors = u32::try_from(floors).unwrap_or(u32::MAX);
    let odds = match survival_one_in(floors) {
        Some(n) => grouped(n),
        None => one_in(survival_probability(floors)),
    };
    let text = fill(M::BuildingOdds.text(language), &[("floors", &count), ("odds", &odds)]);
    doc.line(vec![span(text, Tone::of(Hue::Secondary))]);
    doc
}
