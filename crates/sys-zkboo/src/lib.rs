//! ZKBoo: a zero-knowledge proof from a three-party computation the prover runs in its head.
//!
//! **Teaching implementation · not audited.** Written for this museum from the paper — I. Giacomelli,
//! J. Madsen, C. Orlandi, *ZKBoo: Faster Zero-Knowledge for Boolean Circuits*, USENIX Security 2016,
//! <https://eprint.iacr.org/2016/163> — because no permissively licensed Rust implementation for
//! arithmetic circuits exists. Only the field (`p3-goldilocks`) and the hash (`sha2`) come from crates.
//!
//! The prover splits the witness into three additive shares and runs the paper's *linear
//! (2,3)-decomposition* of the circuit (§4.1): three parties compute on their shares, and each
//! multiplication mixes a party's shares with its right-hand neighbour's and with randomness from
//! both their tapes ([`mod@proof`] has the formulas). It commits to each party's view and publishes every
//! party's share of every assert-zero value. The verifier picks `e ∈ {1, 2, 3}`; the prover opens
//! views `e` and `e+1`; the verifier recomputes view `e`'s multiplications from the two, checks both
//! commitments and checks that every assertion's three shares sum to zero (Fig. 6).
//!
//! A cheat has to break the computation between one pair of neighbours, and only one challenge in
//! three recomputes that pair: one round's soundness error is 2/3 (3-special soundness,
//! Proposition 2). Two views are uniformly random apart from what the statement fixes (2-privacy,
//! Appendix A), so a round shows nothing about the witness: [`hidden_view_for`] builds, for any
//! other witness, the one hidden view that makes an opened pair fit it.
//!
//! A proof repeats the round [`PROOF_ROUNDS`] times and draws the challenges from a hash
//! (Fiat–Shamir, §5.2). The museum's own Trio is MPC in the head with a different decomposition
//! (Beaver triples, five challenges, error 3/5 per round), so the two can be compared side by side.

mod challenge;
mod check;
mod codec;
mod digest;
mod error;
mod field;
mod hash;
mod hidden;
mod meta;
mod mpc;
mod party;
mod program;
pub mod proof;
mod round;
mod system;
mod tape;

pub use challenge::challenges;
pub use check::{Check, CheckKind, check_round};
pub use error::BooError;
pub use field::{ELEMENT_BYTES, Fp, MODULUS};

/// The field every ZKBoo share lives in, under the name every exhibit exports it by, so an activity
/// can compute values for [`zk_core::Prepared::prove_assignment`].
pub type Field = Fp;
pub use hash::Bytes32;
pub use hidden::{HiddenView, hidden_view_for};
pub use meta::META;
pub use party::Party;
pub use program::{Claim, Statement};
pub use proof::{OpenedRound, PROOF_ROUNDS};
pub use round::{Cheat, FirstMessage, OpenedView, Response, Round};
pub use system::ZkBoo;
