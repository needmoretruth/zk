//! Where randomness comes from: the operating system, or a seed for demonstrations that must
//! replay the same way twice.

use core::fmt;

use sha2::{Digest, Sha256};

use crate::cast::Card;
use crate::error::TrioError;
use crate::field::Fp;
use crate::hash::{Bytes32, DOMAIN_COINS};

/// A source of coin flips for a prover's seeds or a verifier's cards.
///
/// Real proofs use [`Coins::os`]. [`Coins::seeded`] exists so a test or a replayed demonstration
/// draws the same cards again; a seeded prover is not hiding anything from whoever knows the seed.
pub struct Coins {
    seed: Option<Bytes32>,
    counter: u64,
}

impl Coins {
    /// Fresh randomness from the operating system.
    pub fn os() -> Self {
        Self { seed: None, counter: 0 }
    }

    /// A reproducible stream: SHA-256 of a domain string, `seed` and a counter.
    pub fn seeded(seed: Bytes32) -> Self {
        Self { seed: Some(seed), counter: 0 }
    }

    /// Fills `out` with random bytes.
    pub fn fill(&mut self, out: &mut [u8]) -> Result<(), TrioError> {
        let Some(seed) = self.seed else {
            return getrandom::fill(out).map_err(|e| TrioError::Randomness(e.to_string()));
        };
        for chunk in out.chunks_mut(32) {
            let block = Sha256::new_with_prefix(DOMAIN_COINS)
                .chain_update(seed)
                .chain_update(self.counter.to_le_bytes())
                .finalize();
            self.counter = self.counter.wrapping_add(1);
            chunk.copy_from_slice(&block[..chunk.len()]);
        }
        Ok(())
    }

    /// A fresh 32-byte seed.
    pub fn seed(&mut self) -> Result<Bytes32, TrioError> {
        let mut seed = [0u8; 32];
        self.fill(&mut seed)?;
        Ok(seed)
    }

    /// One of the five cards, uniformly: a byte below 250 taken modulo 5, larger bytes redrawn.
    pub fn card(&mut self) -> Result<Card, TrioError> {
        loop {
            let mut byte = [0u8; 1];
            self.fill(&mut byte)?;
            if byte[0] < 250
                && let Some(card) = Card::from_digit(byte[0] % 5)
            {
                return Ok(card);
            }
        }
    }

    /// A uniform field element.
    pub fn element(&mut self) -> Result<Fp, TrioError> {
        loop {
            let mut bytes = [0u8; 8];
            self.fill(&mut bytes)?;
            if let Some(element) = Fp::from_canonical(u64::from_le_bytes(bytes)) {
                return Ok(element);
            }
        }
    }
}

impl fmt::Debug for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.seed.is_some() { "Coins(seeded)" } else { "Coins(os)" })
    }
}
