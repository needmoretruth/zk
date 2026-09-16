//! Running an activity to the end without a screen, for the command line.

use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;

use zk_core::Control;
use zk_i18n::Language;

use crate::activity::{Activity, CellRef, Event, JobEvent, Outbox};
use crate::app::{JOB_THREAD, panic_message};
use crate::doc::{Entry, Kind};
use crate::phrases::fill;
use crate::phrases::ui::Msg;

/// What an activity left behind: its cells as they ended, and every record it sent.
#[derive(Debug, Default)]
pub struct Collected {
    /// Cells in the order they were opened, each in its final state.
    pub entries: Vec<Entry>,
    /// Machine-readable records in the order they were sent.
    pub records: Vec<serde_json::Value>,
}

impl Collected {
    /// Whether any cell is an error, so a script can tell from the exit code.
    pub fn failed(&self) -> bool {
        self.entries.iter().any(|entry| entry.kind == Kind::Error)
    }
}

/// Runs `activity` on a worker thread until it finishes, calling `status` with every text the
/// working line would show, and returns its cells and records.
pub fn collect(
    activity: Box<dyn Activity>,
    control: &Control,
    language: Language,
    mut status: impl FnMut(&str),
) -> Collected {
    let (sender, receiver) = mpsc::channel::<JobEvent>();
    let outbox = Outbox::new(0, sender);
    let worker_control = control.clone();
    let spawned = thread::Builder::new().name(JOB_THREAD.to_string()).spawn(move || {
        let outcome =
            panic::catch_unwind(AssertUnwindSafe(|| activity.run(&outbox, &worker_control)));
        if let Err(payload) = outcome {
            outbox.crashed(panic_message(payload.as_ref()));
        }
        outbox.done();
    });
    let crashed =
        |why: &str| Entry::text(Kind::Error, fill(Msg::Crashed.text(language), &[("why", why)]));
    let mut collected = Collected::default();
    if let Err(error) = spawned {
        collected.entries.push(crashed(&error.to_string()));
        return collected;
    }
    let mut cells: Vec<(Option<CellRef>, Entry)> = Vec::new();
    for JobEvent { event, .. } in receiver {
        let find = |cells: &mut Vec<(Option<CellRef>, Entry)>, cell: CellRef| {
            cells.iter_mut().position(|(origin, _)| *origin == Some(cell))
        };
        match event {
            Event::Open(cell, entry) => cells.push((Some(cell), entry)),
            Event::Replace(cell, entry) => {
                if let Some(index) = find(&mut cells, cell) {
                    cells[index].1 = entry;
                }
            }
            Event::Append(cell, beat) => {
                if let Some(index) = find(&mut cells, cell) {
                    cells[index].1.doc.append(beat);
                }
            }
            Event::Record(value) => collected.records.push(value),
            Event::Status(text) => status(&text),
            Event::Crashed(why) => cells.push((None, crashed(&why))),
            Event::Subject(_) | Event::Stage(_) => {}
            Event::Done => break,
        }
    }
    collected.entries = cells.into_iter().map(|(_, entry)| entry).collect();
    collected
}
