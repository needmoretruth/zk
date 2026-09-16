//! Measuring, wrapping and cutting styled text by terminal columns, not by bytes or characters.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::doc::Tone;

/// Text in one tone, already resolved for the terminal's look.
pub(crate) type Piece = (String, Tone);
/// One screen row.
pub(crate) type Row = Vec<Piece>;

/// Columns `text` occupies.
pub(crate) fn width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Columns a row occupies.
pub(crate) fn row_width(row: &[Piece]) -> usize {
    row.iter().map(|(text, _)| width(text)).sum()
}

/// Wraps pieces into rows of at most `max` columns, breaking at spaces and inside words only when
/// a word is wider than a whole row. `\n` always starts a new row.
pub(crate) fn wrap(pieces: &[Piece], max: usize) -> Vec<Row> {
    let mut wrapper = Wrapper { max: max.max(1), ..Wrapper::default() };
    // Word pieces from adjacent spans with nothing between them form one word.
    let mut word: Vec<(&str, Tone)> = Vec::new();
    for (text, tone) in pieces {
        for token in tokens(text) {
            if let Token::Word(part) = token {
                word.push((part, *tone));
                continue;
            }
            if !word.is_empty() {
                wrapper.word(&std::mem::take(&mut word));
            }
            match token {
                Token::Break => wrapper.hard_break(),
                Token::Space(space) => wrapper.space(space, *tone),
                Token::Word(_) => {}
            }
        }
    }
    if !word.is_empty() {
        wrapper.word(&word);
    }
    wrapper.rows.push(wrapper.row);
    wrapper.rows
}

/// Cuts a row to `max` columns, ending with `ellipsis` when anything was cut.
pub(crate) fn truncate(row: Row, max: usize, ellipsis: &str) -> Row {
    if row_width(&row) <= max {
        return row;
    }
    let ellipsis = if width(ellipsis) <= max { ellipsis } else { "" };
    let budget = max - width(ellipsis);
    let mut out = Row::new();
    let mut used = 0;
    let mut last_tone = Tone::BODY;
    for (text, tone) in row {
        let mut kept = String::new();
        let mut full = false;
        for character in text.chars() {
            let columns = character.width().unwrap_or(0);
            if used + columns > budget {
                full = true;
                break;
            }
            kept.push(character);
            used += columns;
        }
        last_tone = tone;
        if !kept.is_empty() {
            out.push((kept, tone));
        }
        if full {
            break;
        }
    }
    if !ellipsis.is_empty() {
        out.push((ellipsis.to_string(), last_tone));
    }
    out
}

/// Pads a row with spaces to `columns`.
pub(crate) fn pad(row: &mut Row, columns: usize) {
    let used = row_width(row);
    if used < columns {
        row.push((" ".repeat(columns - used), Tone::BODY));
    }
}

/// Drops trailing spaces, so rows copied out of the terminal carry no padding.
pub(crate) fn trim_end(row: &mut Row) {
    while let Some((text, _)) = row.last_mut() {
        let trimmed = text.trim_end_matches(' ').len();
        if trimmed == 0 {
            row.pop();
        } else {
            text.truncate(trimmed);
            break;
        }
    }
}

enum Token<'a> {
    Break,
    Space(&'a str),
    Word(&'a str),
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space: Option<bool> = None;
    for (index, character) in text.char_indices() {
        let space = character == ' ';
        if character == '\n' || in_space.is_some_and(|previous| previous != space) {
            push_token(&mut out, &text[start..index], in_space);
            start = index;
            in_space = None;
        }
        if character == '\n' {
            out.push(Token::Break);
            start = index + 1;
            continue;
        }
        in_space = Some(space);
    }
    push_token(&mut out, &text[start..], in_space);
    out
}

fn push_token<'a>(out: &mut Vec<Token<'a>>, slice: &'a str, in_space: Option<bool>) {
    match in_space {
        Some(true) if !slice.is_empty() => out.push(Token::Space(slice)),
        Some(false) if !slice.is_empty() => out.push(Token::Word(slice)),
        _ => {}
    }
}

