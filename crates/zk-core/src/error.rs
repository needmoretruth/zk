//! Why a system or a run stopped.

use core::fmt;

use crate::system::Stage;

/// A failure inside one system's own code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemError {
    /// A stop was asked for.
    Cancelled,
    /// The prover checked the witness and refused a false claim.
    Unsatisfied(String),
    /// The operation does not exist for this system (a flip attack on a live conversation).
    NotApplicable(&'static str),
    /// A companion executable is not installed.
    CompanionMissing {
        /// The executable that was looked for.
        executable: String,
    },
    /// Anything else the system reported.
    Failed(String),
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "stopped on request"),
            Self::Unsatisfied(why) => write!(f, "the prover refused a false claim: {why}"),
            Self::NotApplicable(why) => write!(f, "not applicable: {why}"),
            Self::CompanionMissing { executable } => write!(f, "{executable} is not installed"),
            Self::Failed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for SystemError {}

/// Why a whole run did not produce a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The system does not prove this example; the key names a phrase saying why.
    Unsupported {
        /// Phrase key for the reason.
        reason: &'static str,
    },
    /// A stop was asked for; `after` is the last stage that finished, if any.
    Cancelled {
        /// The last completed stage.
        after: Option<Stage>,
    },
    /// The system failed at `stage`.
    Failed {
        /// Where it failed.
        stage: Stage,
        /// What it said.
        error: SystemError,
    },
    /// The operating system could not provide randomness for the seed.
    Randomness(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { reason } => write!(f, "not supported ({reason})"),
            Self::Cancelled { after: Some(stage) } => write!(f, "stopped after {stage:?}"),
            Self::Cancelled { after: None } => write!(f, "stopped before setup finished"),
            Self::Failed { stage, error } => write!(f, "failed during {stage:?}: {error}"),
            Self::Randomness(why) => write!(f, "no randomness available: {why}"),
        }
    }
}

impl std::error::Error for RunError {}
