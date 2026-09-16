//! Plonky3's univariate STARK (`p3-uni-stark` 0.7.0) over BabyBear, as a museum exhibit.
//!
//! Plonky3 is Polygon's toolkit for hash-based proofs over small fields. This crate runs its
//! univariate STARK on the museum's seven statements: each circuit is built over BabyBear, lowered
//! to the shared wide AIR and implemented as a Plonky3 `Air` (`air`); the configuration is a
//! Keccak Merkle-tree commitment under the FRI polynomial commitment with hiding switched on
//! (`config`); proving and verifying call `p3_uni_stark::prove` and `verify`, and proofs travel
//! as postcard bytes (`stark`, `prepared`).
//!
//! # Zero knowledge, as found in the 0.7.0 code
//!
//! `p3-uni-stark` has no switch of its own: it asks the commitment scheme (`Pcs::ZK`). With the
//! plain `TwoAdicFriPcs` a proof is not hiding. With `p3_fri::HidingFriPcs` it is, and this
//! exhibit runs with it. Hiding changes four things, all inside Plonky3:
//! - the prover commits to a trace of twice the height, the real rows interleaved with random
//!   rows, plus random extra columns (`HidingFriPcs::commit`);
//! - the quotient chunks are randomized by vanishing-polynomial multiples that cancel in the
//!   recombination (ePrint 2024/1037, §4.2), so there are twice as many chunks;
//! - a committed random polynomial blinds the FRI batch, and the random columns' openings travel
//!   separately in the opening proof;
//! - Merkle leaves are salted (`MerkleTreeHidingMmcs`), so a commitment does not fix the leaves.
//!
//! Plonky3's own comment calls the result statistically, not perfectly, zero-knowledge, because
//! the batch-blinding polynomial is sampled as base-field polynomials rather than a true
//! extension-field one.
//!
//! Without hiding, the museum's prediction holds exactly. The trace repeats one row, so each
//! column's polynomial is constant and its opening at the out-of-domain point ζ is the wire value
//! itself. Measured on `one-plus-one` with the plain `TwoAdicFriPcs` over the unsalted Keccak tree
//! of Plonky3's examples, `opened_values.trace_local` equals the row lifted into the extension
//! field. postcard writes
//! each such element as four BabyBear coefficients in their 4-byte little-endian Montgomery form
//! (`value·2^32 mod p`), so a private wire `v` appears as Montgomery(`v`) followed by twelve
//! zero bytes. Both private inputs of `one-plus-one` were found that way. With hiding, the
//! opening at ζ mixes the real row with a random row, and the scan finds neither.
//!
//! The scan searches the Montgomery form only, the one form in which postcard writes a field
//! element in these proofs. The canonical 4-byte forms are not searched, because they
//! match structure instead of data. In a hiding `sudoku` proof, `00 00 00 04` is three absent
//! optional openings followed by the quotient-chunk count, and it would report the cells that
//! hold 4 as found.
//!
//! The trace has height 1, the smallest `p3-uni-stark` accepts; setup draws no keys, only the
//! hiding randomness. The prover checks the witness only in builds with debug assertions, where
//! `p3-uni-stark` runs `check_constraints` first and panics on a false claim; that panic is the
//! prover's refusal. Without debug assertions (release builds) it proves the false claim and
//! `verify` rejects the proof.

mod air;
mod config;
mod field;
mod meta;
mod prepared;
mod stark;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::BabyBear;
pub use meta::META;

/// The field the statements are built over, for activities that compute values in it.
pub type Field = BabyBear;

/// The Plonky3 exhibit; stateless, since every preparation draws its own hiding randomness.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plonky3;

impl ProofSystem for Plonky3 {
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
