//! Part B: Powers of Tau, a reference string many people build in turn, so that one of them not
//! keeping their secret is enough.
//!
//! The string is `([τ⁰]G1, [τ¹]G1, …, [τ⁶³]G1, [τ]G2)` and starts at τ = 1, a τ everyone knows.
//! Each participant draws a secret `s`, multiplies entry `i` of G1 by `sⁱ` and the G2 entry by `s`,
//! which turns τ into `τ·s` without anyone learning either, and publishes a [`Contribution`]. After
//! the last turn τ is the product of every secret; while one of them is unknown, so is τ.
//! [`verify_chain`] lets anyone check that every turn really was such a multiplication.

mod chain;
pub mod cheat;
mod participant;
mod pok;
mod srs;

pub use chain::{ChainError, Check, verify_chain};
pub use participant::{Contribution, KeptSecret, Participant};
pub use pok::KnowledgeProof;
pub use srs::{DEFAULT_PARTICIPANTS, POWERS, Srs, digest};
