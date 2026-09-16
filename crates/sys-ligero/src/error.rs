//! Why a Ligero operation stopped.

use core::fmt;

use zk_core::SystemError;

/// A failure outside the verifier's judgement: the verifier's "no" is a check result, not an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LigeroError {
    /// The operating system could not provide randomness for the encoding or the salts.
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
    /// One block of the extended witness has the wrong length for this circuit.
    WitnessLength {
        /// The block: `w`, `x`, `y` or `z`.
        block: &'static str,
        /// Entries the circuit fixes.
        expected: usize,
        /// Entries supplied.
        got: usize,
    },
    /// No parameters fit: the matrix would need a subgroup larger than the field has.
    TooLarge {
        /// Witness entries (w and the three copies) that had to be placed.
        entries: usize,
    },
    /// A stop was asked for.
    Cancelled,
}

impl fmt::Display for LigeroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Randomness(why) => write!(f, "no randomness available: {why}"),
            Self::Circuit(why) => write!(f, "circuit error: {why}"),
            Self::PublicInputCount { expected, got } => {
                write!(f, "expected {expected} public inputs, got {got}")
            }
            Self::WitnessLength { block, expected, got } => {
                write!(f, "witness block {block} needs {expected} entries, got {got}")
            }
            Self::TooLarge { entries } => {
                write!(
                    f,
                    "{entries} witness entries do not fit any Reed–Solomon code over Goldilocks"
                )
            }
            Self::Cancelled => write!(f, "stopped on request"),
        }
    }
}

impl std::error::Error for LigeroError {}

impl From<LigeroError> for SystemError {
    fn from(error: LigeroError) -> Self {
        match error {
            LigeroError::Cancelled => SystemError::Cancelled,
            other => SystemError::Failed(other.to_string()),
        }
    }
}
