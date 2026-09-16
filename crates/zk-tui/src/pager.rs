//! The full-screen page view for long documents such as a system's catalog page.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use zk_i18n::Language;

use crate::doc::{Doc, Hue, Tone};
use crate::layout::doc_rows;
use crate::look::Look;
use crate::markdown;
use crate::paint::{edge_line, to_line};
use crate::phrases::ui::Msg;
use crate::text::Row;

/// Widest a page's text runs, so lines stay readable on very wide terminals.
const MAX_TEXT_WIDTH: usize = 100;

/// An open page.
#[derive(Debug)]
pub(crate) struct Pager {
    title: String,
    doc: Doc,
    top: usize,
    laid: Option<(usize, Vec<Row>)>,
    body_height: usize,
}

impl Pager {
    pub(crate) fn new(title: &str, source: &str) -> Pager {
        Pager {
            title: title.to_string(),
            doc: markdown::to_doc(source),
            top: 0,
            laid: None,
            body_height: 0,
        }
    }

    /// Handles a key; returns `false` when the pager should close.
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        let page = self.body_height.saturating_sub(1).max(1);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Up | KeyCode::Char('k') => self.scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll(1),
            KeyCode::PageUp => self.scroll(-(page as isize)),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll(page as isize),
            KeyCode::Home | KeyCode::Char('g') => self.top = 0,
            KeyCode::End | KeyCode::Char('G') => self.top = usize::MAX,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,
            _ => {}
        }
        true
    }

    pub(crate) fn scroll(&mut self, rows: isize) {
        self.top = self.top.saturating_add_signed(rows);
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame, area: Rect, look: &Look, language: Language) {
        frame.render_widget(Clear, area);
        let columns = usize::from(area.width);
        let text_width = columns.saturating_sub(4).clamp(1, MAX_TEXT_WIDTH);
        if self.laid.as_ref().is_none_or(|(width, _)| *width != text_width) {
            self.laid = Some((text_width, doc_rows(&self.doc, text_width, look)));
        }
        let rows = self.laid.as_ref().map(|(_, rows)| rows.as_slice()).unwrap_or_default();
        self.body_height = usize::from(area.height).saturating_sub(2);
        let last_top = rows.len().saturating_sub(self.body_height);
        self.top = self.top.min(last_top);

        let secondary = Tone::of(Hue::Secondary);
        let close = Msg::PagerClose.text(language);
        let mut lines = vec![edge_line(
            vec![(look.text(&self.title).into_owned(), Tone::BODY.bold())],
            vec![(look.text(close).into_owned(), secondary)],
            columns,
            look,
        )];
        for row in rows.iter().skip(self.top).take(self.body_height) {
            let mut padded = vec![("  ".to_string(), Tone::BODY)];
            padded.extend(row.iter().cloned());
            lines.push(to_line(padded, look));
        }
        while lines.len() < usize::from(area.height).saturating_sub(1) {
            lines.push(Line::default());
        }
        let percent = (self.top * 100).checked_div(last_top).unwrap_or(100);
        lines.push(edge_line(
            vec![(look.text(Msg::PagerKeys.text(language)).into_owned(), secondary)],
            vec![(format!("{percent}%"), secondary)],
            columns,
            look,
        ));
        frame.render_widget(Paragraph::new(lines), area);
    }
}
