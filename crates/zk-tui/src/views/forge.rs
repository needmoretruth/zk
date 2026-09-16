//! What the BCTV14 forgery shows: each step with the ordinary verifier's answer.

use zk_i18n::Language;

use crate::doc::{Doc, Hue, Tone, plain, span};
use crate::phrases::fill;
use crate::phrases::forge::Msg as M;

/// The top of the story cell: title, the teaching caution and what the flaw is.
pub(crate) fn opening(language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(vec![span(M::Title.text(language), Tone::BODY.bold())]);
    for line in super::cautions(&sys_bctv14::META, language) {
        doc.line(line);
    }
    doc.line(vec![plain(M::Intro.text(language))]);
    doc
}

/// A line led by `✓` when the outcome is safe, `✗` in red when it is a break.
fn marked(safe: bool, text: &str) -> Doc {
    let (glyph, tone) =
        if safe { ("✓ ", Tone::of(Hue::Success)) } else { ("✗ ", Tone::of(Hue::Failure)) };
    let body = if safe { Tone::BODY } else { Tone::of(Hue::Failure) };
    let mut doc = Doc::new();
    doc.led(vec![span(glyph, tone)], vec![span(text, body)]);
    doc
}

/// Where a stop left the forgery.
pub(crate) fn stopped(done: usize, total: usize, language: Language) -> Doc {
    let text = fill(
        M::Stopped.text(language),
        &[("done", &done.to_string()), ("total", &total.to_string())],
    );
    let mut doc = Doc::new();
    doc.line(super::caution(text));
    doc
}

/// The flawed keys and how many extra points they publish.
pub(crate) fn keys(extra: usize, language: Language) -> Doc {
    let mut doc = Doc::new();
    doc.line(super::caution(fill(M::Keys.text(language), &[("count", &extra.to_string())])));
    doc
}

/// The honest proof of the true claim.
pub(crate) fn honest(accepted: bool, language: Language) -> Doc {
    marked(accepted, if accepted { M::Honest } else { M::HonestRejected }.text(language))
}

/// The honest proof offered, unchanged, for the false claim.
pub(crate) fn honest_for_false(accepted: bool, language: Language) -> Doc {
    let msg = if accepted { M::HonestForFalseAccepted } else { M::HonestForFalse };
    marked(!accepted, msg.text(language))
}

/// The proof rewritten with the extra points: accepting it is the break being shown.
pub(crate) fn rewritten(accepted: bool, language: Language) -> Doc {
    marked(false, if accepted { M::Rewritten } else { M::RewrittenRejected }.text(language))
}

/// The rewrite against corrected keys, and the lesson.
pub(crate) fn corrected(accepted: bool, language: Language) -> Doc {
    let msg = if accepted { M::CorrectedAccepted } else { M::Corrected };
    let mut doc = marked(!accepted, msg.text(language));
    doc.line(vec![span(M::Lesson.text(language), Tone::BODY.bold())]);
    doc
}
