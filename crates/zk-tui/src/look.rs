//! Which colours and glyphs this terminal gets, decided once at start-up.
//!
//! The program never picks RGB values: it names one of the 16 ANSI colours and lets the reader's
//! terminal theme decide what that looks like. Consoles that cannot draw box or braille glyphs
//! (`TERM=linux`, or `--ascii`) get ASCII stand-ins, swapped in before text is measured so that
//! wrapping and alignment stay right.

use std::borrow::Cow;
use std::time::Duration;

use ratatui::style::{Color, Modifier, Style};

use crate::doc::{Hue, Tone};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_ASCII: [&str; 4] = ["|", "/", "-", "\\"];
const FRAME_MILLIS: u128 = 80;

/// Colour and glyph choices for one terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    /// Whether colour may be used at all; `NO_COLOR` turns it off.
    pub color: bool,
    /// Whether symbols are replaced by ASCII stand-ins.
    pub ascii: bool,
}

impl Default for Look {
    fn default() -> Look {
        Look { color: true, ascii: false }
    }
}

impl Look {
    /// Reads `NO_COLOR` and `TERM`; `ascii` is the `--ascii` flag.
    pub fn detect(ascii: bool) -> Look {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        let linux_console = std::env::var("TERM").is_ok_and(|term| term == "linux");
        Look { color: !no_color, ascii: ascii || linux_console }
    }

    /// The spinner frame for a task that has been running for `elapsed`.
    pub fn spinner(&self, elapsed: Duration) -> &'static str {
        let tick = elapsed.as_millis() / FRAME_MILLIS;
        if self.ascii { SPINNER_ASCII[(tick % 4) as usize] } else { SPINNER[(tick % 10) as usize] }
    }

    /// `text` with symbols replaced when this look is ASCII-only; letters in any script are kept.
    pub fn text<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if !self.ascii || text.is_ascii() {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len());
        for character in text.chars() {
            match ascii_stand_in(character) {
                Some(stand_in) => out.push_str(stand_in),
                None => out.push(character),
            }
        }
        Cow::Owned(out)
    }

    /// The named colour for a role, or `None` for the default foreground or when colour is off.
    pub fn color(&self, hue: Hue) -> Option<Color> {
        if !self.color {
            return None;
        }
        match hue {
            Hue::Body => None,
            Hue::Accent => Some(Color::Cyan),
            Hue::Success => Some(Color::Green),
            Hue::Failure => Some(Color::Red),
            Hue::Caution => Some(Color::Yellow),
            Hue::Shelf => Some(Color::Magenta),
            Hue::Secondary => Some(Color::DarkGray),
        }
    }

    /// The terminal style for a tone. Backgrounds are never set.
    pub fn style(&self, tone: Tone) -> Style {
        let mut style = Style::default();
        if let Some(color) = self.color(tone.hue) {
            style = style.fg(color);
        }
        if tone.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if tone.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if tone.underline {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        style
    }
}

/// The ASCII stand-in for a symbol this program draws, or `None` to keep the character.
fn ascii_stand_in(character: char) -> Option<&'static str> {
    Some(match character {
        '›' => ">",
        '•' => "*",
        '└' => "`",
        '├' => "+",
        '✗' => "x",
        '⚠' => "!",
        '✓' => "v",
        '↓' => "v",
        '↑' => "^",
        '·' => "-",
        '…' => "...",
        '─' => "-",
        '│' => "|",
        '┃' => "#",
        '┬' => "+",
        '∞' => "inf",
        '╭' | '╮' | '╰' | '╯' => "+",
        'µ' => "u",
        '—' | '–' => "-",
        '×' => "x",
        '→' => "->",
        '‘' | '’' => "'",
        '“' | '”' => "\"",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_look_replaces_symbols_but_keeps_letters() {
        let look = Look { color: true, ascii: true };
        assert_eq!(look.text("› • └ ├ ✗ ⚠ 12 µs"), "> * ` + x ! 12 us");
        assert_eq!(look.text("증명"), "증명");
        assert_eq!(Look::default().text("›"), "›");
    }

    #[test]
    fn spinner_advances_every_80_ms() {
        let look = Look::default();
        assert_eq!(look.spinner(Duration::from_millis(0)), "⠋");
        assert_eq!(look.spinner(Duration::from_millis(80)), "⠙");
        assert_eq!(look.spinner(Duration::from_millis(800)), "⠋");
        let ascii = Look { color: true, ascii: true };
        assert_eq!(ascii.spinner(Duration::from_millis(240)), "\\");
    }

    #[test]
    fn without_colour_only_attributes_remain() {
        let look = Look { color: false, ascii: false };
        let style = look.style(Tone::of(Hue::Failure).bold());
        assert_eq!(style.fg, None);
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(style.bg, None);
    }
}
