//! The Fiat–Shamir challenges: one party per round, drawn from a hash of the statement and of every
//! first message, so the prover cannot choose them.
//!
//! `seed = SHA-256("zk/zkboo/v1/challenge" ‖ u32 id length ‖ example id ‖ circuit digest ‖
//! u32 public inputs ‖ element… ‖ u32 rounds ‖ first message…)`, integers little-endian, elements 8
//! canonical little-endian bytes, each first message in its proof encoding (commitments, then output
//! shares). The output shares are hashed with the commitments because they are part of the paper's
//! first message `a`: were they left out, a prover could pick the hidden party's share after seeing
//! the challenge and make any assertion sum to zero.
//!
//! The challenges are read as the paper reads them (§5.2), by rejection sampling to base 3: bytes
//! of `SHA-256("zk/zkboo/v1/trits" ‖ seed ‖ u64 block)` for blocks 0, 1, …, each byte split into
//! four bit pairs from the most significant; a pair `(a, b)` gives `2a + b + 1` (P1, P2 or P3), and
//! the pair `(1, 1)` is skipped.

use sha2::{Digest, Sha256};

use crate::codec::write_first;
use crate::field::Fp;
use crate::hash::{Bytes32, DOMAIN_CHALLENGE, DOMAIN_TRITS, update_elements};
use crate::party::Party;
use crate::program::Statement;
use crate::round::FirstMessage;

fn challenge_seed(statement: &Statement, public: &[Fp], firsts: &[FirstMessage]) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_CHALLENGE);
    hasher.update((statement.id().len() as u32).to_le_bytes());
    hasher.update(statement.id().as_bytes());
    hasher.update(statement.digest());
    hasher.update((public.len() as u32).to_le_bytes());
    update_elements(&mut hasher, public);
    hasher.update((firsts.len() as u32).to_le_bytes());
    let mut encoded = Vec::new();
    for first in firsts {
        encoded.clear();
        write_first(&mut encoded, first);
        hasher.update(&encoded);
    }
    hasher.finalize().into()
}

/// One challenge per first message, in order.
pub fn challenges(statement: &Statement, public: &[Fp], firsts: &[FirstMessage]) -> Vec<Party> {
    let seed = challenge_seed(statement, public, firsts);
    let mut drawn = Vec::with_capacity(firsts.len());
    let mut block = 0u64;
    while drawn.len() < firsts.len() {
        let bytes = Sha256::new_with_prefix(DOMAIN_TRITS)
            .chain_update(seed)
            .chain_update(block.to_le_bytes())
            .finalize();
        block += 1;
        let pairs =
            bytes.into_iter().flat_map(|byte| [6, 4, 2, 0].map(|shift| (byte >> shift) & 3));
        let missing = firsts.len() - drawn.len();
        drawn.extend(pairs.filter_map(|pair| Party::ALL.get(usize::from(pair))).take(missing));
    }
    drawn
}
