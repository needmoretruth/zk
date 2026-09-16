//! Laid-out rows onto the terminal: the only place a tone becomes a ratatui style.

use ratatui::text::{Line, Span};

use crate::doc::Tone;
use crate::look::Look;
use crate::text::{Row, row_width, truncate};

/// A laid-out row as a ratatui line.
pub(crate) fn to_line(row: Row, look: &Look) -> Line<'static> {
    Line::from(
        row.into_iter()
            .filter(|(text, _)| !text.is_empty())
            .map(|(text, tone)| Span::styled(text, look.style(tone)))
            .collect::<Vec<_>>(),
    )
}

/// A row with `left` at the start and `right` flush with the end; `left` is cut to make room.
pub(crate) fn edge_line(left: Row, right: Row, columns: usize, look: &Look) -> Line<'static> {
    let right_width = row_width(&right);
    let room = if right_width > 0 { columns.saturating_sub(right_width + 1) } else { columns };
    let mut row = truncate(left, room, &look.text("…"));
    if right_width > 0 && right_width < columns {
        let gap = columns.saturating_sub(row_width(&row) + right_width);
        row.push((" ".repeat(gap), Tone::BODY));
        row.extend(right);
    }
    to_line(row, look)
}