#[derive(Default)]
struct Wrapper {
    rows: Vec<Row>,
    row: Row,
    used: usize,
    max: usize,
    pending: Option<Piece>,
    /// Whether the current row began because the previous one was full.
    soft: bool,
}

impl Wrapper {
    fn hard_break(&mut self) {
        self.pending = None;
        self.rows.push(std::mem::take(&mut self.row));
        self.used = 0;
        self.soft = false;
    }

    fn soft_break(&mut self) {
        self.pending = None;
        self.rows.push(std::mem::take(&mut self.row));
        self.used = 0;
        self.soft = true;
    }

    fn space(&mut self, space: &str, tone: Tone) {
        if self.row.is_empty() && self.pending.is_none() {
            // Indentation at the start of a line is kept; the space a wrap landed on is not.
            if !self.soft {
                self.word(&[(space, tone)]);
            }
            return;
        }
        self.pending = Some((space.to_string(), tone));
    }

    /// Places one word, which may be several pieces in different tones with no space between
    /// them (such as a backtick and the code it opens); the pieces stay together.
    fn word(&mut self, pieces: &[(&str, Tone)]) {
        let columns: usize = pieces.iter().map(|(text, _)| width(text)).sum();
        let spacing = self.pending.as_ref().map_or(0, |(space, _)| width(space));
        if self.used + spacing + columns <= self.max {
            self.flush_pending();
        } else if columns <= self.max {
            self.soft_break();
        } else {
            if self.used + spacing < self.max {
                self.flush_pending();
            } else {
                self.soft_break();
            }
            for (text, tone) in pieces {
                for character in text.chars() {
                    let wide = character.width().unwrap_or(0);
                    if self.used + wide > self.max {
                        self.soft_break();
                    }
                    self.push(character.encode_utf8(&mut [0; 4]), *tone);
                }
            }
            return;
        }
        for (text, tone) in pieces {
            self.push(text, *tone);
        }
    }

    fn flush_pending(&mut self) {
        if let Some((space, tone)) = self.pending.take() {
            self.push(&space, tone);
        }
    }

    fn push(&mut self, text: &str, tone: Tone) {
        self.used += width(text);
        match self.row.last_mut() {
            Some((last, last_tone)) if *last_tone == tone => last.push_str(text),
            _ => self.row.push((text.to_string(), tone)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|row| row.iter().map(|(text, _)| text.as_str()).collect()).collect()
    }

    fn piece(text: &str) -> Piece {
        (text.to_string(), Tone::BODY)
    }

    #[test]
    fn wraps_at_spaces_and_drops_the_space_it_breaks_on() {
        let rows = wrap(&[piece("the quick brown fox")], 10);
        assert_eq!(texts(&rows), ["the quick", "brown fox"]);
    }

    #[test]
    fn breaks_words_wider_than_a_row_and_honours_newlines() {
        let rows = wrap(&[piece("abcdefghij\n  xy")], 4);
        assert_eq!(texts(&rows), ["abcd", "efgh", "ij", "  xy"]);
    }

    #[test]
    fn adjacent_spans_without_a_space_wrap_as_one_word() {
        let code = Tone::BODY.bold();
        let rows = wrap(&[piece("crate `"), ("bellman".to_string(), code), piece("`.")], 10);
        assert_eq!(texts(&rows), ["crate", "`bellman`."]);
    }

    #[test]
    fn measures_wide_characters_by_columns() {
        let rows = wrap(&[piece("증명 증명")], 5);
        assert_eq!(texts(&rows), ["증명", "증명"]);
    }

    #[test]
    fn truncates_with_an_ellipsis_inside_the_limit() {
        let row = truncate(vec![piece("abcdef")], 4, "…");
        assert_eq!(texts(&[row]), ["abc…"]);
        let row = truncate(vec![piece("abc")], 4, "…");
        assert_eq!(texts(&[row]), ["abc"]);
    }
}
