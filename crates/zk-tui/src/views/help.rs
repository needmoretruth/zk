//! `/help` and the `?` shortcuts overlay.

use zk_i18n::Language;

use crate::commands::SPECS;
use crate::doc::{Block, Doc, Entry, Hue, Kind, Table, Tone, plain, span};
use crate::phrases::ui::Msg as U;

/// Every command with its usage and what it does.
pub(crate) fn help(language: Language) -> Entry {
    let mut doc = Doc::new();
    doc.line(vec![span(U::HelpCommands.text(language), Tone::BODY.bold())]);
    let rows = SPECS
        .iter()
        .map(|spec| {
            vec![
                vec![plain(spec.usage)],
                vec![span(spec.summary.text(language), Tone::of(Hue::Secondary))],
            ]
        })
        .collect();
    doc.push(Block::Table(Table { rows, wrap: true, ..Table::default() }));
    doc.line(vec![span(U::HelpShortcutsHint.text(language), Tone::of(Hue::Secondary))]);
    Entry::new(Kind::Result, doc)
}

/// Key and description pairs for the shortcuts overlay.
pub(crate) fn shortcuts(language: Language) -> Vec<(&'static str, &'static str)> {
    [
        ("enter", U::KeyEnter),
        ("shift+enter, ctrl+j", U::KeyNewline),
        ("tab", U::KeyTab),
        ("↑ ↓", U::KeyUpDown),
        ("esc", U::KeyEsc),
        ("ctrl+c", U::KeyCtrlC),
        ("pgup pgdn, wheel", U::KeyScroll),
        ("ctrl+o", U::KeyCtrlO),
        ("ctrl+l", U::KeyCtrlL),
        ("?", U::KeyQuestion),
        ("shift+drag", U::KeyShiftDrag),
    ]
    .into_iter()
    .map(|(keys, msg)| (keys, msg.text(language)))
    .collect()
}
