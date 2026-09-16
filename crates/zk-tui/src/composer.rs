//! The line being typed: editing, a cursor that respects multi-byte text, and command history.

use unicode_width::UnicodeWidthChar;

/// The composer's text and history.
#[derive(Debug, Default)]
pub(crate) struct Composer {
    text: String,
    /// Byte offset, always on a character boundary.
    cursor: usize,
    history: Vec<String>,
    /// Index into `history` while recalling earlier commands.
    recall: Option<usize>,
    /// What was typed before recalling started, restored after the newest entry.
    draft: String,
}

/// The composer laid out in rows of a fixed width, with the cursor's row and column.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Visual {
    pub(crate) rows: Vec<String>,
    pub(crate) cursor: (usize, usize),
}

impl Composer {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Whether the text on screen is a recalled command the reader has not edited.
    pub(crate) fn recalling(&self) -> bool {
        self.recall.is_some()
    }

    pub(crate) fn insert(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.recall = None;
    }

    pub(crate) fn backspace(&mut self) {
        if let Some((start, _)) = self.text[..self.cursor].char_indices().next_back() {
            self.text.replace_range(start..self.cursor, "");
            self.cursor = start;
            self.recall = None;
        }
    }

    pub(crate) fn delete(&mut self) {
        if let Some(character) = self.text[self.cursor..].chars().next() {
            self.text.replace_range(self.cursor..self.cursor + character.len_utf8(), "");
            self.recall = None;
        }
    }

    pub(crate) fn left(&mut self) {
        if let Some((start, _)) = self.text[..self.cursor].char_indices().next_back() {
            self.cursor = start;
        }
    }

    pub(crate) fn right(&mut self) {
        if let Some(character) = self.text[self.cursor..].chars().next() {
            self.cursor += character.len_utf8();
        }
    }

    /// Start of the current line.
    pub(crate) fn home(&mut self) {
        self.cursor = self.text[..self.cursor].rfind('\n').map_or(0, |newline| newline + 1);
    }

    /// End of the current line.
    pub(crate) fn end(&mut self) {
        self.cursor += self.text[self.cursor..].find('\n').unwrap_or(self.text.len() - self.cursor);
    }

    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.recall = None;
    }

    /// Replaces the text and puts the cursor at the end.
    pub(crate) fn set(&mut self, text: String) {
        self.cursor = text.len();
        self.text = text;
        self.recall = None;
    }

    /// Takes the text for running and remembers it, unless it repeats the previous command.
    pub(crate) fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.text);
        self.cursor = 0;
        self.recall = None;
        self.draft.clear();
        if !text.trim().is_empty() && self.history.last() != Some(&text) {
            self.history.push(text.clone());
        }
        text
    }

    /// ↑: the previous line of a multi-line text, otherwise the previous command.
    pub(crate) fn up(&mut self) {
        if self.text[..self.cursor].contains('\n') {
            self.move_line(-1);
            return;
        }
        let index = match self.recall {
            None if self.history.is_empty() => return,
            None => {
                self.draft = self.text.clone();
                self.history.len() - 1
            }
            Some(0) => return,
            Some(index) => index - 1,
        };
        self.show_history(index);
    }

    /// ↓: the next line of a multi-line text, otherwise the next command or back to the draft.
    pub(crate) fn down(&mut self) {
        if self.text[self.cursor..].contains('\n') {
            self.move_line(1);
            return;
        }
        match self.recall {
            None => {}
            Some(index) if index + 1 < self.history.len() => self.show_history(index + 1),
            Some(_) => {
                let draft = std::mem::take(&mut self.draft);
                self.set(draft);
            }
        }
    }

    fn show_history(&mut self, index: usize) {
        self.text = self.history[index].clone();
        self.cursor = self.text.len();
        self.recall = Some(index);
    }

    fn move_line(&mut self, direction: isize) {
        let line_start = self.text[..self.cursor].rfind('\n').map_or(0, |n| n + 1);
        let column = self.text[line_start..self.cursor].chars().count();
        let target_start = if direction < 0 {
            let previous_end = line_start.saturating_sub(1);
            self.text[..previous_end].rfind('\n').map_or(0, |n| n + 1)
        } else {
            match self.text[self.cursor..].find('\n') {
                Some(offset) => self.cursor + offset + 1,
                None => return,
            }
        };
        let line = self.text[target_start..].split('\n').next().unwrap_or_default();
        let offset: usize = line.chars().take(column).map(char::len_utf8).sum();
        self.cursor = target_start + offset;
    }

    /// Lays the text out in rows of `width` columns, wrapping long lines by character.
    pub(crate) fn visual(&self, width: usize) -> Visual {
        let width = width.max(1);
        let mut rows = vec![String::new()];
        let mut used = 0;
        let mut cursor = (0, 0);
        for (index, character) in self.text.char_indices() {
            if index == self.cursor {
                cursor = (rows.len() - 1, used);
            }
            if character == '\n' {
                rows.push(String::new());
                used = 0;
                continue;
            }
            let columns = character.width().unwrap_or(0);
            if used + columns > width {
                rows.push(String::new());
                used = 0;
                if index == self.cursor {
                    cursor = (rows.len() - 1, 0);
                }
            }
            if let Some(row) = rows.last_mut() {
                row.push(character);
            }
            used += columns;
        }
        if self.cursor == self.text.len() {
            cursor = (rows.len() - 1, used);
            if used >= width {
                rows.push(String::new());
                cursor = (rows.len() - 1, 0);
            }
        }
        Visual { rows, cursor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_respect_multi_byte_characters() {
        let mut composer = Composer::default();
        composer.insert("증명x");
        composer.left();
        composer.backspace();
        assert_eq!(composer.text(), "증x");
        composer.home();
        composer.delete();
        assert_eq!(composer.text(), "x");
    }

    #[test]
    fn history_walks_back_and_returns_to_the_draft() {
        let mut composer = Composer::default();
        for line in ["/list", "/examples"] {
            composer.insert(line);
            composer.take();
        }
        composer.insert("/he");
        composer.up();
        assert_eq!(composer.text(), "/examples");
        composer.up();
        assert_eq!(composer.text(), "/list");
        composer.up();
        assert_eq!(composer.text(), "/list");
        composer.down();
        composer.down();
        assert_eq!(composer.text(), "/he");
        assert!(!composer.recalling());
    }

    #[test]
    fn visual_rows_wrap_and_place_the_cursor() {
        let mut composer = Composer::default();
        composer.insert("abcdef\ngh");
        assert_eq!(
            composer.visual(4),
            Visual { rows: vec!["abcd".into(), "ef".into(), "gh".into()], cursor: (2, 2) }
        );
        composer.set("abcd".into());
        assert_eq!(composer.visual(4).cursor, (1, 0));
    }

    #[test]
    fn up_and_down_move_between_lines_before_history() {
        let mut composer = Composer::default();
        composer.insert("/one");
        composer.take();
        composer.insert("ab\ncd");
        composer.up();
        assert_eq!(composer.text(), "ab\ncd");
        composer.insert("X");
        assert_eq!(composer.text(), "abX\ncd");
    }
}
