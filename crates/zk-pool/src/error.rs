//! Why an activity could not run.

use core::fmt;

use zk_core::SystemError;

/// A reason an activity stopped before a ledger could decide anything.
///
/// A transaction the ledger turns down is not an error: it comes back as a [`crate::Receipt`] whose
/// [`crate::Decision`] says why, because a refusal is what the reader came to see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PoolError {
    /// Wallet names become file names, so only `a`–`z`, `0`–`9`, `-` and `_`, 1 to 32 of them.
    InvalidName(String),
    /// A wallet with this name already exists.
    WalletExists(String),
    /// No wallet has this name.
    UnknownWallet(String),
    /// Moving nothing is not an activity.
    ZeroAmount,
    /// A shield needs transparent coins to put into the pool.
    NotEnoughTransparentFunds {
        /// The wallet.
        name: String,
        /// Its transparent balance.
        balance: u64,
        /// What it tried to shield.
        amount: u16,
    },
    /// A spend has one input, so a single unspent note must hold the whole amount.
    NoNoteLargeEnough {
        /// The wallet.
        name: String,
        /// What it tried to move.
        amount: u16,
        /// Its largest unspent note, zero when it has none.
        largest: u16,
    },
    /// Reading or writing a file failed.
    Storage(String),
    /// A stored file does not describe a valid pool.
    Corrupt(String),
    /// The stored ledger was made with another proof system or over another field.
    Mismatch {
        /// What this pool uses.
        expected: String,
        /// What the ledger file says.
        found: String,
    },
    /// The spend circuit could not be built or evaluated over this field.
    Circuit(String),
    /// The operating system could not provide randomness for a key or a note.
    Randomness(String),
    /// The proof system failed, refused, or was cancelled while building keys or proving.
    System(SystemError),
}

impl From<SystemError> for PoolError {
    fn from(error: SystemError) -> Self {
        Self::System(error)
    }
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "{name:?} is not a valid wallet name"),
            Self::WalletExists(name) => write!(f, "wallet {name} already exists"),
            Self::UnknownWallet(name) => write!(f, "no wallet named {name}"),
            Self::ZeroAmount => write!(f, "the amount is zero"),
            Self::NotEnoughTransparentFunds { name, balance, amount } => {
                write!(f, "{name} has {balance} transparent, cannot shield {amount}")
            }
            Self::NoNoteLargeEnough { name, amount, largest } => {
                write!(f, "{name} has no note of at least {amount} (largest {largest})")
            }
            Self::Storage(why) => write!(f, "storage: {why}"),
            Self::Corrupt(why) => write!(f, "corrupt pool data: {why}"),
            Self::Mismatch { expected, found } => {
                write!(f, "the ledger belongs to {found}, this pool uses {expected}")
            }
            Self::Circuit(why) => write!(f, "spend circuit: {why}"),
            Self::Randomness(why) => write!(f, "no randomness available: {why}"),
            Self::System(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PoolError {}
