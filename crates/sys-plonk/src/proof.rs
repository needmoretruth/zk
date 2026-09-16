//! lambdaworks' prover and verifier, called with the museum's inputs and classified the museum's way.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::default_types::{
    FrElement, FrField,
};
use lambdaworks_math::traits::{AsBytes, Deserializable};
use lambdaworks_plonk::prover::Proof;
use lambdaworks_plonk::setup::Witness;
use zk_core::{SystemError, Verdict};

use crate::setup::{Keys, Kzg};

/// Where the flip attack lands: byte 35, the last byte of `a(ζ)`, the proof's first element.
///
/// lambdaworks writes a proof as eight scalars and then nine G1 points, each behind a 4-byte
/// big-endian length: a scalar is 32 canonical big-endian bytes, a point its three Jacobian
/// coordinates in 48 little-endian bytes each. Byte 35 is the least significant byte of `a(ζ)`, so
/// flipping its low bit shifts the claimed evaluation by one and the proof still decodes. The
/// verifier absorbs `a(ζ)` into the transcript before drawing the batching challenge `υ`, and uses
/// it in the gate term of the linearisation and in the batched KZG opening, so it is the real
/// pairing check that fails, not the decoder.
pub const TAMPER_OFFSET: usize = LENGTH_PREFIX_BYTES + crate::field::SCALAR_BYTES - 1;

/// Every element of a lambdaworks proof is preceded by its length as a big-endian `u32`.
const LENGTH_PREFIX_BYTES: usize = 4;

/// Runs lambdaworks' five-round prover and serializes the proof with `Proof::as_bytes`.
///
/// The prover never checks the witness: a false claim still yields bytes, and the verifier turns
/// them down. An error or a panic from inside lambdaworks is reported as the prover's refusal.
pub(crate) fn create(
    keys: &Keys,
    witness: &Witness<FrField>,
    public: &[FrElement],
) -> Result<Vec<u8>, SystemError> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        keys.prover.prove(witness, public, &keys.common, &keys.vk)
    }));
    match outcome {
        Ok(Ok(proof)) => Ok(proof.as_bytes()),
        Ok(Err(error)) => Err(SystemError::Unsatisfied(format!("lambdaworks prove: {error}"))),
        Err(panic) => Err(SystemError::Unsatisfied(panic_message("prover", panic.as_ref()))),
    }
}

/// Decodes `bytes` with `Proof::deserialize` and runs `verify_with_validation`.
///
/// Bytes that do not decode, bytes that are not the canonical encoding of the proof they decode to,
/// and points outside the prime-order subgroup (lambdaworks' `VerifierError`) are malformed; a proof
/// that decodes but fails the pairing checks is rejected.
///
/// lambdaworks' decoder is lenient in three ways: it ignores bytes after the last element, reads
/// only as many leading bytes as an integer holds when a length prefix is larger, and reduces an
/// integer at or above the modulus. Each lets different bytes verify as the same proof, so the
/// adapter accepts only the one encoding `Proof::as_bytes` writes, as it does for public inputs.
pub(crate) fn check(
    keys: &Keys,
    public: &[FrElement],
    bytes: &[u8],
) -> Result<Verdict, SystemError> {
    let proof = match Proof::<FrField, Kzg>::deserialize(bytes) {
        Ok(proof) => proof,
        Err(error) => {
            return Ok(Verdict::Malformed(format!("lambdaworks Proof::deserialize: {error:?}")));
        }
    };
    if proof.as_bytes() != bytes {
        return Ok(Verdict::Malformed("not the canonical encoding of a lambdaworks proof".into()));
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        keys.verifier.verify_with_validation(&proof, public, &keys.common, &keys.vk)
    }));
    match outcome {
        Ok(Ok(true)) => Ok(Verdict::Accepted),
        Ok(Ok(false)) => Ok(Verdict::Rejected),
        Ok(Err(error)) => Ok(Verdict::Malformed(error.to_string())),
        Err(panic) => Err(SystemError::Failed(panic_message("verifier", panic.as_ref()))),
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(who: &str, payload: &(dyn Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "no message".to_string());
    format!("lambdaworks {who} panicked: {text}")
}
