//! The start screen: one rounded box, no logo.

use zk_i18n::Language;

use crate::doc::{Block, Doc, Entry, Hue, Kind, Span, Tone, plain, span};
use crate::museum::Museum;
use crate::phrases::catalog::Msg as C;
use crate::phrases::ui::Msg as U;
use crate::text::width;

/// The welcome box: the name, what this is, three commands to try and how to switch to Korean.
pub fn welcome(museum: &Museum, language: Language) -> Entry {
    let secondary = Tone::of(Hue::Secondary);
    let tries = [
        ("/list", U::WelcomeList),
        ("/examples", U::WelcomeExamples),
        ("/run all one-plus-one", U::WelcomeRunAll),
    ];
    let command_width = tries.iter().map(|(command, _)| width(command)).max().unwrap_or(0);
    let row = |command: &str, words: &str| -> Vec<Span> {
        let pad = " ".repeat(command_width - width(command) + 2);
        vec![span(command, Tone::BODY.bold()), plain(pad), span(words, secondary)]
    };

    let mut lines = vec![
        vec![
            span("nmtzk", Tone::BODY.bold()),
            span(format!(" v{}", env!("CARGO_PKG_VERSION")), secondary),
        ],
        vec![plain(U::WelcomeTagline.text(language))],
        Vec::new(),
    ];
    lines.extend(tries.iter().map(|(command, words)| row(command, words.text(language))));
    lines.push(Vec::new());
    lines.push(row("/lang ko", U::WelcomeLang.text(language)));
    if museum.systems.is_empty() {
        lines.push(Vec::new());
        lines.push(super::caution(C::NoSystems.text(language)));
    }
    let mut doc = Doc::new();
    doc.push(Block::Boxed(lines));
    Entry::new(Kind::Welcome, doc)
}
