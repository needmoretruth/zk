//! Proving: commit to every committed wire, then let `sigma-proofs` prove the relation.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ff::Field as _;
use rand_core::OsRng;
use zk_circuit::{Assignment, Circuit};
use zk_core::{ExampleId, SystemError};

use crate::field::PallasScalar;
use crate::layout::Layout;
use crate::pedersen::{Generators, Opening};
use crate::proof;
use crate::relation;

/// Evaluates `assignment` without checking it, commits with fresh blindings from the operating
/// system, and returns the commitments followed by `sigma-proofs`' batchable proof.
///
/// Nothing here checks the witness. `sigma-proofs` does not check it either, so a false claim still
/// becomes a proof, and it is the verifier that turns it down; only if the upstream refuses or
/// panics is the claim reported as unprovable.
pub(crate) fn prove(
    circuit: &Circuit<PallasScalar>,
    example: ExampleId,
    generators: &Generators,
    assignment: &Assignment<PallasScalar>,
) -> Result<Vec<u8>, SystemError> {
    let evaluation =
        circuit.evaluate_unchecked(assignment).map_err(|e| SystemError::Failed(e.to_string()))?;
    let layout = Layout::new(circuit, &assignment.public);
    let openings: Vec<Opening> = layout
        .committed
        .iter()
        .map(|wire| Opening {
            value: evaluation.values.get(*wire),
            blinding: PallasScalar::random(OsRng),
        })
        .collect();
    let commitments: Vec<_> = openings.iter().map(|opening| generators.commit(*opening)).collect();
    let witness = relation::witness(&layout, &openings);
    let statement = relation::build(&layout, &commitments, generators);
    let session = proof::session(example, &assignment.public, &commitments);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        statement.into_nizk(&session)?.prove_batchable(&witness, &mut OsRng)
    }));
    match outcome {
        Ok(Ok(upstream)) => Ok(proof::assemble(&commitments, &upstream)),
        Ok(Err(error)) => Err(SystemError::Unsatisfied(format!("sigma-proofs: {error}"))),
        Err(panic) => Err(SystemError::Unsatisfied(panic_message(panic.as_ref()))),
    }
}

fn panic_message(panic: &(dyn Any + Send)) -> String {
    let message = panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the prover panicked".to_string());
    format!("sigma-proofs panicked: {message}")
}
