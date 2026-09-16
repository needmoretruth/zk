//! Ligero: a zero-knowledge argument from Reed–Solomon codes, whose proof grows with the square root
//! of the circuit and which trusts nothing but a hash.
//!
//! This is a **teaching implementation · not audited**, written for this museum from the paper —
//! S. Ames, C. Hazay, Y. Ishai, M. Venkitasubramaniam, *Ligero: Lightweight Sublinear Arguments
//! Without a Trusted Setup*, CCS 2017, extended version <https://eprint.iacr.org/2022/1608> —
//! because the implementations that exist (libiop, Google's Longfellow ZK) are C++. The field
//! (`p3-goldilocks`), the NTT (`p3-dft`) and the hash (`sha2`) come from crates.
//!
//! How a proof is made (§4.7, §5.2):
//!
//! 1. The museum's R1CS rows become the paper's constraints: a witness vector with linear
//!    constraints `A·x = b` and quadratic constraints `x ⊙ y − z = 0` on aligned triples
//!    (module `constraints` says how each row maps).
//! 2. The extended witness is laid into rows of `ℓ` entries. Each row becomes a random polynomial of
//!    degree below `k` that takes the row's entries at the points `ζ`, and is written down as its
//!    `n` evaluations at other points `η`: an interleaved Reed–Solomon codeword (module `code`). Six
//!    blinding rows are added, three per repetition.
//! 3. Every column of the matrix is hashed with a salt into a Merkle tree; the root is the
//!    commitment (module `merkle`).
//! 4. A hash of the statement, the public inputs and the root gives random combinations. The prover
//!    answers each of the three tests with a polynomial: the combined rows (**proximity**: the rows
//!    are close to codewords), a polynomial whose values at `ζ` must sum to `rᵀb` (**linear**), and
//!    one that must vanish at every `ζ` (**quadratic**).
//! 5. A hash of those answers picks `t = 118` columns; the prover opens them with their salts and
//!    the Merkle siblings. The verifier checks each answer against the opened columns, where a
//!    polynomial that lies disagrees with the committed rows almost everywhere.
//!
//! [`mod@params`] derives `n = 4k`, `t = 118` and two repetitions for about 2^-80 soundness from the
//! paper's bounds, and picks `ℓ` per circuit so the proof grows with the square root of the circuit.
//! The blinding rows and the `k − ℓ > t` free evaluations per row make the opened columns and the
//! answers uniformly random, which is the paper's zero knowledge (Lemma 4.15). [`mod@proof`]
//! documents the byte format.

mod challenge;
mod code;
mod coins;
mod constraints;
mod digest;
mod error;
mod field;
mod hash;
mod merkle;
mod meta;
pub mod params;
pub mod proof;
mod prover;
mod statement;
mod system;
mod verifier;

pub use error::LigeroError;
pub use field::{ELEMENT_BYTES, Fp, MODULUS};

/// The field every Ligero value lives in, under the name every exhibit exports it by, so an activity
/// can compute values for [`zk_core::Prepared::prove_assignment`].
pub type Field = Fp;
pub use hash::Bytes32;
pub use meta::META;
pub use params::{OPENED_COLUMNS, Params, REPETITIONS, SECURITY_BITS};
pub use proof::{OpenedColumn, Proof, Response, tamper_offset};
pub use prover::prove;
pub use statement::{Claim, ExtendedWitness, Statement};
pub use system::Ligero;
pub use verifier::{Checks, check, verify};
