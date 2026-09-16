//! Why a ZKBoo operation stopped.

use core::fmt;

use zk_core::SystemError;

/// A failure outside the verifier's judgement: the verifier's "no" is a check result, not an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BooError {
    /// The operating system could not provide randomness for a seed.
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
    /// The witness was not computed for this circuit.
    WitnessCount {
        /// Witness values the circuit needs.
        expected: usize,
        /// Values supplied.
        got: usize,
    },
    /// A response does not open the two views its challenge asks for, with the lengths the circuit
    /// fixes.
    ResponseDoesNotFit,
    /// A stop was asked for.
    Cancelled,
}

impl fmt::Display for BooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Randomness(why) => write!(f, "no randomness available: {why}"),
            Self::Circuit(why) => write!(f, "circuit error: {why}"),
            Self::PublicInputCount { expected, got } => {
                write!(f, "expected {expected} public inputs, got {got}")
            }
            Self::WitnessCount { expected, got } => {
                write!(f, "expected {expected} witness values, got {got}")
            }
            Self::ResponseDoesNotFit => write!(f, "the response does not fit its challenge"),
            Self::Cancelled => write!(f, "stopped on request"),
        }
    }
}

impl std::error::Error for BooError {}

impl From<BooError> for SystemError {
    fn from(error: BooError) -> Self {
        match error {
            BooError::Cancelled => SystemError::Cancelled,
            other => SystemError::Failed(other.to_string()),
        }
    }
}
