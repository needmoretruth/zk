//! Painting the screen: transcript, working line, composer, footer, and the popup and overlays.

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Modifier;
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};

use super::{App, MIN_HEIGHT, MIN_WIDTH};
use crate::composer::Visual;
use crate::doc::{Hue, Tone};
use crate::format;
use crate::paint::{edge_line, to_line};
use crate::phrases::ui::Msg;
use crate::text::{Row, pad, truncate, width};
use crate::views;

/// Most rows the popup shows.
const POPUP_ROWS: usize = 8;
/// Most text rows inside the composer's border (8 rows with the border).
const COMPOSER_ROWS: usize = 6;
/// Columns before the typed text inside the border: a space and the prompt glyph with its space.
const COMPOSER_LEAD: u16 = 3;

const ASCII_BORDER: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

impl App {
    /// Draws the whole screen into `frame`.
    pub fn draw(&mut self, frame: &mut Frame) {
        self.expire();
        let area = frame.area();
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            self.draw_too_small(frame, area);
            return;
        }
        if let Some(pager) = &mut self.pager {
            pager.draw(frame, area, &self.look, self.language);
            return;
        }
        let text_width = usize::from(area.width.saturating_sub(2 + COMPOSER_LEAD + 1));
        let visual = self.composer.visual(text_width);
        let composer_rows = visual.rows.len().clamp(1, COMPOSER_ROWS) as u16;
        let footer = Rect { y: area.bottom() - 1, height: 1, ..area };
        let composer = Rect { y: footer.y - composer_rows - 2, height: composer_rows + 2, ..area };
        let working_height = u16::from(self.job.is_some());
        let working = Rect { y: composer.y - working_height, height: working_height, ..area };
        let transcript = Rect { height: working.y - area.y, ..area };

