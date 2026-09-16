//! Every hash ZKBoo computes, each behind its own domain string, and the tapes seeds expand to.
//!
//! The domain strings are prefix-free (none is the start of another), so no two kinds of hash can
//! be made to agree by moving bytes between the domain and the data.
//!
//! - view commitment `c_i = SHA-256("zk/zkboo/v1/view" ‖ k_i ‖ (P3 only: x_3 × S) ‖ z_i × M)`, the
//!   paper's `Com(k_i, w_i)` (§5.2 commits with SHA-256 too): the seed is 32 random bytes, so it also
//!   hides the view;
//! - element `n` of a tape: the first 8 bytes of
//!   `SHA-256("zk/zkboo/v1/tape/input" or "zk/zkboo/v1/tape/mul" ‖ k_i ‖ u32 counter)`, read
//!   little-endian and kept only below the modulus (the counter keeps counting past rejections).
//!   The paper expands tapes with AES in counter mode (§5.2); SHA-256 keeps the hash the only
//!   primitive.
//!
//! Field elements are hashed as 8 canonical little-endian bytes each.

use sha2::{Digest, Sha256};

use crate::field::Fp;

/// A 32-byte seed or SHA-256 digest.
pub type Bytes32 = [u8; 32];

pub(crate) const DOMAIN_VIEW: &[u8] = b"zk/zkboo/v1/view";
pub(crate) const DOMAIN_TAPE_INPUT: &[u8] = b"zk/zkboo/v1/tape/input";
pub(crate) const DOMAIN_TAPE_MUL: &[u8] = b"zk/zkboo/v1/tape/mul";
pub(crate) const DOMAIN_CHALLENGE: &[u8] = b"zk/zkboo/v1/challenge";
pub(crate) const DOMAIN_TRITS: &[u8] = b"zk/zkboo/v1/trits";
pub(crate) const DOMAIN_CIRCUIT: &[u8] = b"zk/zkboo/v1/circuit";

/// Feeds field elements into a hash in their canonical encoding.
pub(crate) fn update_elements(hasher: &mut Sha256, elements: &[Fp]) {
    for element in elements {
        hasher.update(element.encode());
    }
}

/// `c_i`: binds a party's seed, its input shares when the seed does not produce them (P3; empty
/// for P1 and P2), and every multiplication output it computed.
pub(crate) fn view_commitment(seed: &Bytes32, input_shares: &[Fp], mul_outputs: &[Fp]) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_VIEW);
    hasher.update(seed);
    update_elements(&mut hasher, input_shares);
    update_elements(&mut hasher, mul_outputs);
    hasher.finalize().into()
}

/// A fresh 32-byte seed from the operating system.
pub(crate) fn fresh_seed() -> Result<Bytes32, String> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| e.to_string())?;
    Ok(seed)
}

/// The field elements a seed expands to, by rejection sampling so each is uniform.
pub(crate) struct SeedStream {
    domain: &'static [u8],
    seed: Bytes32,
    counter: u32,
}

impl SeedStream {
    /// The input-share tape of a P1 or P2 seed.
    pub(crate) fn inputs(seed: &Bytes32) -> Self {
        Self { domain: DOMAIN_TAPE_INPUT, seed: *seed, counter: 0 }
    }

    /// The multiplication tape `R_i(1), R_i(2), …` of any party's seed.
    pub(crate) fn multiplications(seed: &Bytes32) -> Self {
        Self { domain: DOMAIN_TAPE_MUL, seed: *seed, counter: 0 }
    }

    /// The next uniform element. A draw is rejected with probability below 2^-32, so the 32-bit
    /// counter cannot run out for any circuit that fits in memory.
    fn next_element(&mut self) -> Fp {
        loop {
            let digest = Sha256::new_with_prefix(self.domain)
                .chain_update(self.seed)
                .chain_update(self.counter.to_le_bytes())
                .finalize();
            self.counter = self.counter.wrapping_add(1);
            let mut first = [0u8; 8];
            first.copy_from_slice(&digest[..8]);
            if let Some(element) = Fp::from_canonical(u64::from_le_bytes(first)) {
                return element;
            }
        }
    }

    /// The next `count` elements.
    pub(crate) fn take(mut self, count: usize) -> Vec<Fp> {
        (0..count).map(|_| self.next_element()).collect()
    }
}
