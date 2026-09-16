//! Starting activities on worker threads and folding what they report into the screen.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::thread;
use std::time::Instant;

use zk_core::{Control, Stage};

use super::App;
use crate::activity::{Activity, Event, JobEvent, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::fill;
use crate::phrases::ui::Msg;

/// Name of every worker thread, so the panic hook can tell a crashed run from a crashed screen.
pub(crate) const JOB_THREAD: &str = "nmtzk-job";

/// The activity that is running now.
#[derive(Debug)]
pub(super) struct Job {
    id: u64,
    pub(super) control: Control,
    pub(super) started: Instant,
    pub(super) subject: String,
    pub(super) stage: Option<Stage>,
    pub(super) status: Option<String>,
    pub(super) stopping: bool,
}

impl App {
    /// Starts `activity` on a worker thread. Only one runs at a time; while one runs this adds a
    /// warning cell and returns `false`.
    pub fn start(&mut self, activity: Box<dyn Activity>) -> bool {
        if self.job.is_some() {
            self.transcript.push(Entry::text(Kind::Warning, Msg::Busy.text(self.language)));
            return false;
        }
        self.next_job += 1;
        let id = self.next_job;
        let progress = self.sender.clone();
        let control = Control::with_progress(move |stage| {
            let _ = progress.send(JobEvent { job: id, event: Event::Stage(stage) });
        });
        let subject = activity.subject();
        let outbox = Outbox::new(id, self.sender.clone());
        let worker_control = control.clone();
        let spawned = thread::Builder::new().name(JOB_THREAD.to_string()).spawn(move || {
            let outcome =
                panic::catch_unwind(AssertUnwindSafe(|| activity.run(&outbox, &worker_control)));
            if let Err(payload) = outcome {
                outbox.crashed(panic_message(payload.as_ref()));
            }
            outbox.done();
        });
        if let Err(error) = spawned {
            let text = fill(Msg::Crashed.text(self.language), &[("why", &error.to_string())]);
            self.transcript.push(Entry::text(Kind::Error, text));
            return false;
        }
        self.job = Some(Job {
            id,
            control,
            started: Instant::now(),
            subject,
            stage: None,
            status: None,
            stopping: false,
        });
        true
    }

    /// Whether an activity is running.
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Asks the running activity, if any, to stop.
    pub fn stop(&mut self) {
        if let Some(job) = &mut self.job {
            job.control.cancel();
            job.stopping = true;
        }
    }

    /// Applies everything activities have reported since the last call. Call it between frames.
    pub fn pump(&mut self) {
        self.expire();
        while let Ok(JobEvent { job, event }) = self.receiver.try_recv() {
            match event {
                Event::Open(cell, entry) => self.transcript.push_from(cell, entry),
                Event::Replace(cell, entry) => self.transcript.replace(cell, entry),
                Event::Append(cell, beat) => self.transcript.append(cell, beat),
                Event::Crashed(why) => {
                    let text = fill(Msg::Crashed.text(self.language), &[("why", &why)]);
                    self.transcript.push(Entry::text(Kind::Error, text));
                }
                Event::Done if self.job.as_ref().is_some_and(|running| running.id == job) => {
                    self.job = None;
                }
                other => {
                    if let Some(running) = self.job.as_mut().filter(|running| running.id == job) {
                        running.note(other);
                    }
                }
            }
        }
    }
}

impl Job {
    /// Updates what the working line says.
    fn note(&mut self, event: Event) {
        match event {
            Event::Subject(subject) => {
                self.subject = subject;
                self.stage = None;
                self.status = None;
            }
            Event::Stage(stage) => {
                self.stage = Some(stage);
                self.status = None;
            }
            Event::Status(text) => self.status = Some(text),
            _ => {}
        }
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    payload.downcast_ref::<String>().cloned().unwrap_or_else(|| "panic".to_string())
}
