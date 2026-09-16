//! Work that runs on its own thread and talks to the transcript while it goes.
//!
//! A proof run is the first activity; the cave story, Trio rounds, the toy shielded pool and a
//! trusted-setup ceremony are the next ones. Each gets an [`Outbox`] to open cells, replace them,
//! append story beats to them and say what the working line should show, and a [`Control`] to
//! notice when the reader pressed Esc. The screen never blocks on an activity: it drains the
//! outbox between frames.

use std::cell::Cell;
use std::sync::mpsc::Sender;

use zk_core::{Control, Stage};

use crate::doc::{Doc, Entry};

/// A long piece of work the screen starts and watches.
pub trait Activity: Send + 'static {
    /// What the working line names before the activity says anything, such as a system's name.
    fn subject(&self) -> String;

    /// Does the work, reporting through `outbox`. Checks `control` between steps and returns early
    /// when a stop was asked for; it is cooperative, nobody kills the thread.
    fn run(self: Box<Self>, outbox: &Outbox, control: &Control);
}

/// A handle on a cell an activity opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellRef {
    job: u64,
    index: u32,
}

/// One message from an activity to the screen.
#[derive(Debug)]
pub(crate) enum Event {
    Subject(String),
    Stage(Stage),
    Status(String),
    Open(CellRef, Entry),
    Replace(CellRef, Entry),
    Append(CellRef, Doc),
    Record(serde_json::Value),
    Crashed(String),
    Done,
}

/// An event tagged with the job it belongs to, so late messages from a finished job are ignored.
#[derive(Debug)]
pub(crate) struct JobEvent {
    pub(crate) job: u64,
    pub(crate) event: Event,
}

/// Where an activity sends what happens. Sending never fails from the activity's side: if the
/// screen has gone away, messages are dropped.
#[derive(Debug)]
pub struct Outbox {
    job: u64,
    next: Cell<u32>,
    sender: Sender<JobEvent>,
}

impl Outbox {
    pub(crate) fn new(job: u64, sender: Sender<JobEvent>) -> Outbox {
        Outbox { job, next: Cell::new(0), sender }
    }

    fn send(&self, event: Event) {
        let _ = self.sender.send(JobEvent { job: self.job, event });
    }

    /// Names what the working line talks about from now on, such as the next system in `/run all`.
    pub fn subject(&self, subject: impl Into<String>) {
        self.send(Event::Subject(subject.into()));
    }

    /// Replaces the working line's text outright, for activities without harness stages.
    pub fn status(&self, text: impl Into<String>) {
        self.send(Event::Status(text.into()));
    }

    /// Adds a cell at the bottom of the transcript.
    pub fn open(&self, entry: Entry) -> CellRef {
        let index = self.next.get();
        self.next.set(index.wrapping_add(1));
        let cell = CellRef { job: self.job, index };
        self.send(Event::Open(cell, entry));
        cell
    }

    /// Replaces a cell's kind and content, such as a running cell becoming a result.
    pub fn replace(&self, cell: CellRef, entry: Entry) {
        self.send(Event::Replace(cell, entry));
    }

    /// Adds blocks to the end of a cell: one story beat.
    pub fn append(&self, cell: CellRef, beat: Doc) {
        self.send(Event::Append(cell, beat));
    }

    /// Hands over one machine-readable record of what just happened, such as one scene of the cave.
    /// The screen ignores records; `nmtzk --json` prints them as JSON lines.
    pub fn record(&self, value: serde_json::Value) {
        self.send(Event::Record(value));
    }

    pub(crate) fn crashed(&self, why: String) {
        self.send(Event::Crashed(why));
    }

    pub(crate) fn done(&self) {
        self.send(Event::Done);
    }
}
