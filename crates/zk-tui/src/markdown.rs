//! Catalog pages from markdown into a document the pager and the plain printer can lay out.
//!
//! Only what the museum's pages use is drawn with care: headings, emphasis, lists, code, tables,
//! quotes and links. HTML is skipped rather than shown as tags.

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::doc::{Align, Block, Doc, Hue, Span, Table, Tone, span};

/// The document for a markdown page.
pub(crate) fn to_doc(markdown: &str) -> Doc {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, options) {
        builder.event(event);
    }
    builder.flush();
    while builder.doc.blocks.last() == Some(&Block::Blank) {
        builder.doc.blocks.pop();
    }
    builder.doc
}

#[derive(Default)]
struct Builder {
    doc: Doc,
    inline: Vec<Span>,
    heading: Option<HeadingLevel>,
    strong: usize,
    emphasis: usize,
    struck: usize,
    link: Option<(String, usize)>,
    quote: usize,
    lists: Vec<Option<u64>>,
    items: Vec<Item>,
    code: Option<String>,
    table: Option<TableState>,
}

struct Item {
    marker: String,
    fresh: bool,
}

#[derive(Default)]
struct TableState {
    table: Table,
    row: Vec<Vec<Span>>,
    cell: Vec<Span>,
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => match &mut self.code {
                Some(code) => code.push_str(&text),
                None => self.push(text.to_string()),
            },
            Event::Code(code) => {
                let tick = Tone::of(Hue::Secondary);
                let tone = self.tone();
                self.target().extend([
                    span("`", tick),
                    span(code.to_string(), tone),
                    span("`", tick),
                ]);
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => self.push(math.to_string()),
            Event::SoftBreak => self.push(" ".to_string()),
            Event::HardBreak => self.push("\n".to_string()),
            Event::Rule => {
                self.flush();
                self.doc.push(Block::Rule);
                self.blank();
            }
            Event::TaskListMarker(done) => {
                self.push(if done { "[x] " } else { "[ ] " }.to_string())
            }
            Event::FootnoteReference(label) => self.push(format!("[{label}]")),
            Event::Html(_) | Event::InlineHtml(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush();
                if !self.doc.blocks.is_empty() {
                    self.blank();
                }
                self.heading = Some(level);
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.quote += 1;
            }
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push(start);
            }
            Tag::Item => self.start_item(),
            Tag::Table(alignments) => {
                self.flush();
                let align = alignments
                    .iter()
                    .map(|a| if *a == Alignment::Right { Align::Right } else { Align::Left });
                let table = Table { align: align.collect(), wrap: true, ..Table::default() };
                self.table = Some(TableState { table, ..TableState::default() });
            }
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strong => self.strong += 1,
            Tag::Strikethrough => self.struck += 1,
            Tag::Link { dest_url, .. } => {
                let start = self.target().len();
                self.link = Some((dest_url.to_string(), start));
            }
            Tag::Image { .. } => self.push("[".to_string()),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Heading(_) => {
                self.flush();
                self.heading = None;
                self.blank();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
                self.blank();
            }
            TagEnd::CodeBlock => self.end_code(),
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Item => {
                self.flush();
                if self.items.last().is_some_and(|item| item.fresh) {
                    let (lead, _) = self.prefixes();
                    self.doc.push(Block::Text { lead, hang: None, body: Vec::new() });
                }
                self.items.pop();
            }
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Strikethrough => self.struck = self.struck.saturating_sub(1),
            TagEnd::Link => self.end_link(),
            TagEnd::Image => self.push("]".to_string()),
            other => self.end_table_part(other),
        }
    }

    fn end_table_part(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::TableCell => self.end_cell(),
            TagEnd::TableHead => {
                if let Some(state) = &mut self.table {
                    state.table.header = std::mem::take(&mut state.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(state) = &mut self.table {
                    let row = std::mem::take(&mut state.row);
                    state.table.rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(state) = self.table.take() {
                    self.doc.push(Block::Table(state.table));
                    self.blank();
                }
            }
            _ => {}
        }
    }

    fn start_item(&mut self) {
        self.flush();
        let marker = match self.lists.last_mut() {
            Some(Some(number)) => {
                let marker = format!("{number}. ");
                *number += 1;
                marker
            }
            _ => "• ".to_string(),
        };
        self.items.push(Item { marker, fresh: true });
    }

    fn end_code(&mut self) {
        let Some(code) = self.code.take() else { return };
        let (_, hang) = self.prefixes();
        let lines = code
            .trim_end_matches('\n')
            .split('\n')
            .map(|line| {
                let mut row = hang.clone();
                row.push(span(format!("    {}", line.replace('\t', "    ")), Tone::BODY));
                row
            })
            .collect();
        self.doc.push(Block::Pre(lines));
        if self.lists.is_empty() {
            self.blank();
        }
    }

    fn end_link(&mut self) {
        let Some((url, start)) = self.link.take() else { return };
        let text: String = self.target().iter().skip(start).map(|s| s.text.as_str()).collect();
        if url.starts_with("http") && text != url {
            self.target().push(span(format!(" ({url})"), Tone::of(Hue::Secondary)));
        }
    }

    fn end_cell(&mut self) {
        if let Some(state) = &mut self.table {
            let cell = std::mem::take(&mut state.cell);
            state.row.push(cell);
        }
    }

    /// Where inline text goes: the open table cell, or the running paragraph.
    fn target(&mut self) -> &mut Vec<Span> {
        match &mut self.table {
            Some(state) => &mut state.cell,
            None => &mut self.inline,
        }
    }

    fn push(&mut self, text: String) {
        let tone = self.tone();
        self.target().push(span(text, tone));
    }

    fn tone(&self) -> Tone {
        let mut tone = Tone::BODY;
        match self.heading {
            Some(HeadingLevel::H1) => tone = tone.bold().underline(),
            Some(HeadingLevel::H2) => tone = tone.bold(),
            Some(_) => tone = tone.bold().italic(),
            None => {}
        }
        if self.strong > 0 {
            tone = tone.bold();
        }
        if self.emphasis > 0 {
            tone = tone.italic();
        }
        if self.struck > 0 {
            tone.hue = Hue::Secondary;
        }
        if self.link.is_some() {
            tone = Tone { hue: Hue::Accent, ..tone.underline() };
        }
        tone
    }

    /// First-row and continuation prefixes for the current quote and list nesting.
    fn prefixes(&mut self) -> (Vec<Span>, Vec<Span>) {
        let bars = "│ ".repeat(self.quote);
        let outer: usize =
            self.items.iter().rev().skip(1).map(|item| crate::text::width(&item.marker)).sum();
        let mut lead =
            vec![span(bars.clone(), Tone::of(Hue::Secondary)), span(" ".repeat(outer), Tone::BODY)];
        let mut hang = lead.clone();
        if let Some(item) = self.items.last_mut() {
            let marker_width = crate::text::width(&item.marker);
            let marker = if item.fresh { item.marker.clone() } else { " ".repeat(marker_width) };
            item.fresh = false;
            lead.push(span(marker, Tone::of(Hue::Secondary)));
            hang.push(span(" ".repeat(marker_width), Tone::BODY));
        }
        lead.retain(|s| !s.text.is_empty());
        hang.retain(|s| !s.text.is_empty());
        (lead, hang)
    }

    /// One empty row between blocks, never two in a row and never at the top.
    fn blank(&mut self) {
        if !matches!(self.doc.blocks.last(), None | Some(Block::Blank)) {
            self.doc.blank();
        }
    }

    /// Ends the running paragraph as a wrapped text block.
    fn flush(&mut self) {
        if self.inline.is_empty() {
            return;
        }
        let body = std::mem::take(&mut self.inline);
        let (lead, hang) = self.prefixes();
        self.doc.push(Block::Text { lead, hang: Some(hang), body });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::doc_rows;
    use crate::look::Look;

    fn render(markdown: &str, width: usize) -> Vec<String> {
        doc_rows(&to_doc(markdown), width, &Look::default())
            .iter()
            .map(|row| row.iter().map(|(text, _)| text.as_str()).collect())
            .collect()
    }

    #[test]
    fn headings_are_bold_and_paragraphs_wrap() {
        let doc = to_doc("# Groth16\n\nA short proof of knowledge.");
        let rows = doc_rows(&doc, 12, &Look::default());
        assert!(rows[0][0].1.bold && rows[0][0].1.underline);
        assert_eq!(
            render("# Groth16\n\nA short proof of knowledge.", 12),
            ["Groth16", "", "A short", "proof of", "knowledge."]
        );
    }

    #[test]
    fn lists_nest_and_hang() {
        let rows = render("- one two three\n- four\n  1. five\n", 12);
        assert_eq!(rows, ["• one two", "  three", "• four", "  1. five"]);
    }

    #[test]
    fn links_show_their_address_and_code_is_kept_verbatim() {
        let rows =
            render("[paper](https://eprint.iacr.org/2016/260)\n\n```\nfn main() {}\n```", 80);
        assert_eq!(rows[0], "paper (https://eprint.iacr.org/2016/260)");
        assert_eq!(rows[2], "    fn main() {}");
    }

    #[test]
    fn a_quote_is_followed_by_one_empty_row() {
        assert_eq!(render("> quoted\n\nafter", 40), ["│ quoted", "", "after"]);
    }

    #[test]
    fn tables_keep_their_columns() {
        let rows = render("| a | b |\n|---|--:|\n| x | 12 |\n", 40);
        assert_eq!(rows, ["a   b", "x  12"]);
    }
}
