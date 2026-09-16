//! Commitments and openings as bytes, for the non-interactive proof.
//!
//! An opening's layout depends only on its card and on the circuit's counts (`M` multiplications,
//! `S` witness values, `A` assertions), so it carries no tags or lengths:
//!
//! - dealer card: `pre_1 ‖ pre_2 ‖ pre_3 ‖ c^3 × M`
//! - peek card hiding `h`: for each other friend in order `pre ‖ on`, then (only when P3 is opened)
//!   `c^3 × M ‖ w^3 × S`, then `u_h ‖ broadcasts of h × (2M + A)`
//!
//! Seeds and digests are 32 bytes, elements 8 canonical little-endian bytes.

use crate::cast::{Card, Friend};
use crate::deal::Corrections;
use crate::field::{ELEMENT_BYTES, Fp};
use crate::hash::Bytes32;
use crate::program::Statement;
use crate::round::{Commitments, OpenedSeeds, Opening};

/// Bytes of one round's commitments.
pub const COMMITMENT_BYTES: usize = 6 * 32;

/// Reads a proof front to back; every failure is a sentence for [`zk_core::Verdict::Malformed`].
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position.checked_add(count).filter(|end| *end <= self.bytes.len());
        let end =
            end.ok_or_else(|| format!("the proof ends early, at byte {}", self.bytes.len()))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, String> {
        let mut array = [0u8; 4];
        array.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(array))
    }

    pub(crate) fn bytes32(&mut self) -> Result<Bytes32, String> {
        let mut array = [0u8; 32];
        array.copy_from_slice(self.take(32)?);
        Ok(array)
    }

    pub(crate) fn elements(&mut self, count: usize) -> Result<Vec<Fp>, String> {
        let bytes = self.take(count.checked_mul(ELEMENT_BYTES).ok_or("too many elements")?)?;
        bytes.as_chunks::<ELEMENT_BYTES>().0.iter().map(|chunk| Fp::decode(chunk)).collect()
    }

    /// Refuses bytes after the proof, so one proof has one encoding.
    pub(crate) fn finish(self) -> Result<(), String> {
        let extra = self.bytes.len() - self.position;
        if extra == 0 { Ok(()) } else { Err(format!("{extra} bytes follow the proof")) }
    }
}

pub(crate) fn write_elements(out: &mut Vec<u8>, elements: &[Fp]) {
    for element in elements {
        out.extend_from_slice(&element.encode());
    }
}

pub(crate) fn write_commitments(out: &mut Vec<u8>, commitments: &Commitments) {
    for digest in commitments.cards.iter().chain(&commitments.views) {
        out.extend_from_slice(digest);
    }
}

pub(crate) fn read_commitments(reader: &mut Reader<'_>) -> Result<Commitments, String> {
    let cards = [reader.bytes32()?, reader.bytes32()?, reader.bytes32()?];
    let views = [reader.bytes32()?, reader.bytes32()?, reader.bytes32()?];
    Ok(Commitments { cards, views })
}

pub(crate) fn write_opening(out: &mut Vec<u8>, opening: &Opening) {
    match opening {
        Opening::Dealer { card_seeds, card_correction } => {
            for seed in card_seeds {
                out.extend_from_slice(seed);
            }
            write_elements(out, card_correction);
        }
        Opening::Peek { opened, corrections, hidden_view_key, hidden_broadcasts, .. } => {
            for seeds in opened {
                out.extend_from_slice(&seeds.card_seed);
                out.extend_from_slice(&seeds.input_seed);
            }
            if let Some(corrections) = corrections {
                write_elements(out, &corrections.cards);
                write_elements(out, &corrections.inputs);
            }
            out.extend_from_slice(hidden_view_key);
            write_elements(out, hidden_broadcasts);
        }
    }
}

pub(crate) fn read_opening(
    reader: &mut Reader<'_>,
    statement: &Statement,
    card: Card,
) -> Result<Opening, String> {
    let m = statement.multiplications();
    let Some(hidden) = card.hidden() else {
        let card_seeds = [reader.bytes32()?, reader.bytes32()?, reader.bytes32()?];
        return Ok(Opening::Dealer { card_seeds, card_correction: reader.elements(m)? });
    };
    let [first, second] = hidden.others();
    let mut seeds = |friend: Friend| -> Result<OpenedSeeds, String> {
        Ok(OpenedSeeds { friend, card_seed: reader.bytes32()?, input_seed: reader.bytes32()? })
    };
    let opened = [seeds(first)?, seeds(second)?];
    let corrections = if hidden == Friend::P3 {
        None
    } else {
        let cards = reader.elements(m)?;
        Some(Corrections { cards, inputs: reader.elements(statement.witness_values())? })
    };
    let hidden_view_key = reader.bytes32()?;
    let hidden_broadcasts = reader.elements(statement.broadcasts_per_friend())?;
    Ok(Opening::Peek { hidden, opened, corrections, hidden_view_key, hidden_broadcasts })
}
