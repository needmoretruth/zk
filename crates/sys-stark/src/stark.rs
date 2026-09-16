//! Calls into lambdaworks' STARK prover and verifier: the real protocol, in bytes.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use bincode::config::{Configuration, standard};
use lambdaworks_math::traits::AsBytes;
use stark_platinum_prover::PrimeField;
use stark_platinum_prover::proof::options::{ProofOptions, SecurityLevel};
use stark_platinum_prover::proof::stark::StarkProof;
use stark_platinum_prover::prover::{IsStarkProver, Prover};
use stark_platinum_prover::trace::TraceTable;
use stark_platinum_prover::traits::AIR;
use stark_platinum_prover::transcript::StoneProverTranscript;
use stark_platinum_prover::verifier::{IsStarkVerifier, Verifier};
use zk_core::{SystemError, Verdict};

use crate::air::{CircuitAir, Statement};
use crate::field::Val;

/// A proof over Stark252 with no extension field, as lambdaworks writes it.
type Proof = StarkProof<PrimeField, PrimeField>;

/// Rows in every trace this exhibit proves: the one wire row, repeated four times.
///
/// Four is the fewest lambdaworks can verify, measured by a test below. Its FRI folds the DEEP
/// composition polynomial `log2(rows)` times and sends the last fold as a value, so a trace of one
/// or two rows commits no inner FRI layer and answers no query, and `Verifier::verify` rejects any
/// proof with fewer answers than queries. The prover itself only asks for a power of two.
pub(crate) const TRACE_LENGTH: usize = 4;

/// The coset offset of the low-degree extension domain: 3, as in lambdaworks' tests and its
/// default test options.
const COSET_OFFSET: u64 = 3;

/// lambdaworks' preset for 128 bits of conjectured security (ethSTARK §5.10.1): blowup 4, 55 FRI
/// queries, 20 bits of grinding.
pub(crate) fn options() -> ProofOptions {
    ProofOptions::new_secure(SecurityLevel::Conjecturable128Bits, COSET_OFFSET)
}

/// bincode's standard configuration, which lambdaworks' own STARK command-line prover used.
fn config() -> Configuration {
    standard()
}

/// The transcript both sides start from, seeded with the claimed public values.
fn transcript(statement: &Statement) -> StoneProverTranscript {
    StoneProverTranscript::new(&statement.as_bytes())
}

/// Runs `Prover::prove` on the wire row repeated [`TRACE_LENGTH`] times and serializes the proof
/// with `bincode::serde`.
///
/// Nothing is checked here, and lambdaworks does not refuse either: in builds with debug assertions
/// it validates the trace against the AIR but only logs what fails, so a false claim is proved and
/// left to the verifier. An error the prover returns, or a panic, is still reported as its refusal.
pub(crate) fn prove(statement: &Statement, row: &[Val]) -> Result<Vec<u8>, SystemError> {
    let proof = run_prover(statement, row, TRACE_LENGTH)?;
    bincode::serde::encode_to_vec(&proof, config())
        .map_err(|e| SystemError::Failed(format!("bincode could not serialize the proof: {e}")))
}

/// `Prover::prove` on `row` repeated `rows` times, with the prover's errors and panics as refusals.
fn run_prover(statement: &Statement, row: &[Val], rows: usize) -> Result<Proof, SystemError> {
    let columns = row.iter().map(|value| vec![*value; rows]).collect();
    let mut trace = TraceTable::from_columns_main(columns, 1);
    let air = CircuitAir::new(rows, statement, &options());
    let mut transcript = transcript(statement);
    let proved =
        catch_unwind(AssertUnwindSafe(|| Prover::prove(&air, &mut trace, &mut transcript)));
    match proved {
        Ok(Ok(proof)) => Ok(proof),
        Ok(Err(error)) => {
            Err(SystemError::Unsatisfied(format!("lambdaworks prover refused: {error}")))
        }
        Err(panic) => Err(SystemError::Unsatisfied(panic_message("prove", panic.as_ref()))),
    }
}

/// Decodes `bytes` and runs `Verifier::verify`, classifying the outcome.
///
/// Bytes bincode cannot decode, bytes left over, and a proof whose trace length or out-of-domain
/// table does not have this statement's shape are malformed, and so is a verifier panic: lambdaworks
/// indexes a decoded proof's vectors without checking their lengths. A `false` from the verifier is
/// a rejection.
pub(crate) fn verify(statement: &Statement, bytes: &[u8]) -> Verdict {
    let proof = match decode(bytes) {
        Ok(proof) => proof,
        Err(why) => return Verdict::Malformed(why),
    };
    if let Some(why) = shape_mismatch(&proof, statement.columns()) {
        return Verdict::Malformed(why);
    }
    let air = CircuitAir::new(TRACE_LENGTH, statement, &options());
    let mut transcript = transcript(statement);
    let verified =
        catch_unwind(AssertUnwindSafe(|| Verifier::verify(&proof, &air, &mut transcript)));
    match verified {
        Ok(true) => Verdict::Accepted,
        Ok(false) => Verdict::Rejected,
        Err(panic) => Verdict::Malformed(panic_message("verify", panic.as_ref())),
    }
}

