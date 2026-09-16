//! Turns documents and cells into screen rows at a given width.

use std::time::Duration;

use crate::doc::{Block, Doc, Entry, Hue, Kind, Span, Tone};
use crate::look::Look;
use crate::table;
use crate::text::{Row, pad, row_width, truncate, width, wrap};

/// Resolves spans for the look: ASCII stand-ins are swapped in here, before anything is measured.
pub(crate) fn resolve(spans: &[Span], look: &Look) -> Row {
    spans.iter().map(|span| (look.text(&span.text).into_owned(), span.tone)).collect()
}

/// Every row of `doc` at `max` columns.
pub(crate) fn doc_rows(doc: &Doc, max: usize, look: &Look) -> Vec<Row> {
    let mut rows = Vec::new();
    for block in &doc.blocks {
        match block {
            Block::Text { lead, hang, body } => {
                rows.extend(text_rows(lead, hang.as_deref(), body, max, look));
            }
            Block::Pre(lines) => {
                let ellipsis = look.text("…");
                rows.extend(lines.iter().map(|line| truncate(resolve(line, look), max, &ellipsis)));
            }
            Block::Table(table) => rows.extend(table::rows(table, max, look)),
            Block::Boxed(lines) => rows.extend(boxed_rows(lines, max, look)),
            Block::Rule => {
                let rule = look.text("─").repeat(max.min(100));
                rows.push(vec![(rule, Tone::of(Hue::Secondary))]);
            }
            Block::Blank => rows.push(Row::new()),
        }
    }
    rows
}

fn text_rows(
    lead: &[Span],
    hang: Option<&[Span]>,
    body: &[Span],
    max: usize,
    look: &Look,
) -> Vec<Row> {
    let lead = resolve(lead, look);
    let lead_width = row_width(&lead);
    let hang = match hang {
        Some(hang) => resolve(hang, look),
        None if lead_width > 0 => vec![(" ".repeat(lead_width), Tone::BODY)],
        None => Row::new(),
    };
    let room = max.saturating_sub(lead_width.max(row_width(&hang))).max(1);
    wrap(&resolve(body, look), room)
        .into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            let mut out = if index == 0 { lead.clone() } else { hang.clone() };
            out.append(&mut row);
            out
        })
        .collect()
}

fn boxed_rows(lines: &[Vec<Span>], max: usize, look: &Look) -> Vec<Row> {
    let border = Tone::of(Hue::Secondary);
    let content: Vec<Row> = lines.iter().map(|line| resolve(line, look)).collect();
    let natural = content.iter().map(|row| row_width(row)).max().unwrap_or(0);
    let inner = natural.min(max.saturating_sub(4));
    let ellipsis = look.text("…");
    let horizontal = look.text("─").repeat(inner + 2);
    let edge = |left: &str, right: &str| {
        vec![(format!("{}{horizontal}{}", look.text(left), look.text(right)), border)]
    };
    let mut rows = vec![edge("╭", "╮")];
    for row in content {
        let mut out = vec![(format!("{} ", look.text("│")), border)];
        let mut cut = truncate(row, inner, &ellipsis);
        pad(&mut cut, inner);
        out.append(&mut cut);
        out.push((format!(" {}", look.text("│")), border));
        rows.push(out);
    }
    rows.push(edge("╰", "╯"));
    rows
}

/// The head glyph of a cell kind and its tone; `None` for cells drawn without one.
fn head(kind: Kind, look: &Look, running_for: Duration) -> Option<Piece> {
    let (glyph, hue) = match kind {
        Kind::Welcome => return None,
        Kind::Command => ("›", Hue::Accent),
        Kind::Result | Kind::Story => ("•", Hue::Body),
        Kind::Running => (look.spinner(running_for), Hue::Accent),
        Kind::Warning => ("⚠", Hue::Caution),
        Kind::Error => ("✗", Hue::Failure),
    };
    Some((look.text(glyph).into_owned(), Tone::of(hue)))
}

type Piece = (String, Tone);

/// Every row of a cell: the head glyph and a space before the first row, two spaces before the rest.
pub(crate) fn entry_rows(
    entry: &Entry,
    max: usize,
    look: &Look,
    running_for: Duration,
) -> Vec<Row> {
    let Some(head) = head(entry.kind, look, running_for) else {
        return doc_rows(&entry.doc, max, look);
    };
    let indent = width(&head.0) + 1;
    let mut rows = doc_rows(&entry.doc, max.saturating_sub(indent).max(1), look);
    if rows.is_empty() {
        rows.push(Row::new());
    }
    rows.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            let mut out = if index == 0 {
                vec![head.clone(), (" ".to_string(), Tone::BODY)]
            } else {
                vec![(" ".repeat(indent), Tone::BODY)]
            };
            out.append(&mut row);
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::plain;

    fn texts(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|row| row.iter().map(|(text, _)| text.as_str()).collect()).collect()
    }

    #[test]
    fn tree_items_hang_under_their_text() {
        let mut doc = Doc::new();
        doc.tree(vec![vec![plain("one two three")], vec![plain("four")]]);
        let rows = doc_rows(&doc, 9, &Look::default());
        assert_eq!(texts(&rows), ["├ one two", "  three", "└ four"]);
    }

    #[test]
    fn result_cells_start_with_a_bullet_and_indent_the_rest() {
        let mut doc = Doc::new();
        doc.line(vec![plain("first")]);
        doc.line(vec![plain("second")]);
        let rows = entry_rows(&Entry::new(Kind::Result, doc), 40, &Look::default(), Duration::ZERO);
        assert_eq!(texts(&rows), ["• first", "  second"]);
    }

    #[test]
    fn boxes_are_rounded_and_fit_the_width() {
        let mut doc = Doc::new();
        doc.push(Block::Boxed(vec![vec![plain("nmtzk")], vec![plain("a longer line of text")]]));
        let rows = texts(&doc_rows(&doc, 16, &Look::default()));
        assert_eq!(rows[0], "╭──────────────╮");
        assert_eq!(rows[1], "│ nmtzk        │");
        assert_eq!(rows[2], "│ a longer li… │");
        let ascii = texts(&doc_rows(&doc, 16, &Look { color: true, ascii: true }));
        assert_eq!(ascii[0], "+--------------+");
    }
}
