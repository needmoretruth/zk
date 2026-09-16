//! Every hash Ligero computes, each behind its own domain string.
//!
//! The domain strings are prefix-free (none is the start of another), so no two kinds of hash can
//! be made to agree by moving bytes between the domain and the data.
//!
//! | domain                    | hashes                                   | where               |
//! |---------------------------|------------------------------------------|---------------------|
//! | `zk/ligero/v1/circuit`    | the circuit                              | [`crate::digest`]   |
//! | `zk/ligero/v1/leaf`       | one salted column                        | [`crate::merkle`]   |
//! | `zk/ligero/v1/node`       | two child hashes                         | [`crate::merkle`]   |
//! | `zk/ligero/v1/tests`      | statement and commitment: seed of round 1 | [`crate::challenge`] |
//! | `zk/ligero/v1/combiners`  | seed of round 1 and a block counter      | [`crate::challenge`] |
//! | `zk/ligero/v1/open`       | seed of round 1 and the responses        | [`crate::challenge`] |
//! | `zk/ligero/v1/columns`    | seed of round 2 and a block counter      | [`crate::challenge`] |
//!
//! Field elements are hashed as 8 canonical little-endian bytes each.

use p3_goldilocks::Goldilocks;
use sha2::{Digest, Sha256};

use crate::field::Fp;

/// A 32-byte salt or SHA-256 digest.
pub type Bytes32 = [u8; 32];

pub(crate) const DOMAIN_CIRCUIT: &[u8] = b"zk/ligero/v1/circuit";
pub(crate) const DOMAIN_LEAF: &[u8] = b"zk/ligero/v1/leaf";
pub(crate) const DOMAIN_NODE: &[u8] = b"zk/ligero/v1/node";
pub(crate) const DOMAIN_TESTS: &[u8] = b"zk/ligero/v1/tests";
pub(crate) const DOMAIN_COMBINERS: &[u8] = b"zk/ligero/v1/combiners";
pub(crate) const DOMAIN_OPEN: &[u8] = b"zk/ligero/v1/open";
pub(crate) const DOMAIN_COLUMNS: &[u8] = b"zk/ligero/v1/columns";

/// Feeds field elements into a hash in their canonical encoding.
pub(crate) fn update_elements(hasher: &mut Sha256, elements: impl IntoIterator<Item = Goldilocks>) {
    for element in elements {
        hasher.update(Fp(element).encode());
    }
}

/// Uniform field elements expanded from a seed: block `b` is `SHA-256(domain ‖ seed ‖ u64 b)`, read
/// as four 8-byte little-endian integers, each kept only below the modulus.
pub(crate) struct ElementStream {
    domain: &'static [u8],
    seed: Bytes32,
    block: u64,
    pending: Vec<u64>,
}

impl ElementStream {
    /// A stream for `seed` under `domain`.
    pub(crate) fn new(domain: &'static [u8], seed: &Bytes32) -> Self {
        Self { domain, seed: *seed, block: 0, pending: Vec::new() }
    }

    /// The next uniform element. A candidate is rejected with probability below 2^-32.
    pub(crate) fn next_element(&mut self) -> Goldilocks {
        loop {
            if let Some(value) = self.pending.pop() {
                if let Some(element) = Fp::from_canonical(value) {
                    return element.0;
                }
                continue;
            }
            let digest = Sha256::new_with_prefix(self.domain)
                .chain_update(self.seed)
                .chain_update(self.block.to_le_bytes())
                .finalize();
            self.block = self.block.wrapping_add(1);
            let (chunks, _) = digest.as_chunks::<8>();
            // Reversed so that `pop` hands the four integers out in reading order.
            self.pending = chunks.iter().rev().map(|chunk| u64::from_le_bytes(*chunk)).collect();
        }
    }

    /// The next `count` elements.
    pub(crate) fn take(&mut self, count: usize) -> Vec<Goldilocks> {
        (0..count).map(|_| self.next_element()).collect()
    }
}