/// `decode_from_slice` over the whole input; trailing bytes are refused rather than ignored.
fn decode(bytes: &[u8]) -> Result<Proof, String> {
    let (proof, used) = bincode::serde::decode_from_slice::<Proof, _>(bytes, config())
        .map_err(|e| format!("bincode could not decode the proof: {e}"))?;
    if used != bytes.len() {
        return Err(format!("{} bytes follow the proof", bytes.len() - used));
    }
    Ok(proof)
}

/// Why a decoded proof cannot be about this statement's trace, if it cannot.
fn shape_mismatch(proof: &Proof, columns: usize) -> Option<String> {
    let table = &proof.trace_ood_evaluations;
    let length = proof.trace_length;
    let consistent = table.width.checked_mul(table.height) == Some(table.data.len());
    if length != TRACE_LENGTH || table.width != columns || table.height != 1 || !consistent {
        let (width, height) = (table.width, table.height);
        return Some(format!(
            "the proof is for {length} rows and an out-of-domain table of {width}×{height}, \
             not {TRACE_LENGTH} rows and {columns}×1"
        ));
    }
    None
}

/// Where the lowest byte of the first out-of-domain trace value sits in a serialized proof, if the
/// bytes decode.
///
/// bincode writes a struct as its fields in order with no framing, so the offset is measured with
/// bincode itself: the fields before `trace_ood_evaluations`, the length of its `data` vector, then
/// the first element, whose last byte is the lowest of its 32 big-endian bytes.
pub(crate) fn ood_trace_offset(bytes: &[u8]) -> Option<usize> {
    let proof = decode(bytes).ok()?;
    let data = &proof.trace_ood_evaluations.data;
    let size = |encoded: Result<Vec<u8>, _>| encoded.map_or(0, |bytes: Vec<u8>| bytes.len());
    let head =
        (proof.trace_length, proof.lde_trace_main_merkle_root, proof.lde_trace_aux_merkle_root);
    let before = size(bincode::serde::encode_to_vec(head, config()))
        + size(bincode::serde::encode_to_vec(data.len(), config()));
    let first = size(bincode::serde::encode_to_vec(data.first()?, config()));
    let offset = (before + first).checked_sub(1)?;
    (offset < bytes.len()).then_some(offset)
}

/// The text a panic carried, when it carried text.
fn panic_message(step: &str, payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("lambdaworks {step} panicked: {text}"),
        (_, Some(text)) => format!("lambdaworks {step} panicked: {text}"),
        _ => format!("lambdaworks {step} panicked"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::sync::Arc;

    use stark_platinum_prover::traits::AIR;
    use stark_platinum_prover::verifier::{IsStarkVerifier, Verifier};
    use zk_circuit::lower::wide_air::WideAir;
    use zk_examples::{ExampleId, InstanceKind};

    use super::{options, run_prover, transcript};
    use crate::air::{CircuitAir, Statement};
    use crate::field::{Stark252, Val};

    /// Fails if the fewest rows lambdaworks can verify moves away from four.
    ///
    /// lambdaworks runs FRI with `log2(rows)` folds of which the last is sent as a value, so one or
    /// two rows commit no inner layer and answer no query, and `Verifier::verify` refuses a proof
    /// with fewer answers than queries: an honest proof on one or two rows is rejected, and the same
    /// claim on four rows is accepted.
    #[test]
    fn one_or_two_rows_answer_no_fri_query_and_four_rows_verify() {
        let circuit = ExampleId::OnePlusOne.circuit::<Stark252>().unwrap();
        let wide = Arc::new(WideAir::from_circuit(&circuit));
        let honest = ExampleId::OnePlusOne.instance::<Stark252>(InstanceKind::Honest, &[3; 32]);
        let values = circuit.evaluate_unchecked(&honest).unwrap().values;
        let row: Vec<Val> = wide.row(&values).iter().map(|value| value.0).collect();
        let public = honest.public.iter().map(|value| value.0).collect();
        let statement = Statement::new(wide, public);
        for (rows, verifies) in [(1, false), (2, false), (4, true)] {
            let proof = run_prover(&statement, &row, rows).unwrap();
            assert_eq!(proof.query_list.is_empty(), !verifies);
            let air = CircuitAir::new(rows, &statement, &options());
            let verdict = Verifier::verify(&proof, &air, &mut transcript(&statement));
            assert_eq!(verdict, verifies, "{rows} rows");
        }
    }
}
