//! The program's own standard output and standard error, kept apart from what upstream crates print.
//!
//! Proof-system crates the museum runs print on their own: ark-marlin writes `PC::Check failed` to
//! standard error for every proof it rejects, and Remainder writes a warning to standard output when
//! it proves a false claim and a panic message for every tampered proof it rejects. Every run proves
//! a false claim and verifies tampered proofs on purpose, so those lines would land on top of the
//! full-screen program and inside the JSON lines a script reads. Filtering crate by crate would miss
//! the next upstream, so [`start`] keeps private duplicates of descriptors 1 and 2 for this program
//! and points the descriptors themselves at `/dev/null`. Everything `nmtzk` says goes through [`out`]
//! and [`err`]; whatever a dependency writes to descriptor 1 or 2 goes nowhere.
//!
//! Setting `NMTZK_UPSTREAM_OUTPUT=1` leaves descriptors 1 and 2 alone, for anyone debugging an
//! exhibit who wants to see what its upstream prints.

use std::backtrace::{Backtrace, BacktraceStatus};
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::os::fd::AsFd;
use std::panic::{self, PanicHookInfo};
use std::sync::{Mutex, OnceLock, PoisonError};

/// The environment variable that, set to `1`, keeps upstream prints visible.
const UPSTREAM_OUTPUT: &str = "NMTZK_UPSTREAM_OUTPUT";

/// The exit code of a Rust program whose main thread panicked, kept for panics `main` catches.
pub(crate) const PANICKED: u8 = 101;

static SAVED: OnceLock<Saved> = OnceLock::new();

/// The latest panic message while descriptor 2 is silenced. Systems catch upstream panics as
/// rejections, so a message is printed only once a panic reaches `main` ([`report_panic`]).
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// The duplicates taken at start-up. `None` where a descriptor could not be duplicated (it was
/// closed): that stream then falls back to the standard one, which is what it was before.
struct Saved {
    out: Option<File>,
    err: Option<File>,
    /// Held open for the life of the process: when descriptor 1 or 2 was closed at start-up,
    /// `/dev/null` may have been opened on that very number.
    _sink: Option<File>,
}

/// Duplicates standard output and standard error, then points descriptors 1 and 2 at `/dev/null`
/// unless `NMTZK_UPSTREAM_OUTPUT=1`.
///
/// Called once after the command line is parsed (clap prints help and usage errors on the standard
/// streams itself) and before any system runs. A descriptor is redirected only when its duplicate
/// was saved, so a failure here costs visibility of upstream prints, never the program's own output.
pub(crate) fn start() {
    SAVED.get_or_init(|| {
        let out = io::stdout().as_fd().try_clone_to_owned().ok().map(File::from);
        let err = io::stderr().as_fd().try_clone_to_owned().ok().map(File::from);
        if std::env::var_os(UPSTREAM_OUTPUT).is_some_and(|value| value == "1") {
            return Saved { out, err, _sink: None };
        }
        let sink = File::options().write(true).open("/dev/null").ok();
        if let Some(sink) = &sink {
            if out.is_some() {
                let _ = io::stdout().flush();
                let _ = rustix::stdio::dup2_stdout(sink);
            }
            if err.is_some() && rustix::stdio::dup2_stderr(sink).is_ok() {
                panic::set_hook(Box::new(remember_panic));
            }
        }
        Saved { out, err, _sink: sink }
    });
}

/// One of the program's own streams.
pub(crate) enum Stream {
    /// A duplicate saved by [`start`].
    Saved(&'static File),
    /// Standard output itself, when there is no duplicate.
    Stdout(io::Stdout),
    /// Standard error itself, when there is no duplicate.
    Stderr(io::Stderr),
}

/// The program's standard output.
pub(crate) fn out() -> Stream {
    match SAVED.get().and_then(|saved| saved.out.as_ref()) {
        Some(file) => Stream::Saved(file),
        None => Stream::Stdout(io::stdout()),
    }
}

/// The program's standard error.
pub(crate) fn err() -> Stream {
    match SAVED.get().and_then(|saved| saved.err.as_ref()) {
        Some(file) => Stream::Saved(file),
        None => Stream::Stderr(io::stderr()),
    }
}

/// Writes `text` to the program's standard error. Failures are dropped: there is nowhere left to
/// report them.
pub(crate) fn write_err(text: &str) {
    let mut err = err();
    let _ = err.write_all(text.as_bytes()).and_then(|()| err.flush());
}

/// A handle on the program's standard output for the full-screen program to draw on.
pub(crate) fn screen() -> io::Result<File> {
    match SAVED.get().and_then(|saved| saved.out.as_ref()) {
        Some(file) => file.try_clone(),
        None => io::stdout().as_fd().try_clone_to_owned().map(File::from),
    }
}

impl Stream {
    /// Whether the stream is a terminal. Asked of the duplicate, because descriptors 1 and 2 point
    /// at `/dev/null` once [`start`] has run.
    pub(crate) fn is_terminal(&self) -> bool {
        match self {
            Stream::Saved(file) => file.is_terminal(),
            Stream::Stdout(stdout) => stdout.is_terminal(),
            Stream::Stderr(stderr) => stderr.is_terminal(),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Saved(file) => file.write(buf),
            Stream::Stdout(stdout) => stdout.write(buf),
            Stream::Stderr(stderr) => stderr.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Saved(file) => file.flush(),
            Stream::Stdout(stdout) => stdout.flush(),
            Stream::Stderr(stderr) => stderr.flush(),
        }
    }
}

/// Prints the latest panic message on the program's standard error, if one was kept.
pub(crate) fn report_panic() {
    let message = LAST_PANIC.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(message) = message {
        write_err(&message);
    }
}

/// The panic hook while descriptor 2 is silenced: keeps the message the default hook would print.
fn remember_panic(info: &PanicHookInfo<'_>) {
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("<unnamed>");
    let location = info.location().map(|location| format!(" at {location}")).unwrap_or_default();
    let payload = info.payload_as_str().unwrap_or("Box<dyn Any>");
    let mut message = format!("thread '{name}' panicked{location}:\n{payload}\n");
    let backtrace = Backtrace::capture();
    if backtrace.status() == BacktraceStatus::Captured {
        message.push_str(&format!("stack backtrace:\n{backtrace}\n"));
    } else {
        message.push_str(
            "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n",
        );
    }
    *LAST_PANIC.lock().unwrap_or_else(PoisonError::into_inner) = Some(message);
}
