//! Circle STARKs (Plonky3's `p3-circle` 0.7.0 under `p3-uni-stark` 0.7.0) over Mersenne31, as a
//! museum exhibit.
//!
//! A STARK needs a large multiplicative subgroup of power-of-two order, which the Mersenne prime
//! `2^31 − 1` does not have: `p − 1` is divisible by 2 only once. Circle STARKs (Haböck, Levit and
//! Papini, 2024) take the domain from the circle `x² + y² = 1` over the field instead, whose group
//! of points has order `p + 1 = 2^31`. That keeps Mersenne31's cheap arithmetic, a reduction that
//! is a shift and an addition, for a hash-based STARK.
//!
//! This crate runs Plonky3's implementation on the museum's seven statements: each circuit is built
//! over Mersenne31, lowered to the shared wide AIR and implemented as a Plonky3 `Air` (`air`); the
//! configuration is Plonky3's own circle configuration, a Keccak Merkle-tree commitment under the
//! circle FRI commitment `CirclePcs` (`config`); proving and verifying call `p3_uni_stark::prove`
//! and `verify`, and proofs travel as postcard bytes (`stark`, `prepared`).
//!
//! # Not zero-knowledge
//!
//! `p3-uni-stark` asks the commitment scheme whether to hide (`Pcs::ZK`), and `CirclePcs` answers
//! `false`: 0.7.0 has no hiding circle commitment, no random rows, no salted Merkle leaves. The
//! museum's prediction for a wide AIR without hiding then holds exactly. The trace repeats one row,
//! so each column's polynomial is constant and its opening at the out-of-domain point ζ is the
//! wire value itself.
//!
//! Measured on all seven statements:
//! - `opened_values.trace_local` equals the row lifted into the degree-3 extension field. postcard
//!   writes each such element as three Mersenne31 coefficients in their canonical 4-byte
//!   little-endian form, so a private wire `v` appears as `v` followed by eight zero bytes. Every
//!   private input the scan searches for (2^16 or more) was found that way.
//! - The same 4 bytes appear 100 more times inside the opening proof, which equals the 100 FRI
//!   queries: the committed low-degree extension of a constant column is constant, so each
//!   query's opened row carries the wire values in the clear.
//!
//! The scan searches the 12-byte form only, the one in which the ζ opening is written; bare
//! 4-byte forms of small values match lengths and counts instead of data.
//!
//! The trace has height 4, the smallest `CirclePcs` commits to. The prover checks the witness only
//! in builds with debug assertions, where `p3-uni-stark` runs `check_constraints` first and panics
//! on a false claim; that panic is the prover's refusal. Without debug assertions (release builds)
//! it proves the false claim and `verify` rejects the proof.

mod air;
mod config;
mod field;
mod meta;
mod prepared;
mod stark;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Mersenne31;
pub use meta::META;

/// The field the statements are built over, for activities that compute values in it.
pub type Field = Mersenne31;

/// The Circle STARK exhibit; stateless, since its configuration holds no secrets or randomness.
#[derive(Clone, Copy, Debug, Default)]
pub struct CircleStark;

impl ProofSystem for CircleStark {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        Ok(Box::new(prepared::Ready::build(example, control)?))
    }
}
