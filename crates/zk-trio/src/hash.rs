//! Every hash Trio computes, each behind its own domain string.
//!
//! The domain strings are prefix-free (none is the start of another), so no two kinds of hash can
//! be made to agree by moving bytes between the domain and the data.
//!
//! - `K_i = SHA-256("zk/trio/v1/cards" ‖ pre_i ‖ (P3 only: card correction))`
//! - `u_i = SHA-256("zk/trio/v1/on" ‖ on_i ‖ (P3 only: input correction))`
//! - `V_i = SHA-256("zk/trio/v1/view" ‖ u_i ‖ every broadcast of P_i)`
//! - element `n` of a seed's tape: the first 8 bytes of
//!   `SHA-256("zk/trio/v1/tape/pre" or "zk/trio/v1/tape/on" ‖ seed ‖ u32 counter)`, read
//!   little-endian and kept only below the modulus (the counter keeps counting past rejections).
//!
//! Field elements are hashed as 8 canonical little-endian bytes each.

use sha2::{Digest, Sha256};

use crate::field::Fp;

/// A 32-byte seed or SHA-256 digest.
pub type Bytes32 = [u8; 32];

pub(crate) const DOMAIN_CARDS: &[u8] = b"zk/trio/v1/cards";
pub(crate) const DOMAIN_ON: &[u8] = b"zk/trio/v1/on";
pub(crate) const DOMAIN_VIEW: &[u8] = b"zk/trio/v1/view";
pub(crate) const DOMAIN_CHALLENGE: &[u8] = b"zk/trio/v1/challenge";
pub(crate) const DOMAIN_DIGITS: &[u8] = b"zk/trio/v1/digits";
pub(crate) const DOMAIN_CIRCUIT: &[u8] = b"zk/trio/v1/circuit";
pub(crate) const DOMAIN_TAPE_PRE: &[u8] = b"zk/trio/v1/tape/pre";
pub(crate) const DOMAIN_TAPE_ON: &[u8] = b"zk/trio/v1/tape/on";
pub(crate) const DOMAIN_COINS: &[u8] = b"zk/trio/v1/coins";

/// Feeds field elements into a hash in their canonical encoding.
pub(crate) fn update_elements(hasher: &mut Sha256, elements: &[Fp]) {
    for element in elements {
        hasher.update(element.encode());
    }
}

/// `K_i`: binds a friend's card seed and, for P3, the dealer's correction.
pub(crate) fn card_commitment(card_seed: &Bytes32, correction: Option<&[Fp]>) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_CARDS);
    hasher.update(card_seed);
    update_elements(&mut hasher, correction.unwrap_or_default());
    hasher.finalize().into()
}

/// `u_i`: binds a friend's input seed (32 bytes of salt) and, for P3, the input correction; sent
/// for the hidden friend on a peek card so its `V` can be recomputed without its seed.
pub(crate) fn view_key(input_seed: &Bytes32, correction: Option<&[Fp]>) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_ON);
    hasher.update(input_seed);
    update_elements(&mut hasher, correction.unwrap_or_default());
    hasher.finalize().into()
}

/// `V_i`: binds `u_i` and every message the friend broadcast.
pub(crate) fn view_commitment(view_key: &Bytes32, broadcasts: &[Fp]) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_VIEW);
    hasher.update(view_key);
    update_elements(&mut hasher, broadcasts);
    hasher.finalize().into()
}

/// The field elements a seed expands to, by rejection sampling so each is uniform.
pub(crate) struct SeedStream {
    domain: &'static [u8],
    seed: Bytes32,
    counter: u32,
}

impl SeedStream {
    /// The card tape of a `pre` seed.
    pub(crate) fn cards(seed: &Bytes32) -> Self {
        Self { domain: DOMAIN_TAPE_PRE, seed: *seed, counter: 0 }
    }

    /// The input tape of an `on` seed.
    pub(crate) fn inputs(seed: &Bytes32) -> Self {
        Self { domain: DOMAIN_TAPE_ON, seed: *seed, counter: 0 }
    }

    /// The next uniform element. A draw is rejected with probability below 2^-32, so the 32-bit
    /// counter cannot run out for any circuit that fits in memory.
    pub(crate) fn next_element(&mut self) -> Fp {
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
    pub(crate) fn take(&mut self, count: usize) -> Vec<Fp> {
        (0..count).map(|_| self.next_element()).collect()
    }
}
