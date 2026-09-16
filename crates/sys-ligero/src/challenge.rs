//! The verifier's two messages, drawn from hashes of everything sent before them (Fiat–Shamir, §5.2).
//!
//! **Round 1, the random combinations.**
//! `seed₁ = SHA-256("zk/ligero/v1/tests" ‖ u32 id length ‖ example id ‖ circuit digest ‖
//! u32 public inputs ‖ element… ‖ root)`, with the digest of [`crate::digest`]. Field elements are
//! read from `SHA-256("zk/ligero/v1/combiners" ‖ seed₁ ‖ u64 block)` for blocks 0, 1, … as four
//! 8-byte little-endian integers each, those at or above the modulus skipped. For every repetition
//! in turn: one element per tested row (proximity test), one per linear constraint (linear test),
//! one per row of the block `x` (quadratic test).
//!
//! **Round 2, the opened columns.**
//! `seed₂ = SHA-256("zk/ligero/v1/open" ‖ seed₁ ‖ responses)`, the responses exactly as the proof
//! encodes them. `SHA-256("zk/ligero/v1/columns" ‖ seed₂ ‖ u64 block)` for blocks 0, 1, … is read
//! as eight 4-byte little-endian integers each; an integer names the column `integer mod n` (`n` is a
//! power of two, so every column is equally likely), a repeat is skipped, and reading stops at `t`
//! distinct columns. The columns are opened in ascending order.

use std::collections::BTreeSet;

use p3_goldilocks::Goldilocks;
use sha2::{Digest, Sha256};

use crate::field::Fp;
use crate::hash::{
    Bytes32, DOMAIN_COLUMNS, DOMAIN_COMBINERS, DOMAIN_OPEN, DOMAIN_TESTS, ElementStream,
    update_elements,
};
use crate::params::REPETITIONS;
use crate::statement::Statement;

type F = Goldilocks;

/// One repetition's random combinations.
#[derive(Clone, Debug)]
pub(crate) struct Combiners {
    /// `r`: how the proximity test combines the tested rows.
    pub code: Vec<F>,
    /// `r^add`: how the linear test combines the linear constraints.
    pub linear: Vec<F>,
    /// `r^q`: how the quadratic test combines the rows of triples.
    pub quadratic: Vec<F>,
}

/// `seed₁`: binds the statement, the public inputs and the commitment.
pub(crate) fn tests_seed(statement: &Statement, public: &[Fp], root: &Bytes32) -> Bytes32 {
    let id = statement.id();
    let mut hasher = Sha256::new_with_prefix(DOMAIN_TESTS);
    hasher.update((id.len() as u32).to_le_bytes());
    hasher.update(id.as_bytes());
    hasher.update(statement.digest());
    hasher.update((public.len() as u32).to_le_bytes());
    update_elements(&mut hasher, public.iter().map(|value| value.0));
    hasher.update(root);
    hasher.finalize().into()
}

/// Every repetition's combinations, drawn from `seed₁`.
pub(crate) fn combiners(statement: &Statement, seed: &Bytes32) -> Vec<Combiners> {
    let params = statement.params();
    let mut stream = ElementStream::new(DOMAIN_COMBINERS, seed);
    (0..REPETITIONS)
        .map(|_| Combiners {
            code: stream.take(params.tested_rows()),
            linear: stream.take(statement.linear_constraints()),
            quadratic: stream.take(params.triple_rows),
        })
        .collect()
}

/// `seed₂`: binds round 1 and the prover's answers to it.
pub(crate) fn open_seed(tests_seed: &Bytes32, responses: &[u8]) -> Bytes32 {
    Sha256::new_with_prefix(DOMAIN_OPEN)
        .chain_update(tests_seed)
        .chain_update(responses)
        .finalize()
        .into()
}

/// The `count` columns to open, ascending.
pub(crate) fn opened_columns(seed: &Bytes32, length: usize, count: usize) -> Vec<usize> {
    let count = count.min(length);
    let mut chosen = BTreeSet::new();
    let mut block = 0u64;
    while chosen.len() < count {
        let digest = Sha256::new_with_prefix(DOMAIN_COLUMNS)
            .chain_update(seed)
            .chain_update(block.to_le_bytes())
            .finalize();
        block += 1;
        for chunk in digest.as_chunks::<4>().0 {
            if chosen.len() == count {
                break;
            }
            chosen.insert(u32::from_le_bytes(*chunk) as usize & (length - 1));
        }
    }
    chosen.into_iter().collect()
}
