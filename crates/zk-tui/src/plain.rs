//! The TUI's cells as plain text, for commands that print and exit.
//!
//! `nmtzk list` and `/list` must say the same thing, so both lay out the same documents; this
//! printer only swaps the terminal widget for text with optional ANSI colour codes.

use std::io::IsTerminal;

use crate::doc::{Entry, Hue, Tone};
use crate::layout::{doc_rows, entry_rows};
use crate::look::Look;
use crate::markdown;
use crate::text::Row;

/// How printed text looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Printer {
    width: usize,
    ansi: bool,
    look: Look,
}

impl Printer {
    /// A printer for standard output: the terminal's width and colour when it is a terminal and
    /// `NO_COLOR` is unset; unwrapped, uncoloured text when it is a pipe or a file.
    pub fn stdout(look: Look) -> Printer {
        let terminal = std::io::stdout().is_terminal();
        let width = if terminal {
            crossterm::terminal::size().map_or(100, |(columns, _)| usize::from(columns))
        } else {
            usize::MAX / 4
        };
        Printer { width, ansi: terminal && look.color, look }
    }

    /// A printer with a fixed width, for tests.
    pub fn new(width: usize, ansi: bool, look: Look) -> Printer {
        Printer { width, ansi, look }
    }

    /// A cell as text, ending with a newline.
    pub fn entry(&self, entry: &Entry) -> String {
        self.text(entry_rows(entry, self.width, &self.look, std::time::Duration::ZERO))
    }

    /// A markdown page as text, ending with a newline.
    pub fn page(&self, source: &str) -> String {
        self.text(doc_rows(&markdown::to_doc(source), self.width, &self.look))
    }

    fn text(&self, rows: Vec<Row>) -> String {
        let mut out = String::new();
        for mut row in rows {
            crate::text::trim_end(&mut row);
            for (text, tone) in row {
                match self.sgr(tone) {
                    Some(codes) => out.push_str(&format!("\x1b[{codes}m{text}\x1b[0m")),
                    None => out.push_str(&text),
                }
            }
            out.push('\n');
        }
        out
    }

    /// The SGR parameters for a tone, or `None` when nothing needs setting.
    fn sgr(&self, tone: Tone) -> Option<String> {
        if !self.ansi {
            return None;
        }
        let mut codes = Vec::new();
        if tone.bold {
            codes.push("1");
        }
        if tone.italic {
            codes.push("3");
        }
        if tone.underline {
            codes.push("4");
        }
        let color = match tone.hue {
            Hue::Body => None,
            Hue::Accent => Some("36"),
            Hue::Success => Some("32"),
            Hue::Failure => Some("31"),
            Hue::Caution => Some("33"),
            Hue::Shelf => Some("35"),
            Hue::Secondary => Some("90"),
        };
        if self.look.color {
            codes.extend(color);
        }
        (!codes.is_empty()).then(|| codes.join(";"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{Kind, span};

    #[test]
    fn colour_codes_appear_only_when_asked_for() {
        let mut entry = Entry::text(Kind::Result, "ok");
        entry.doc.line(vec![span("fine", Tone::of(Hue::Success))]);
        let coloured = Printer::new(40, true, Look::default()).entry(&entry);
        assert_eq!(coloured, "• ok\n  \x1b[32mfine\x1b[0m\n");
        let plain = Printer::new(40, false, Look::default()).entry(&entry);
        assert_eq!(plain, "• ok\n  fine\n");
    }
}
