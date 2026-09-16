//! The videotape: exactly what the camera at the fork could see, and nothing more.
//!
//! # What is on tape
//! For each scene, in order: the side the reporter called, and the side the prover came out on.
//! Scene numbers are positions on the tape.
//!
//! # What is not on tape
//! - which passage the prover entered, and anything that happened at the wall — the camera stays
//!   at the fork;
//! - who was in the cave — the double looks like Mick;
//! - the magic words;
//! - the coin as a value of its own — heads always means the call "right", so the call already says
//!   what the camera saw of the coin, and a tape made by prior agreement, where no coin was flipped,
//!   has the same fields as a genuine one (the story's court could not tell those apart either);
//! - the takes an editor cut.
//!
//! # Byte format, version 1
//! | offset | bytes | content |
//! |---|---|---|
//! | 0 | 4 | ASCII `CAVE` |
//! | 4 | 1 | format version, `1` |
//! | 5 | 4 | number of scenes `n`, unsigned little-endian |
//! | 9 | 2·`n` | per scene: the call, then the exit, each ASCII `L` (left) or `R` (right) |
//!
//! Sides are letters rather than 0 and 1 so the tape never holds more than three zero bytes in a
//! row: a small secret such as 3 encodes as a 3 followed by seven zeros, and the museum's secret
//! scan must not find that pattern in a tape by accident.

use crate::cast::Side;

/// The first bytes of every tape.
pub const TAPE_MAGIC: [u8; 4] = *b"CAVE";

/// The byte format this crate writes.
pub const TAPE_VERSION: u8 = 1;

const HEADER_BYTES: usize = 9;

/// What the camera recorded of one scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TapeScene {
    /// The side the reporter called.
    pub call: Side,
    /// The side the prover came out on.
    pub exit: Side,
}

impl TapeScene {
    /// Whether the prover came out on the called side.
    pub fn succeeded(&self) -> bool {
        self.call == self.exit
    }
}

/// A videotape of scenes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tape {
    /// Scenes in the order they appear.
    pub scenes: Vec<TapeScene>,
}

impl Tape {
    /// Whether every scene on the tape succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.scenes.iter().all(TapeScene::succeeded)
    }

    /// The tape in the byte format described in the module documentation.
    pub fn to_bytes(&self) -> Vec<u8> {
        // Scene counts come from `u32` scene numbers everywhere in this crate; a hand-built tape
        // longer than that is cut at `u32::MAX` scenes so the header stays truthful.
        let count = u32::try_from(self.scenes.len()).unwrap_or(u32::MAX);
        let mut bytes = Vec::with_capacity(HEADER_BYTES + 2 * self.scenes.len());
        bytes.extend_from_slice(&TAPE_MAGIC);
        bytes.push(TAPE_VERSION);
        bytes.extend_from_slice(&count.to_le_bytes());
        for scene in self.scenes.iter().take(count as usize) {
            bytes.push(side_byte(scene.call));
            bytes.push(side_byte(scene.exit));
        }
        bytes
    }

    /// Reads a tape written by [`Tape::to_bytes`]; `None` for anything that is not exactly one.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (header, body) = bytes.split_at_checked(HEADER_BYTES)?;
        if header[..4] != TAPE_MAGIC || header[4] != TAPE_VERSION {
            return None;
        }
        let count = u32::from_le_bytes([header[5], header[6], header[7], header[8]]) as usize;
        let (pairs, rest) = body.as_chunks::<2>();
        if pairs.len() != count || !rest.is_empty() {
            return None;
        }
        let scenes = pairs
            .iter()
            .map(|[call, exit]| {
                Some(TapeScene { call: byte_side(*call)?, exit: byte_side(*exit)? })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { scenes })
    }
}

impl FromIterator<TapeScene> for Tape {
    fn from_iter<I: IntoIterator<Item = TapeScene>>(iter: I) -> Self {
        Self { scenes: iter.into_iter().collect() }
    }
}

fn side_byte(side: Side) -> u8 {
    match side {
        Side::Left => b'L',
        Side::Right => b'R',
    }
}

fn byte_side(byte: u8) -> Option<Side> {
    match byte {
        b'L' => Some(Side::Left),
        b'R' => Some(Side::Right),
        _ => None,
    }
}
