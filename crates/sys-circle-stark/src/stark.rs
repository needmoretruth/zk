//! Calls into `p3-uni-stark` with the circle commitment: the real prover and verifier, with their
//! output in bytes.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{Proof, VerificationError};
use zk_core::{SystemError, Verdict};

use crate::air::CircuitAir;
use crate::config::Config;
use crate::field::Val;

/// Rows in the trace handed to the prover: the one circuit row, repeated.
///
/// `CirclePcs` refuses to commit to fewer than 4 rows (its own assertion: one bit is folded
/// bivariately and FRI needs one more), so the smallest trace it accepts is 4 copies of the row.
pub(crate) const TRACE_HEIGHT: usize = 4;

/// Runs `p3_uni_stark::prove` and serializes the proof with postcard, the format Plonky3's own
/// backward-compatibility fixtures use for their circle proofs.
///
/// The prover checks nothing in release builds. In builds with debug assertions (`cargo test`
/// profiles) `p3-uni-stark` first runs `check_constraints` and panics on a false witness; that
/// panic is caught and reported as the prover's refusal.
pub(crate) fn prove(
    config: &Config,
    air: &CircuitAir,
    row: &[Val],
    public: &[Val],
) -> Result<Vec<u8>, SystemError> {
    let trace = RowMajorMatrix::new(row.repeat(TRACE_HEIGHT), row.len());
    let proved = catch_unwind(AssertUnwindSafe(|| p3_uni_stark::prove(config, air, trace, public)));
    match proved {
        Ok(proof) => postcard::to_allocvec(&proof).map_err(|e| {
            SystemError::Failed(format!("postcard could not serialize the proof: {e}"))
        }),
        Err(panic) => Err(SystemError::Unsatisfied(panic_message("prove", panic.as_ref()))),
    }
}

/// Decodes `bytes` and runs `p3_uni_stark::verify`, classifying the outcome.
///
/// Bytes postcard cannot decode (including a field element at or above the modulus, which
/// Mersenne31's deserializer refuses), bytes left over after the proof, proofs whose shape the
/// verifier refuses before any cryptographic check (`InvalidProofShape`), and proofs that make the
/// verifier panic are malformed. Every other verifier error — a failed opening argument, a
/// proof-of-work or Merkle mismatch, an out-of-domain evaluation that does not match the
/// quotient — is a rejection.
pub(crate) fn verify(config: &Config, air: &CircuitAir, bytes: &[u8], public: &[Val]) -> Verdict {
    let proof = match postcard::take_from_bytes::<Proof<Config>>(bytes) {
        Ok((proof, [])) => proof,
        Ok((_, rest)) => {
            return Verdict::Malformed(format!("{} bytes follow the proof", rest.len()));
        }
        Err(e) => return Verdict::Malformed(format!("postcard could not decode the proof: {e}")),
    };
    let verified =
        catch_unwind(AssertUnwindSafe(|| p3_uni_stark::verify(config, air, &proof, public)));
    match verified {
        Ok(Ok(())) => Verdict::Accepted,
        Ok(Err(VerificationError::InvalidProofShape(shape))) => {
            Verdict::Malformed(shape.to_string())
        }
        Ok(Err(_)) => Verdict::Rejected,
        Err(panic) => Verdict::Malformed(panic_message("verify", panic.as_ref())),
    }
}

/// Where the first opened trace value starts in a serialized proof, if the bytes decode.
///
/// postcard writes a struct as its fields in order with no framing, and `Proof` begins with the
/// commitments followed by `opened_values.trace_local`, a length-prefixed vector. Re-serializing
/// the decoded commitments and the length prefix measures the offset with the upstream encoder
/// itself instead of a hand-written layout.
pub(crate) fn trace_opening_offset(bytes: &[u8]) -> Option<usize> {
    let proof: Proof<Config> = postcard::from_bytes(bytes).ok()?;
    let commitments = postcard::to_allocvec(&proof.commitments).ok()?.len();
    let prefix = postcard::to_allocvec(&proof.opened_values.trace_local.len()).ok()?.len();
    let offset = commitments + prefix;
    (offset < bytes.len()).then_some(offset)
}

/// The text a panic carried, when it carried text.
fn panic_message(step: &str, payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("p3_uni_stark::{step} panicked: {text}"),
        (_, Some(text)) => format!("p3_uni_stark::{step} panicked: {text}"),
        _ => format!("p3_uni_stark::{step} panicked"),
    }
}
