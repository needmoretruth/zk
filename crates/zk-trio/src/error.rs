//! Why a Trio operation stopped.

use core::fmt;

use zk_core::SystemError;

use crate::cast::Card;

/// A failure outside the verifier's judgement: the verifier's "no" is a check result, not an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrioError {
    /// The operating system could not provide randomness.
    Randomness(String),
    /// The circuit could not be built or evaluated.
    Circuit(String),
    /// The public inputs do not match the circuit.
    PublicInputCount {
        /// Public inputs the circuit declares.
        expected: usize,
        /// Values supplied.
        got: usize,
    },
    /// A prover was asked to open a round before committing to one.
    NothingCommitted,
    /// The simulator was shown a card other than the one it prepared the round for.
    UnplannedCard {
        /// The card the simulator prepared for.
        planned: Card,
        /// The card actually drawn.
        drawn: Card,
    },
    /// The simulator has no planned card left for another round.
    PlanExhausted,
    /// The session already has a verdict; no further round is played.
    SessionOver,
    /// No single rigged multiplication card makes any false claim of this example pass.
    NoRiggableClaim,
    /// A stop was asked for.
    Cancelled,
}

impl fmt::Display for TrioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Randomness(why) => write!(f, "no randomness available: {why}"),
            Self::Circuit(why) => write!(f, "circuit error: {why}"),
            Self::PublicInputCount { expected, got } => {
                write!(f, "expected {expected} public inputs, got {got}")
            }
            Self::NothingCommitted => write!(f, "asked to open a round that was never committed"),
            Self::UnplannedCard { planned, drawn } => {
                write!(f, "the simulator planned for {planned:?} but {drawn:?} was drawn")
            }
            Self::PlanExhausted => write!(f, "the simulator has no planned card left"),
            Self::SessionOver => write!(f, "the session is over"),
            Self::NoRiggableClaim => {
                write!(f, "no single rigged multiplication card makes a false claim pass")
            }
            Self::Cancelled => write!(f, "stopped on request"),
        }
    }
}

impl std::error::Error for TrioError {}

impl From<TrioError> for SystemError {
    fn from(error: TrioError) -> Self {
        match error {
            TrioError::Cancelled => SystemError::Cancelled,
            other => SystemError::Failed(other.to_string()),
        }
    }
}
