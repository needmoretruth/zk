//! Why a scene could not be played.

use core::fmt;

use zk_core::SystemError;

/// A failure of the machinery around the story, never an outcome of it: a double being caught is a
/// result ([`crate::Demonstration::caught_at`]), not an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaveError {
    /// The random source could not flip the coin or pick a passage.
    Randomness(String),
    /// The example's circuit could not be built over Goldilocks.
    Circuit(String),
    /// The jealous reporter stopped filming after `takes` takes with only `kept` good scenes. With
    /// a fair coin this practically never happens (each take succeeds half the time), so it points
    /// to a broken random source; the limit keeps such a source from filming forever.
    EditNeverFinished {
        /// Takes filmed.
        takes: u32,
        /// Successful scenes among them.
        kept: u32,
    },
}

impl fmt::Display for CaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Randomness(why) => write!(f, "no randomness for the coin or the passage: {why}"),
            Self::Circuit(why) => write!(f, "the wall's lock could not be built: {why}"),
            Self::EditNeverFinished { takes, kept } => {
                write!(f, "the edit stopped after {takes} takes with {kept} good scenes")
            }
        }
    }
}

impl std::error::Error for CaveError {}

impl From<CaveError> for SystemError {
    fn from(error: CaveError) -> Self {
        Self::Failed(error.to_string())
    }
}