        let rows = self.transcript.window(
            usize::from(transcript.width),
            usize::from(transcript.height),
            &self.look,
        );
        let lines: Vec<Line> = rows.into_iter().map(|row| to_line(row, &self.look)).collect();
        frame.render_widget(Paragraph::new(lines), transcript);
        if working_height > 0 {
            self.draw_working(frame, working);
        }
        self.draw_composer(frame, composer, &visual);
        self.draw_footer(frame, footer);
        self.draw_popup(frame, composer, area);
        if self.shortcuts {
            self.draw_shortcuts(frame, area);
        }
    }

    fn draw_too_small(&self, frame: &mut Frame, area: Rect) {
        let text = self.look.text(Msg::TooSmall.text(self.language)).into_owned();
        let row = truncate(vec![(text, Tone::BODY)], usize::from(area.width), "");
        let line = Rect { y: area.y + area.height / 2, height: 1.min(area.height), ..area };
        frame.render_widget(Paragraph::new(to_line(row, &self.look)), line);
    }

    fn draw_working(&self, frame: &mut Frame, area: Rect) {
        let Some(job) = &self.job else { return };
        let secondary = Tone::of(Hue::Secondary);
        let status = job
            .status
            .clone()
            .unwrap_or_else(|| views::status(job.stage, &job.subject, self.language));
        let tail = if job.stopping { Msg::WorkingStopping } else { Msg::WorkingStop };
        let row: Row = vec![
            (self.look.spinner(job.started.elapsed()).to_string(), Tone::of(Hue::Accent)),
            (" ".to_string(), Tone::BODY),
            (self.look.text(&status).into_owned(), Tone::BODY),
            (self.look.text(" · ").into_owned(), secondary),
            (format::elapsed(job.started.elapsed()), secondary),
            (self.look.text(&format!(" · {}", tail.text(self.language))).into_owned(), secondary),
        ];
        let ellipsis = self.look.text("…");
        frame.render_widget(
            Paragraph::new(to_line(truncate(row, usize::from(area.width), &ellipsis), &self.look)),
            area,
        );
    }

    fn draw_composer(&self, frame: &mut Frame, area: Rect, visual: &Visual) {
        let set = if self.look.ascii { ASCII_BORDER } else { border::ROUNDED };
        let block = Block::bordered()
            .border_set(set)
            .border_style(self.look.style(Tone::of(Hue::Secondary)));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let rows = usize::from(inner.height);
        let first = visual.cursor.0.saturating_sub(rows.saturating_sub(1));
        let prompt = (self.look.text("› ").into_owned(), Tone::of(Hue::Accent));
        let mut lines = Vec::new();
        if self.composer.is_empty() {
            let hint = self.look.text(Msg::Placeholder.text(self.language)).into_owned();
            let row = vec![(" ".to_string(), Tone::BODY), prompt, (hint, Tone::of(Hue::Secondary))];
            lines.push(to_line(truncate(row, usize::from(inner.width), ""), &self.look));
        } else {
            for (index, text) in visual.rows.iter().enumerate().skip(first).take(rows) {
                let lead = if index == 0 { prompt.clone() } else { ("  ".to_string(), Tone::BODY) };
                let row = vec![(" ".to_string(), Tone::BODY), lead, (text.clone(), Tone::BODY)];
                lines.push(to_line(row, &self.look));
            }
        }
        frame.render_widget(Paragraph::new(lines), inner);
        if !self.shortcuts {
            let x = inner.x + COMPOSER_LEAD + visual.cursor.1 as u16;
            let y = inner.y + (visual.cursor.0 - first) as u16;
            frame.set_cursor_position(Position { x: x.min(inner.right().saturating_sub(1)), y });
        }
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let secondary = Tone::of(Hue::Secondary);
        let left = if self.quit_armed() {
            (Msg::FooterQuitAgain.text(self.language).to_string(), Tone::BODY)
        } else if let Some((text, _)) = &self.flash {
            (text.clone(), Tone::BODY)
        } else {
            (Msg::FooterHints.text(self.language).to_string(), secondary)
        };
        let left =
            vec![(" ".to_string(), Tone::BODY), (self.look.text(&left.0).into_owned(), left.1)];
        let right = if self.transcript.unseen() {
            let text = format!("{} ", Msg::FooterNewOutput.text(self.language));
            vec![(self.look.text(&text).into_owned(), Tone::of(Hue::Accent))]
        } else {
            Row::new()
        };
        frame.render_widget(
            Paragraph::new(edge_line(left, right, usize::from(area.width), &self.look)),
            area,
        );
    }

    fn draw_popup(&self, frame: &mut Frame, composer: Rect, area: Rect) {
        let candidates = self.candidates();
        let room = usize::from(composer.y - area.y);
        let shown = candidates.len().min(POPUP_ROWS).min(room);
        if shown == 0 {
            return;
        }
        let selected = self.popup.selected.min(candidates.len() - 1);
        let first = selected.saturating_sub(shown - 1);
        let visible = &candidates[first..first + shown];
        let rect = Rect { y: composer.y - shown as u16, height: shown as u16, ..area };
        frame.render_widget(Clear, rect);
        let columns = usize::from(area.width);
        let label_width =
            visible.iter().map(|c| width(&self.look.text(&c.label))).max().unwrap_or(0) + 2;
        let lines: Vec<Line> = visible
            .iter()
            .enumerate()
            .map(|(offset, candidate)| {
                let chosen = first + offset == selected;
                let mut label = vec![
                    (" ".to_string(), Tone::BODY),
                    (self.look.text(&candidate.label).into_owned(), Tone::BODY),
                ];
                pad(&mut label, label_width + 1);
                label.push((
                    self.look.text(&candidate.detail).into_owned(),
                    Tone::of(Hue::Secondary),
                ));
                let mut row = truncate(label, columns, &self.look.text("…"));
                if chosen {
                    pad(&mut row, columns);
                    for (_, tone) in &mut row {
                        tone.hue = Hue::Accent;
                    }
                }
                let line = to_line(row, &self.look);
                if chosen {
                    line.patch_style(
                        ratatui::style::Style::default().add_modifier(Modifier::REVERSED),
                    )
                } else {
                    line
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), rect);
    }

    fn draw_shortcuts(&self, frame: &mut Frame, area: Rect) {
        let pairs = views::shortcuts(self.language);
        let key_width =
            pairs.iter().map(|(keys, _)| width(&self.look.text(keys))).max().unwrap_or(0);
        let mut rows: Vec<Row> = vec![
            vec![(Msg::ShortcutsTitle.text(self.language).to_string(), Tone::BODY.bold())],
            Row::new(),
        ];
        for (keys, words) in pairs {
            let mut row = vec![(self.look.text(keys).into_owned(), Tone::of(Hue::Accent))];
            pad(&mut row, key_width + 2);
            row.push((self.look.text(words).into_owned(), Tone::BODY));
            rows.push(row);
        }
        rows.push(Row::new());
        rows.push(vec![(
            Msg::ShortcutsClose.text(self.language).to_string(),
            Tone::of(Hue::Secondary),
        )]);
        let widest = rows.iter().map(|row| crate::text::row_width(row)).max().unwrap_or(0);
        let box_width = (widest + 4).min(usize::from(area.width)) as u16;
        let box_height = (rows.len() + 2).min(usize::from(area.height)) as u16;
        let rect = Rect {
            x: area.x + (area.width - box_width) / 2,
            y: area.y + (area.height - box_height) / 2,
            width: box_width,
            height: box_height,
        };
        frame.render_widget(Clear, rect);
        let set = if self.look.ascii { ASCII_BORDER } else { border::ROUNDED };
        let block = Block::bordered()
            .border_set(set)
            .border_style(self.look.style(Tone::of(Hue::Secondary)));
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        let ellipsis = self.look.text("…");
        let lines: Vec<Line> = rows
            .into_iter()
            .map(|row| {
                let mut row = truncate(row, usize::from(inner.width.saturating_sub(1)), &ellipsis);
                row.insert(0, (" ".to_string(), Tone::BODY));
                to_line(row, &self.look)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }
}
