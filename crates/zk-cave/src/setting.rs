//! A cave ready for one of the seven examples.

use zk_circuit::Circuit;
use zk_examples::{ExampleId, InstanceKind};

use crate::error::CaveError;
use crate::field::Goldilocks;
use crate::wall::{MagicWords, Wall};

/// The wall set to an example's true claim, the words that open it, and words that do not.
#[derive(Clone, Debug)]
pub struct Setting {
    /// Locked to the true claim's public inputs.
    pub wall: Wall,
    /// The true claim's private inputs: Mick's words.
    pub words: MagicWords,
    /// The false claim's private inputs, for a double who learned the wrong words. For each of the
    /// seven examples they keep this wall shut; the crate's tests check that.
    pub wrong_words: MagicWords,
}

impl Setting {
    /// The setting for `example`, with the example's salts derived from `seed`.
    ///
    /// `seed` only salts the example. The coin and the passages come from the random source handed
    /// to each mode.
    pub fn for_example(example: ExampleId, seed: &[u8; 32]) -> Result<Self, CaveError> {
        Ok(Self::with_circuit(example, circuit(example)?, seed))
    }

    /// One setting per floor of an apartment building, floor 1 first.
    ///
    /// Each floor's salts come from `seed` with the floor number mixed into its last four bytes, so
    /// examples with a salt (`one-plus-one`, `age`) get different words on every floor. The other
    /// examples have no salt, and their floors share the same words.
    pub fn floors(example: ExampleId, count: u32, seed: &[u8; 32]) -> Result<Vec<Self>, CaveError> {
        let circuit = circuit(example)?;
        Ok((1..=count)
            .map(|floor| {
                let mut floor_seed = *seed;
                for (byte, mix) in floor_seed[28..].iter_mut().zip(floor.to_le_bytes()) {
                    *byte ^= mix;
                }
                Self::with_circuit(example, circuit.clone(), &floor_seed)
            })
            .collect())
    }

    fn with_circuit(example: ExampleId, circuit: Circuit<Goldilocks>, seed: &[u8; 32]) -> Self {
        let claim = example.instance::<Goldilocks>(InstanceKind::Honest, seed);
        let false_claim = example.instance::<Goldilocks>(InstanceKind::Dishonest, seed);
        Self {
            wall: Wall::new(circuit, claim.public),
            words: MagicWords::new(claim.private),
            wrong_words: MagicWords::new(false_claim.private),
        }
    }
}

fn circuit(example: ExampleId) -> Result<Circuit<Goldilocks>, CaveError> {
    example.circuit::<Goldilocks>().map_err(|e| CaveError::Circuit(e.to_string()))
}
