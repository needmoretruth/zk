//! Keys, the mouse wheel and pasted text.

use std::time::Instant;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

use super::App;
use crate::commands;
use crate::phrases::ui::Msg;

/// Rows one wheel notch scrolls.
const WHEEL_ROWS: usize = 3;

impl App {
    /// Handles one terminal event.
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.key(key),
            Event::Mouse(mouse) => self.mouse(mouse),
            Event::Paste(text) => self.paste(&text),
            _ => {}
        }
    }

    /// Handles one key press.
    pub fn key(&mut self, key: KeyEvent) {
        if self.shortcuts {
            self.shortcuts = false;
            return;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if control && key.code == KeyCode::Char('c') {
            self.ctrl_c();
            return;
        }
        if let Some(pager) = &mut self.pager {
            if !pager.key(key) {
                self.pager = None;
            }
            return;
        }
        if control {
            self.control_key(key.code);
            return;
        }
        let shifted = key.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);
        let page = self.transcript.height().saturating_sub(2).max(1);
        match key.code {
            KeyCode::Enter if shifted => self.edit(|composer| composer.insert("\n")),
            KeyCode::Enter => self.enter(),
            KeyCode::Tab => self.tab(),
            KeyCode::Up => self.arrow(-1),
            KeyCode::Down => self.arrow(1),
            KeyCode::Esc => self.escape(),
            KeyCode::PageUp => self.transcript.scroll_up(page),
            KeyCode::PageDown => self.transcript.scroll_down(page),
            KeyCode::Left => self.composer.left(),
            KeyCode::Right => self.composer.right(),
            KeyCode::Home => self.composer.home(),
            KeyCode::End => self.composer.end(),
            KeyCode::Backspace => self.edit(|composer| composer.backspace()),
            KeyCode::Delete => self.edit(|composer| composer.delete()),
            KeyCode::Char('?') if self.composer.is_empty() => self.shortcuts = true,
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::ALT) => {
                self.edit(|composer| composer.insert(character.encode_utf8(&mut [0; 4])));
            }
            _ => {}
        }
    }

    fn control_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') => self.edit(|composer| composer.insert("\n")),
            KeyCode::Char('o') => match self.last_page.clone() {
                Some(page) => self.open_page(page),
                None => self.flash(Msg::FooterNothingToReopen.text(self.language)),
            },
            KeyCode::Char('l') => self.clear_requested = true,
            _ => {}
        }
    }

    fn mouse(&mut self, mouse: MouseEvent) {
        let rows = match mouse.kind {
            MouseEventKind::ScrollUp => -(WHEEL_ROWS as isize),
            MouseEventKind::ScrollDown => WHEEL_ROWS as isize,
            _ => return,
        };
        match &mut self.pager {
            Some(pager) => pager.scroll(rows),
            None if rows < 0 => self.transcript.scroll_up(WHEEL_ROWS),
            None => self.transcript.scroll_down(WHEEL_ROWS),
        }
    }

    fn paste(&mut self, text: &str) {
        if self.pager.is_none() {
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            self.edit(|composer| composer.insert(&text));
        }
    }

    /// An edit to the composer; the popup starts again from its first row.
    fn edit(&mut self, change: impl FnOnce(&mut crate::composer::Composer)) {
        change(&mut self.composer);
        self.popup.selected = 0;
    }

    /// ctrl+c: close the page, clear the line, or quit when pressed twice on an empty line.
    fn ctrl_c(&mut self) {
        if self.pager.take().is_some() {
            return;
        }
        if !self.composer.is_empty() {
            self.edit(|composer| composer.clear());
            self.quit_armed = None;
            return;
        }
        if self.quit_armed() {
            self.quit = true;
        } else {
            self.quit_armed = Some(Instant::now());
        }
    }

    /// Enter: with the popup open, take its row first; run the line once it is a whole command.
    fn enter(&mut self) {
        let candidates = self.candidates();
        let chosen = candidates.get(self.popup.selected.min(candidates.len().saturating_sub(1)));
        if let Some(candidate) = chosen
            && candidate.line != self.composer.text()
        {
            let line = candidate.line.clone();
            self.edit(|composer| composer.set(line.clone()));
            if commands::parse(&line).is_err() {
                return;
            }
        }
        self.submit();
    }

    fn tab(&mut self) {
        let candidates = self.candidates();
        if let Some(candidate) =
            candidates.get(self.popup.selected.min(candidates.len().saturating_sub(1)))
        {
            let line = candidate.line.clone();
            self.edit(|composer| composer.set(line));
        }
    }

    fn arrow(&mut self, step: isize) {
        let count = self.candidates().len();
        if count > 0 {
            let next = (self.popup.selected as isize + step).rem_euclid(count as isize);
            self.popup.selected = next as usize;
        } else if step < 0 {
            self.composer.up();
        } else {
            self.composer.down();
        }
    }

    /// Esc: close the popup; with no popup, ask a running task to stop.
    fn escape(&mut self) {
        if !self.candidates().is_empty() {
            self.popup.dismissed = Some(self.composer.text().to_string());
            return;
        }
        self.stop();
    }
}
