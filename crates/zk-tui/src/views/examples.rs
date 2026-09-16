//! `/examples`: the seven statements, what is public and what stays secret.

use zk_core::ExampleId;
use zk_i18n::Language;

use crate::doc::{Doc, Entry, Hue, Kind, Tone, plain, span};
use crate::phrases::catalog::Msg as C;
use crate::phrases::fill;
use crate::text::width;

/// Every example with its statement and input names.
pub fn examples(language: Language) -> Entry {
    let secondary = Tone::of(Hue::Secondary);
    let id_width = ExampleId::ALL.iter().map(|example| width(example.id())).max().unwrap_or(0) + 2;
    let mut doc = Doc::new();
    doc.line(vec![plain(C::ExamplesTitle.text(language))]);
    doc.blank();
    for example in ExampleId::ALL {
        let id = format!("{:<id_width$}", example.id());
        doc.led(
            vec![span(id, Tone::BODY.bold())],
            vec![plain(super::statement(example, language))],
        );
        let inputs = fill(
            C::ExamplesInputs.text(language),
            &[
                ("public", &names(example.public_input_names(), language)),
                ("private", &names(example.private_input_names(), language)),
            ],
        );
        doc.led(vec![plain(" ".repeat(id_width))], vec![span(inputs, secondary)]);
    }
    doc.blank();
    doc.line(vec![span(C::ExamplesToyHash.text(language), secondary)]);
    Entry::new(Kind::Result, doc)
}

/// Input names, shortened when a statement has many (a sudoku grid, a Merkle path).
fn names(list: Vec<String>, language: Language) -> String {
    match list.as_slice() {
        [first, second, .., last] if list.len() > 4 => fill(
            C::ExamplesMany.text(language),
            &[
                ("first", &format!("{first}, {second}")),
                ("last", last),
                ("count", &list.len().to_string()),
            ],
        ),
        _ => list.join(", "),
    }
}
