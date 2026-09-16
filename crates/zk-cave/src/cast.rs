//! The two sides of the cave, the coin, and the person who walks in.

use rand_core::TryRng;

use crate::error::CaveError;
use crate::wall::{MagicWords, Wall};

/// The operating system's random source. Outside tests the reporter's coin and the prover's choice
/// of passage come from here, never from the example's seed, so a run cannot be replayed to predict
/// the coin. A re-export rather than an alias, so `&mut OsRng` works as a value.
pub use getrandom::SysRng as OsRng;

/// One of the two dark winding passages the cave's entryway forks into. Both end in a dead end;
/// only the wall joins their ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The left-hand passage.
    Left,
    /// The right-hand passage, where Ali Baba hid under the sacks and overheard the words.
    Right,
}

impl Side {
    /// The passage across the wall.
    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// A passage picked with one random bit (1 is right).
    pub(crate) fn draw<R: TryRng + ?Sized>(rng: &mut R) -> Result<Self, CaveError> {
        Ok(if bit(rng)? { Self::Right } else { Self::Left })
    }
}

/// The two faces of the reporter's coin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Coin {
    /// "Mick, come out on the right."
    Heads,
    /// "Mick, come out on the left."
    Tails,
}

impl Coin {
    /// The exit the reporter calls for this face, as in the paper: heads right, tails left.
    pub fn call(self) -> Side {
        match self {
            Self::Heads => Side::Right,
            Self::Tails => Side::Left,
        }
    }

    /// A fair flip: one random bit (1 is heads).
    pub(crate) fn flip<R: TryRng + ?Sized>(rng: &mut R) -> Result<Self, CaveError> {
        Ok(if bit(rng)? { Self::Heads } else { Self::Tails })
    }
}

/// Who walks into the cave, so an animation draws the right figure.
///
/// Only the label differs: the double looks like Mick and follows the same steps, and whether the
/// wall opens depends on the words carried, never on who carries them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Character {
    /// Mick Ali, perhaps a descendant of Ali Baba, who knows the words.
    MickAli,
    /// The stage actor who looks like Mick and does not know the words.
    Double,
}

/// The person in the cave and the words they will whisper if they have to cross.
#[derive(Clone, Debug)]
pub struct Prover {
    who: Character,
    words: Option<MagicWords>,
}

impl Prover {
    /// Mick Ali with the words he knows.
    pub fn mick_ali(words: MagicWords) -> Self {
        Self { who: Character::MickAli, words: Some(words) }
    }

    /// The double, with wrong words or none at all.
    pub fn double(words: Option<MagicWords>) -> Self {
        Self { who: Character::Double, words }
    }

    /// Who this is.
    pub fn who(&self) -> Character {
        self.who
    }

    /// Standing at the dead end of `entered` when the reporter calls `call`: walk straight back if
    /// already on that side, otherwise whisper at the wall. Returns what happened at the wall and
    /// the side the prover comes out on.
    pub(crate) fn respond(&self, wall: &Wall, entered: Side, call: Side) -> (WallOutcome, Side) {
        if entered == call {
            return (WallOutcome::NotNeeded, entered);
        }
        match &self.words {
            None => (WallOutcome::NothingToWhisper, entered),
            Some(words) if wall.whisper(words) => (WallOutcome::Opened, call),
            Some(_) => (WallOutcome::StayedShut, entered),
        }
    }
}

/// What happened at the wall in one scene. Hidden from the reporter and not on the tape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WallOutcome {
    /// The prover was already in the called passage and never went to the wall.
    NotNeeded,
    /// The prover whispered, the wall slid open, the prover passed, and it slid closed again.
    Opened,
    /// The prover whispered words that do not open it.
    StayedShut,
    /// The prover had no words to whisper.
    NothingToWhisper,
}

/// One random bit: the lowest bit of one `u32` from `rng`.
fn bit<R: TryRng + ?Sized>(rng: &mut R) -> Result<bool, CaveError> {
    rng.try_next_u32().map(|word| word & 1 == 1).map_err(|e| CaveError::Randomness(e.to_string()))
}
