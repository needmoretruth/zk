//! The court: two tapes side by side, and the counts a judge can make on them.

use serde_json::json;
use zk_cave::TapeScene;
use zk_cave::court::{Averages, Hearing, TapeStats};
use zk_i18n::Language;

use super::{mark, side};
use crate::doc::{Align, Block, Doc, Hue, Table, Tone, plain, span};
use crate::phrases::cave::Msg as M;
use crate::phrases::fill;

fn pair(scene: &TapeScene, language: Language) -> String {
    fill(
        M::TapePair.text(language),
        &[("call", side(scene.call, language)), ("exit", side(scene.exit, language))],
    )
}

/// One scene of each tape, side by side.
pub(crate) fn court_beat(
    number: usize,
    mick: &TapeScene,
    edited: &TapeScene,
    language: Language,
) -> Doc {
    let label = fill(M::SceneLabel.text(language), &[("number", &number.to_string())]);
    let secondary = Tone::of(Hue::Secondary);
    let mut doc = Doc::new();
    doc.led(
        vec![span(format!("{label}  "), Tone::BODY.bold())],
        vec![
            span(format!("{}: ", M::TapeMick.text(language)), secondary),
            plain(pair(mick, language)),
            span(format!(" · {}: ", M::TapeEdited.text(language)), secondary),
            plain(pair(edited, language)),
        ],
    );
    doc
}

fn count_table(header: [String; 3], rows: Vec<(M, String, String)>, language: Language) -> Block {
    let header = header.into_iter().map(|text| vec![plain(text)]).collect();
    let rows = rows
        .into_iter()
        .map(|(name, left, right)| {
            vec![vec![plain(name.text(language))], vec![plain(left)], vec![plain(right)]]
        })
        .collect();
    Block::Table(Table {
        header,
        align: vec![Align::Left, Align::Right, Align::Right],
        rows,
        ..Table::default()
    })
}

fn tape_headers(first: &str, language: Language) -> [String; 3] {
    [first.to_string(), M::TapeMick.text(language).into(), M::TapeEdited.text(language).into()]
}

/// The two tapes' counts in one table.
pub(crate) fn hearing(hearing: &Hearing, language: Language) -> Doc {
    let (g, e) = (&hearing.genuine, &hearing.edited);
    let row =
        |name: M, value: fn(&TapeStats) -> u32| (name, value(g).to_string(), value(e).to_string());
    let rows = vec![
        row(M::CountScenes, |s| s.scenes),
        row(M::CountRightCalls, |s| s.right_calls),
        row(M::CountLeftCalls, |s| s.left_calls),
        row(M::CountMatching, |s| s.exits_matching_call),
        row(M::CountRuns, |s| s.runs),
        row(M::CountLongestRun, |s| s.longest_run),
    ];
    let mut doc = Doc::new();
    doc.line(vec![span(M::CourtCounts.text(language), Tone::BODY.bold())]);
    doc.push(count_table(tape_headers("", language), rows, language));
    doc
}

/// The same counts over many tapes, and what the court concludes.
pub(crate) fn averages(
    tapes: u32,
    genuine: &Averages,
    edited: &Averages,
    language: Language,
) -> Doc {
    let two = |value: f64| format!("{value:.2}");
    let rows = vec![
        (M::AvgRightShare, two(genuine.right_call_share), two(edited.right_call_share)),
        (M::AvgMatchShare, two(genuine.match_share), two(edited.match_share)),
        (M::AvgRuns, two(genuine.runs), two(edited.runs)),
        (M::AvgLongestRun, two(genuine.longest_run), two(edited.longest_run)),
    ];
    let title = fill(M::CourtAverages.text(language), &[("tapes", &tapes.to_string())]);
    let mut doc = Doc::new();
    doc.line(vec![span(title, Tone::BODY.bold())]);
    doc.push(count_table(tape_headers("", language), rows, language));
    doc.line(vec![span(M::CourtChance.text(language), Tone::of(Hue::Secondary))]);
    doc.led(vec![mark(true)], vec![span(M::CourtVerdict.text(language), Tone::BODY.bold())]);
    doc
}

/// The court's counts as a JSON record.
pub(crate) fn court_record(
    hearing: &Hearing,
    genuine: &Averages,
    edited: &Averages,
) -> serde_json::Value {
    let stats = |s: &TapeStats| {
        json!({
            "scenes": s.scenes,
            "right_calls": s.right_calls,
            "left_calls": s.left_calls,
            "exits_matching_call": s.exits_matching_call,
            "runs": s.runs,
            "longest_run": s.longest_run,
        })
    };
    let average = |a: &Averages| {
        json!({
            "tapes": a.tapes,
            "right_call_share": a.right_call_share,
            "match_share": a.match_share,
            "runs": a.runs,
            "longest_run": a.longest_run,
        })
    };
    json!({
        "genuine": stats(&hearing.genuine),
        "edited": stats(&hearing.edited),
        "genuine_averages": average(genuine),
        "edited_averages": average(edited),
    })
}
