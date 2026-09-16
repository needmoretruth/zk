//! The hands-on exhibits: Ali Baba's cave, Trio's rounds, the Toy Shielded Pool, the trusted-setup
//! ceremony and the BCTV14 forgery.
//!
//! Each one is an [`crate::Activity`]: it runs the real engine, turns what happened into documents
//! through the views, and sends them through the [`crate::Outbox`] one beat at a time, together
//! with a machine-readable record of each beat. The TUI waits between beats and stops on Esc; the
//! command line runs the same activity without waiting through [`collect`] and prints the cells it
//! ends with, or the records as JSON lines.

pub mod cave;
pub mod ceremony;
mod collect;
pub mod forge;
pub mod pool;
mod request;
pub mod trio;

use std::cell::Cell;
use std::thread;
use std::time::{Duration, Instant};

use zk_core::Control;
use zk_i18n::Language;

use crate::activity::{CellRef, Outbox};
use crate::doc::Doc;

pub use collect::{Collected, collect};
pub use request::{Request, request, reset_declined, reset_question};

/// Time between two scenes of the cave on screen.
pub(crate) const CAVE_BEAT: Duration = Duration::from_millis(250);
/// Time between two rounds of Trio on screen.
pub(crate) const TRIO_BEAT: Duration = Duration::from_millis(120);
/// Time between two turns of the ceremony, and between the steps of the forgery, on screen.
pub(crate) const CEREMONY_BEAT: Duration = Duration::from_millis(400);
/// How often a waiting activity looks for Esc.
const WAKE: Duration = Duration::from_millis(10);

/// Whether an activity waits between its beats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pace {
    /// One beat at a time, at the pace the design sets for the exhibit: the TUI.
    #[default]
    Story,
    /// Every beat at once: the command line, scripts and tests.
    Instant,
}

/// Waits one beat at `pace`. Returns `false` when a stop was asked for, before or during the wait.
pub(crate) fn pause(control: &Control, pace: Pace, beat: Duration) -> bool {
    if pace == Pace::Story {
        let until = Instant::now() + beat;
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            if control.is_cancelled() || left.is_zero() {
                break;
            }
            thread::sleep(left.min(WAKE));
        }
    }
    !control.is_cancelled()
}

/// A story told in numbered steps, for the activities whose beats are not scenes or rounds: each
/// step waits a beat, names itself on the working line, then appends its lines and its record.
pub(crate) struct Steps<'a> {
    pub(crate) outbox: &'a Outbox,
    pub(crate) control: &'a Control,
    pub(crate) pace: Pace,
    pub(crate) beat: Duration,
    pub(crate) cell: CellRef,
    pub(crate) language: Language,
    /// Steps planned, for saying where a stop came.
    pub(crate) total: usize,
    /// What to append when a stop comes after `done` of `total` steps.
    pub(crate) stopped: fn(usize, usize, Language) -> Doc,
    pub(crate) done: Cell<usize>,
    /// Set when an outcome differs from what the design says must happen.
    pub(crate) unexpected: Cell<bool>,
}

impl<'a> Steps<'a> {
    /// Steps for a story cell `cell` told at `beat`.
    pub(crate) fn new(
        outbox: &'a Outbox,
        control: &'a Control,
        (pace, beat): (Pace, Duration),
        (cell, language): (CellRef, Language),
        (total, stopped): (usize, fn(usize, usize, Language) -> Doc),
    ) -> Steps<'a> {
        Steps {
            outbox,
            control,
            pace,
            beat,
            cell,
            language,
            total,
            stopped,
            done: Cell::new(0),
            unexpected: Cell::new(false),
        }
    }

    /// Waits a beat and shows `working`; `false` after appending where the story stopped.
    pub(crate) fn next(&self, working: impl Into<String>) -> bool {
        if pause(self.control, self.pace, self.beat) {
            self.outbox.status(working);
            return true;
        }
        self.outbox.append(self.cell, (self.stopped)(self.done.get(), self.total, self.language));
        false
    }

    /// Appends a finished step's lines and hands over its record; `expected` is whether the step
    /// came out the way the design says it must.
    pub(crate) fn show(&self, beat: Doc, record: serde_json::Value, expected: bool) {
        self.done.set(self.done.get() + 1);
        if !expected {
            self.unexpected.set(true);
        }
        self.outbox.append(self.cell, beat);
        self.outbox.record(record);
    }
}

/// A fresh 32-byte seed from the operating system, for an example's salts.
pub(crate) fn fresh_seed() -> Result<[u8; 32], String> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|error| error.to_string())?;
    Ok(seed)
}

/// A whole number with thousands separated, however large: `1,099,511,627,776`.
pub(crate) fn grouped(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// `N` in "1 in N" for a small probability: grouped digits while they stay readable, otherwise a
/// power of ten such as `1.6 × 10^24`. A probability of zero or less reads as infinity.
pub(crate) fn one_in(probability: f64) -> String {
    if probability <= 0.0 {
        return "∞".to_string();
    }
    let inverse = 1.0 / probability;
    if inverse < 1e15 {
        return grouped(inverse.round() as u128);
    }
    let exponent = inverse.log10().floor();
    format!("{:.1} × 10^{exponent}", inverse / 10f64.powf(exponent))
}

/// The first `bytes` bytes of a hex string, followed by `…` when there was more.
pub(crate) fn short_hex(hex: &str, bytes: usize) -> String {
    let keep = (bytes * 2).min(hex.len());
    if keep < hex.len() { format!("{}…", &hex[..keep]) } else { hex.to_string() }
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odds_read_as_grouped_digits_or_powers_of_ten() {
        assert_eq!(grouped(1_099_511_627_776), "1,099,511,627,776");
        assert_eq!(one_in(0.5f64.powi(10)), "1,024");
        assert_eq!(one_in(0.5f64.powi(100)), "1.3 × 10^30");
        assert_eq!(one_in(0.0), "∞");
    }

    #[test]
    fn hex_is_shortened_with_an_ellipsis() {
        assert_eq!(short_hex("00112233", 2), "0011…");
        assert_eq!(short_hex("0011", 2), "0011");
        assert_eq!(hex(&[0xab, 0x01]), "ab01");
    }

    #[test]
    fn a_stop_ends_the_wait_at_once() {
        let control = Control::new();
        assert!(pause(&control, Pace::Instant, Duration::from_secs(60)));
        control.cancel();
        let started = Instant::now();
        assert!(!pause(&control, Pace::Story, Duration::from_secs(60)));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
