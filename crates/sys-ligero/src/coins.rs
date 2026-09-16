//! The prover's private randomness, from the operating system.
//!
//! Zero knowledge rests on it: the free evaluations of every row polynomial, the blinding rows and
//! the column salts. The verifier's randomness, by contrast, comes from hashes ([`crate::challenge`]).

use p3_goldilocks::Goldilocks;

use crate::error::LigeroError;
use crate::field::Fp;
use crate::hash::Bytes32;

/// Bytes drawn from the operating system at a time.
const BUFFER_BYTES: usize = 1 << 16;

/// A buffered source of operating-system randomness.
pub(crate) struct Coins {
    buffer: Vec<u8>,
    position: usize,
}

impl Coins {
    /// An empty buffer; the first draw fills it.
    pub(crate) fn new() -> Self {
        Self { buffer: Vec::new(), position: 0 }
    }

    fn fill(&mut self, out: &mut [u8]) -> Result<(), LigeroError> {
        let mut written = 0;
        while written < out.len() {
            if self.position == self.buffer.len() {
                self.buffer.resize(BUFFER_BYTES, 0);
                getrandom::fill(&mut self.buffer)
                    .map_err(|e| LigeroError::Randomness(e.to_string()))?;
                self.position = 0;
            }
            let count = (out.len() - written).min(self.buffer.len() - self.position);
            out[written..written + count]
                .copy_from_slice(&self.buffer[self.position..self.position + count]);
            self.position += count;
            written += count;
        }
        Ok(())
    }

    /// A fresh 32-byte salt.
    pub(crate) fn salt(&mut self) -> Result<Bytes32, LigeroError> {
        let mut salt = [0u8; 32];
        self.fill(&mut salt)?;
        Ok(salt)
    }

    /// A uniform field element, by rejection sampling of 64-bit integers.
    pub(crate) fn element(&mut self) -> Result<Goldilocks, LigeroError> {
        loop {
            let mut bytes = [0u8; 8];
            self.fill(&mut bytes)?;
            if let Some(element) = Fp::from_canonical(u64::from_le_bytes(bytes)) {
                return Ok(element.0);
            }
        }
    }

    /// `count` uniform field elements.
    pub(crate) fn elements(&mut self, count: usize) -> Result<Vec<Goldilocks>, LigeroError> {
        (0..count).map(|_| self.element()).collect()
    }
}
