//! What each command says, as documents both the TUI and the plain-text commands draw.

mod catalog;
mod compare;
mod examples;
mod help;
mod run;
mod stage;
mod welcome;

pub use catalog::{Page, about, list, no_systems, unknown_system};
pub use compare::Comparison;
pub use examples::examples;
pub(crate) use help::{help, shortcuts};
pub use run::{report, running};
pub use stage::status;
pub use welcome::welcome;

use zk_core::ExampleId;
use zk_core::catalog::{
    Assumption, Implementation, Shelf, SystemMeta, TrustedSetup, ZeroKnowledge,
};
use zk_i18n::Language;

use crate::doc::{Hue, Span, Tone, span};
use crate::phrases::catalog::Msg as C;

/// A caution line: `⚠` and the text, both yellow.
pub(crate) fn caution(text: impl Into<String>) -> Vec<Span> {
    let tone = Tone::of(Hue::Caution);
    vec![span("⚠ ", tone), span(text, tone)]
}

/// The caution lines a system's metadata calls for: teaching implementation, not zero-knowledge,
/// trusted component.
pub(crate) fn cautions(meta: &SystemMeta, language: Language) -> Vec<Vec<Span>> {
    let mut lines = Vec::new();
    if matches!(meta.implementation, Implementation::Teaching { .. }) {
        lines.push(caution(C::CautionTeaching.text(language)));
    }
    match meta.zero_knowledge {
        ZeroKnowledge::No => lines.push(caution(C::CautionNotZk.text(language))),
        ZeroKnowledge::Optional => lines.push(caution(C::CautionOptionalZk.text(language))),
        ZeroKnowledge::Yes => {}
    }
    let trusted = meta.trusted_setup == TrustedSetup::TrustedComponent
        || meta.assumptions.contains(&Assumption::TrustedComponent);
    if trusted {
        lines.push(caution(C::CautionTrusted.text(language)));
    }
    lines
}

/// A shelf's display name.
pub(crate) fn shelf_name(shelf: Shelf, language: Language) -> &'static str {
    match shelf {
        Shelf::Zcash => C::ShelfZcash,
        Shelf::Aztec => C::ShelfAztec,
        Shelf::Polygon => C::ShelfPolygon,
        Shelf::Others => C::ShelfOthers,
        Shelf::Homemade => C::ShelfHomemade,
    }
    .text(language)
}

/// The one-line statement an example proves.
pub(crate) fn statement(example: ExampleId, language: Language) -> &'static str {
    match example {
        ExampleId::OnePlusOne => C::ExOnePlusOne,
        ExampleId::Password => C::ExPassword,
        ExampleId::Sudoku => C::ExSudoku,
        ExampleId::Age => C::ExAge,
        ExampleId::Membership => C::ExMembership,
        ExampleId::Factoring => C::ExFactoring,
        ExampleId::PoolSpend => C::ExPoolSpend,
        ExampleId::PoolSpendWithoutRangeChecks => C::ExPoolSpendNoRangeChecks,
        ExampleId::PoolSpendWithoutNullifierBinding => C::ExPoolSpendUnboundNullifier,
    }
    .text(language)
}
