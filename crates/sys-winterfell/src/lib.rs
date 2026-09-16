//! Winterfell's STARK prover (`winter-prover` 0.13.1) over its f128 field, as a museum exhibit.
//!
//! Winterfell is the STARK prover Meta released in 2021, which Polygon Miden VM ran on until it
//! moved to Plonky3. This crate runs it on the museum's statements: each circuit is built over
//! Winterfell's f128, lowered to the shared wide AIR and implemented as a Winterfell `Air`
//! (`air`); proving calls `Prover::prove` with Winterfell's default trace extension, constraint
//! evaluator and constraint commitment over Blake3-256, and verifying calls
//! `winter_verifier::verify`; proofs travel as `Proof::to_bytes` (`stark`, `prepared`).
//!
//! The trace is eight rows: every wire column holds its value on all of them, and one extra column
//! counts steps, because `winter-prover` 0.13.1 panics on a trace whose columns are all constant.
//! `air` explains that column and why every constraint declares degree 1.
//!
//! # Not zero-knowledge, and the museum shows it
//!
//! Winterfell extends and commits to the trace exactly as given. Each wire column's polynomial is
//! constant, so every place a proof carries a trace value carries the wire value itself: the
//! out-of-domain frame (each column at `z` and at `g·z`) and the trace queries the verifier opens
//! against the Merkle commitment (each column at the queried points of the extended domain), each
//! as 16 little-endian bytes. The leak scan finds every private input of at least 2^16.
//!
//! # Why f128
//!
//! Winterfell's own example for "~96-bit security" runs over f128 with no field extension; over
//! f62 the same security needs the quadratic extension.
//!
//! # Two statements do not fit
//!
//! A Winterfell trace has at most 255 columns, and a proof read back from bytes at most 254
//! (`TraceInfo` stores the width in one byte), one of which is the step counter. With one column
//! per wire, `membership` needs 321 and `pool-spend` 1008, so [`Winterfell::support`] reports them
//! unsupported rather than proving a different circuit.
//!
//! # When the prover refuses
//!
//! In builds with debug assertions Winterfell validates the trace against the AIR before proving
//! and panics on a false claim ("main transition constraint … did not evaluate to ZERO"); that
//! panic is the prover's refusal. Without debug assertions (release builds) it proves the false
//! claim and `winter_verifier::verify` rejects the proof.

mod air;
mod field;
mod meta;
mod prepared;
mod stark;

use zk_circuit::lower::wide_air::WideAir;
use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, Support, SystemError};

pub use field::F128;
pub use meta::META;

/// The field the statements are built over, for activities that compute values in it.
pub type Field = F128;

/// Phrase key for a statement whose wide row has more columns than a Winterfell trace can hold.
pub const TRACE_TOO_WIDE: &str = "winterfell-trace-too-wide";

/// The Winterfell exhibit; stateless, since a STARK has no keys to keep.
#[derive(Clone, Copy, Debug, Default)]
pub struct Winterfell;

impl ProofSystem for Winterfell {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    /// Unsupported exactly when the lowered circuit needs more columns than a proof can carry.
    ///
    /// The count is measured on the circuit itself rather than listed by name, so a statement that
    /// grows or shrinks moves across the line without anyone editing this crate.
    fn support(&self, example: ExampleId) -> Support {
        match example.circuit::<F128>() {
            Ok(circuit)
                if WideAir::from_circuit(&circuit).num_columns() > air::MAX_WIRE_COLUMNS =>
            {
                Support::Unsupported { reason: TRACE_TOO_WIDE }
            }
            _ => Support::Full,
        }
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        Ok(Box::new(prepared::Ready::build(example, control)?))
    }
}
