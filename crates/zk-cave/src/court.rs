//! The court: two tapes in evidence, and what the judges and experts could count on them.
//!
//! "The judges and the experts could not tell the tapes apart." A judge sees only tapes, so these
//! counts are all a judge can compare. A genuine tape and an edited one draw every count from the
//! same distribution: in the genuine tape each call is a fair coin and Mick always comes out on the
//! called side; in the edited one the double's passage is a fair coin independent of the call, so
//! the kept scenes (passage equal to call) still carry fair, independent calls, and every kept exit
//! matches. An unedited tape of the double is different — about half its exits miss — which is why
//! the counts are worth making.

use crate::cast::Side;
use crate::tape::Tape;

/// What a judge can count on one tape.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TapeStats {
    /// Scenes on the tape.
    pub scenes: u32,
    /// Scenes where the reporter called right.
    pub right_calls: u32,
    /// Scenes where the reporter called left.
    pub left_calls: u32,
    /// Scenes where the prover came out on the called side.
    pub exits_matching_call: u32,
    /// Runs of the same call in a row: right, right, left, right has three.
    pub runs: u32,
    /// The longest run of the same call.
    pub longest_run: u32,
    /// `run_lengths[k]` counts the runs of exactly `k + 1` equal calls.
    pub run_lengths: Vec<u32>,
}

impl TapeStats {
    /// Counts `tape`.
    pub fn of(tape: &Tape) -> Self {
        let mut stats = Self::default();
        let mut current = 0u32;
        let mut previous = None;
        for scene in &tape.scenes {
            stats.scenes += 1;
            match scene.call {
                Side::Right => stats.right_calls += 1,
                Side::Left => stats.left_calls += 1,
            }
            if scene.succeeded() {
                stats.exits_matching_call += 1;
            }
            if previous == Some(scene.call) {
                current += 1;
            } else {
                stats.close_run(current);
                current = 1;
            }
            previous = Some(scene.call);
        }
        stats.close_run(current);
        stats
    }

    /// The share of calls that were right, 0 for an empty tape.
    pub fn right_call_share(&self) -> f64 {
        share(self.right_calls, self.scenes)
    }

    /// The share of exits that matched the call, 0 for an empty tape.
    pub fn match_share(&self) -> f64 {
        share(self.exits_matching_call, self.scenes)
    }

    fn close_run(&mut self, length: u32) {
        let Some(index) = (length as usize).checked_sub(1) else { return };
        self.runs += 1;
        self.longest_run = self.longest_run.max(length);
        if self.run_lengths.len() <= index {
            self.run_lengths.resize(index + 1, 0);
        }
        self.run_lengths[index] += 1;
    }
}

/// Two tapes side by side, as the court saw them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hearing {
    /// The first network's tape of Mick.
    pub genuine: TapeStats,
    /// The jealous reporter's edited tape of the double.
    pub edited: TapeStats,
}

/// Counts both tapes for the court.
pub fn court(genuine: &Tape, edited: &Tape) -> Hearing {
    Hearing { genuine: TapeStats::of(genuine), edited: TapeStats::of(edited) }
}

/// The same counts averaged over many tapes, where a real difference in distribution would show
/// even though any single pair of tapes differs by chance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Averages {
    /// Tapes averaged.
    pub tapes: u32,
    /// Mean share of right calls.
    pub right_call_share: f64,
    /// Mean share of exits matching the call.
    pub match_share: f64,
    /// Mean number of runs per tape.
    pub runs: f64,
    /// Mean longest run per tape.
    pub longest_run: f64,
}

impl Averages {
    /// Averages the counts of `tapes`; all zero when there are none.
    pub fn of(tapes: &[Tape]) -> Self {
        let stats: Vec<TapeStats> = tapes.iter().map(TapeStats::of).collect();
        let count = stats.len() as f64;
        if stats.is_empty() {
            return Self::default();
        }
        let mean = |value: fn(&TapeStats) -> f64| stats.iter().map(value).sum::<f64>() / count;
        Self {
            tapes: u32::try_from(stats.len()).unwrap_or(u32::MAX),
            right_call_share: mean(TapeStats::right_call_share),
            match_share: mean(TapeStats::match_share),
            runs: mean(|s| f64::from(s.runs)),
            longest_run: mean(|s| f64::from(s.longest_run)),
        }
    }
}

fn share(part: u32, whole: u32) -> f64 {
    if whole == 0 { 0.0 } else { f64::from(part) / f64::from(whole) }
}
