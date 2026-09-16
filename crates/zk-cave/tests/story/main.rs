//! The story's rules, one behaviour per test: the demonstration, the double, the edit, the court,
//! prior agreement, the apartment building, the tape and the odds.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod court;
mod demonstration;
mod filming;
mod scripted;
mod tape;

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use zk_cave::Setting;
use zk_core::ExampleId;

/// Seed for the examples' salts.
pub const SEED: [u8; 32] = [42; 32];

/// A setting for a small example, so tests that play many scenes stay quick.
pub fn setting() -> Setting {
    Setting::for_example(ExampleId::Factoring, &SEED).unwrap()
}

/// A reproducible fair random source.
pub fn seeded(stream: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(stream)
}
