//! The list of cells above the composer, and where the reader has scrolled to.
//!
//! Following the bottom is the normal state: new cells appear and stay in view. Scrolling up stops
//! following, so reading is never yanked away; new output then only raises a flag the footer shows.

use std::time::Instant;

use crate::activity::CellRef;
use crate::doc::{Doc, Entry, Kind};
use crate::layout::entry_rows;
use crate::look::Look;
use crate::text::Row;

#[derive(Debug)]
struct Cell {
    origin: Option<CellRef>,
    entry: Entry,
    started: Instant,
}

/// Cells and scroll position.
#[derive(Debug, Default)]
pub(crate) struct Transcript {
    cells: Vec<Cell>,
    /// First visible row while scrolled up; `None` while following the bottom.
    top: Option<usize>,
    unseen: bool,
    total: usize,
    height: usize,
}

impl Transcript {
    pub(crate) fn push(&mut self, entry: Entry) {
        self.add(None, entry);
    }

    pub(crate) fn push_from(&mut self, origin: CellRef, entry: Entry) {
        self.add(Some(origin), entry);
    }

    fn add(&mut self, origin: Option<CellRef>, entry: Entry) {
        self.cells.push(Cell { origin, entry, started: Instant::now() });
        self.changed();
    }

    /// Replaces an activity's cell; a cell cleared away in the meantime is simply gone.
    pub(crate) fn replace(&mut self, origin: CellRef, entry: Entry) {
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.origin == Some(origin)) {
            if entry.kind == Kind::Running && cell.entry.kind != Kind::Running {
                cell.started = Instant::now();
            }
            cell.entry = entry;
            self.changed();
        }
    }

    /// Adds a story beat to the end of an activity's cell.
    pub(crate) fn append(&mut self, origin: CellRef, beat: Doc) {
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.origin == Some(origin)) {
            cell.entry.doc.append(beat);
            self.changed();
        }
    }

    pub(crate) fn clear(&mut self) {
        self.cells.clear();
        self.follow();
    }

    fn changed(&mut self) {
        if self.top.is_some() {
            self.unseen = true;
        }
    }

    pub(crate) fn follow(&mut self) {
        self.top = None;
        self.unseen = false;
    }

    pub(crate) fn unseen(&self) -> bool {
        self.unseen
    }

    /// Whether any cell shows a spinner, so the screen keeps animating.
    pub(crate) fn animating(&self) -> bool {
        self.cells.iter().any(|cell| cell.entry.kind == Kind::Running)
    }

    /// The last height drawn, for page-sized scrolling.
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn scroll_up(&mut self, rows: usize) {
        let bottom_top = self.total.saturating_sub(self.height);
        let top = self.top.unwrap_or(bottom_top);
        if bottom_top > 0 {
            self.top = Some(top.saturating_sub(rows));
        }
    }

    pub(crate) fn scroll_down(&mut self, rows: usize) {
        let Some(top) = self.top else { return };
        let top = top + rows;
        if top + self.height >= self.total {
            self.follow();
        } else {
            self.top = Some(top);
        }
    }

    /// The rows visible in a window of `width` × `height`, with one blank row between cells.
    pub(crate) fn window(&mut self, width: usize, height: usize, look: &Look) -> Vec<Row> {
        let now = Instant::now();
        let mut rows = Vec::new();
        for (index, cell) in self.cells.iter().enumerate() {
            if index > 0 {
                rows.push(Row::new());
            }
            rows.extend(entry_rows(&cell.entry, width, look, now.duration_since(cell.started)));
        }
        self.total = rows.len();
        self.height = height;
        let bottom_top = self.total.saturating_sub(height);
        let top = match self.top {
            Some(top) if top < bottom_top => top,
            Some(_) => {
                self.follow();
                bottom_top
            }
            None => bottom_top,
        };
        rows.into_iter().skip(top).take(height).collect()
    }
}
