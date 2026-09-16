//! Trio: a proof designed for this museum, which trusts nothing but a hash.
//!
//! Ali Baba's cave convinces the reporter only because everyone trusts the magic wall. Trio proves
//! any of the museum's statements with SHA-256 alone. It is MPC-in-the-head: the prover imagines
//! three friends who hold the witness split into three random shares that add up to it, and who
//! compute the circuit together without any of them learning it. A dealer hands out, in advance,
//! one card `(a, b, c = a·b)` per multiplication, split among the friends the same way (Beaver
//! triples). The prover commits to every friend's seeds and messages; then the verifier draws one
//! of five cards:
//!
//! - two **dealer cards** open every card seed and check `c = a·b` for every multiplication;
//! - three **peek cards** each hide one friend, open the other two, re-run their computation and
//!   check every commitment and that every assertion's three shares sum to zero.
//!
//! A rigged card survives three cards of five, and so does a friend who computed wrongly. A cheat
//! survives one round with probability 3/5: (3/5)^55 < 2^-40 for a live conversation
//! ([`Session`]), (3/5)^109 < 2^-80 for a proof made alone, where the cards come from a hash
//! (Fiat–Shamir). Dealer and peek cards open only uniformly random shares, so a round shows
//! nothing about the witness; [`Simulator`] makes accepted rounds without one.
//!
//! A simplified redesign of the preprocessing approach of KKW (Katz–Kolesnikov–Wang, CCS 2018)
//! with a five-card challenge. Teaching implementation, not audited.
//!
//! The first draft of the design committed a friend's input seed and messages in one hash, so
//! a peek card could not check the hidden friend's messages and a forger could rewrite them after
//! seeing the card. [`Cheat::RewriteHiddenShares`] replays that forgery against the fixed design.

mod cast;
mod challenge;
mod cheat;
mod check;
mod codec;
mod coins;
mod deal;
pub mod digest;
mod error;
mod field;
mod hash;
mod meta;
mod mpc;
mod program;
pub mod proof;
mod prover;
mod rig;
mod round;
mod session;
mod simulator;
mod system;

pub use cast::{Card, Friend};
pub use cheat::{
    Cheat, CheatRun, CheatingProver, ClaimSource, FalseClaim, escape_odds, escape_probability,
    play_cheat,
};
pub use check::{Check, CheckKind, check_round};
pub use codec::COMMITMENT_BYTES;
pub use coins::Coins;
pub use deal::Corrections;
pub use error::TrioError;
pub use field::{ELEMENT_BYTES, Fp, MODULUS};
pub use hash::Bytes32;
pub use meta::META;
pub use program::{Claim, Statement};
pub use proof::{PROOF_ROUNDS, TAMPER_OFFSET};
pub use prover::{HonestProver, RoundProver};
pub use rig::RiggedCard;
pub use round::{Commitments, OpenedSeeds, Opening};
pub use session::{
    INTERACTIVE_ROUNDS, Opened, RoundEvent, Session, SessionSummary, opened_items, play,
};
pub use simulator::{SimulatedRound, Simulator, simulate};
pub use system::Trio;
