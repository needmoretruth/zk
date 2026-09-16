//! A random source that hands out a fixed script, so a test can say exactly which passage the
//! prover takes and which face the coin shows.

use core::convert::Infallible;
use std::collections::VecDeque;

use rand_core::TryRng;

/// Draws, in the crate's encoding: a side is 0 for left and 1 for right, a coin 0 for tails and 1
/// for heads (heads calls right).
pub const L: u32 = 0;
pub const R: u32 = 1;
pub const TAILS: u32 = 0;
pub const HEADS: u32 = 1;

/// Hands out the scripted draws in order and panics when they run out: a mode that draws more than
/// the test expected is a failure worth seeing.
pub struct Scripted {
    draws: VecDeque<u32>,
}

impl Scripted {
    pub fn new(draws: &[u32]) -> Self {
        Self { draws: draws.iter().copied().collect() }
    }

    pub fn is_spent(&self) -> bool {
        self.draws.is_empty()
    }
}

impl TryRng for Scripted {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.draws.pop_front().expect("the script ran out of draws"))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        self.try_next_u32().map(u64::from)
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        for byte in dst {
            *byte = self.try_next_u32()? as u8;
        }
        Ok(())
    }
}
