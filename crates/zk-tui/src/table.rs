//! Borderless tables: two spaces between columns, a bold header, numbers on the right.

use crate::doc::{Align, Table, Tone};
use crate::layout::resolve;
use crate::look::Look;
use crate::text::{Row, pad, row_width, trim_end, truncate, wrap};

const GAP: usize = 2;

/// Lays `table` out in at most `max` columns. Columns shrink widest-first when the table does not
/// fit; data tables cut cells with `…`, prose tables wrap them.
pub(crate) fn rows(table: &Table, max: usize, look: &Look) -> Vec<Row> {
    let header: Vec<Row> = table
        .header
        .iter()
        .map(|cell| {
            let mut row = resolve(cell, look);
            for (_, tone) in &mut row {
                *tone = tone.bold();
            }
            row
        })
        .collect();
    let body: Vec<Vec<Row>> =
        table.rows.iter().map(|cells| cells.iter().map(|c| resolve(c, look)).collect()).collect();
    let columns = body.iter().map(Vec::len).chain([header.len(), table.align.len()]).max();
    let Some(columns) = columns.filter(|count| *count > 0) else { return Vec::new() };

    let mut natural = vec![0; columns];
    for cells in body.iter().chain(std::iter::once(&header)) {
        for (index, cell) in cells.iter().enumerate() {
            natural[index] = natural[index].max(row_width(cell));
        }
    }
    let visible = visible_columns(&natural, &table.optional, max);
    let natural: Vec<usize> = visible.iter().map(|column| natural[*column]).collect();
    let budget = max.saturating_sub(GAP * (visible.len() - 1));
    let widths = fit(&natural, budget, table.wrap);
    let layout =
        Columns { visible: &visible, widths: &widths, align: &table.align, wrap: table.wrap };

    let mut out = Vec::new();
    if !header.is_empty() {
        out.extend(join(&header, &layout, look));
    }
    for cells in &body {
        out.extend(join(cells, &layout, look));
    }
    out
}

struct Columns<'a> {
    visible: &'a [usize],
    widths: &'a [usize],
    align: &'a [Align],
    wrap: bool,
}

/// Every column index, minus optional columns dropped in order until the rest fits.
fn visible_columns(natural: &[usize], optional: &[usize], max: usize) -> Vec<usize> {
    let mut visible: Vec<usize> = (0..natural.len()).collect();
    let used = |visible: &[usize]| {
        visible.iter().map(|column| natural[*column]).sum::<usize>()
            + GAP * visible.len().saturating_sub(1)
    };
    for column in optional {
        if used(&visible) <= max || visible.len() <= 1 {
            break;
        }
        visible.retain(|kept| kept != column);
    }
    visible
}

/// Column widths that fit `budget`, taking one column from the widest column at a time.
fn fit(natural: &[usize], budget: usize, wrap: bool) -> Vec<usize> {
    let floor = if wrap { 8 } else { 3 };
    let mut widths = natural.to_vec();
    let mut total: usize = widths.iter().sum();
    while total > budget {
        let widest = widths
            .iter()
            .enumerate()
            .filter(|(index, width)| **width > floor.min(natural[*index]))
            .max_by_key(|(_, width)| **width)
            .map(|(index, _)| index);
        let Some(index) = widest else { break };
        widths[index] -= 1;
        total -= 1;
    }
    widths
}

/// One table row, which becomes several screen rows when a prose cell wraps.
fn join(cells: &[Row], layout: &Columns<'_>, look: &Look) -> Vec<Row> {
    let ellipsis = look.text("…");
    let laid: Vec<Vec<Row>> = layout
        .visible
        .iter()
        .zip(layout.widths)
        .map(|(column, width)| {
            let cell = cells.get(*column).cloned().unwrap_or_default();
            if layout.wrap { wrap(&cell, *width) } else { vec![truncate(cell, *width, &ellipsis)] }
        })
        .collect();
    let height = laid.iter().map(Vec::len).max().unwrap_or(1);
    (0..height)
        .map(|line| {
            let mut row = Row::new();
            for (index, (column, width)) in layout.visible.iter().zip(layout.widths).enumerate() {
                if index > 0 {
                    row.push((" ".repeat(GAP), Tone::BODY));
                }
                let mut cell = laid[index].get(line).cloned().unwrap_or_default();
                let used = row_width(&cell);
                if layout.align.get(*column) == Some(&Align::Right) && used < *width {
                    row.push((" ".repeat(width - used), Tone::BODY));
                    row.append(&mut cell);
                } else {
                    pad(&mut cell, *width);
                    row.append(&mut cell);
                }
            }
            trim_end(&mut row);
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::plain;

    fn render(table: &Table, max: usize) -> Vec<String> {
        rows(table, max, &Look::default())
            .iter()
            .map(|row| row.iter().map(|(text, _)| text.as_str()).collect())
            .collect()
    }

    fn sample(wrap: bool) -> Table {
        Table {
            header: vec![vec![plain("System")], vec![plain("Prove")]],
            align: vec![Align::Left, Align::Right],
            rows: vec![
                vec![vec![plain("groth16")], vec![plain("88.1 ms")]],
                vec![vec![plain("halo2")], vec![plain("1.24 s")]],
            ],
            wrap,
            optional: vec![0],
        }
    }

    #[test]
    fn columns_are_two_spaces_apart_and_numbers_align_right() {
        assert_eq!(
            render(&sample(false), 80),
            ["System     Prove", "groth16  88.1 ms", "halo2     1.24 s"]
        );
    }

    #[test]
    fn the_header_is_bold() {
        let laid = rows(&sample(false), 80, &Look::default());
        assert!(laid[0][0].1.bold);
        assert!(!laid[1][0].1.bold);
    }

    #[test]
    fn optional_columns_go_first_when_the_table_is_too_wide() {
        assert_eq!(render(&sample(false), 7), ["  Prove", "88.1 ms", " 1.24 s"]);
    }

    #[test]
    fn a_narrow_data_table_cuts_the_widest_column() {
        let table = Table { optional: Vec::new(), ..sample(false) };
        let laid = render(&table, 14);
        assert!(laid.iter().all(|row| crate::text::width(row) <= 14), "{laid:?}");
        assert!(laid[1].contains('…'));
    }
}
