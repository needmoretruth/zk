//! jellyfish's prover and verifier, called with the museum's inputs and classified the museum's way.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_plonk::PlonkError;
use jf_plonk::proof_system::UniversalSNARK;
use jf_plonk::proof_system::structs::Proof;
use jf_plonk::transcript::StandardTranscript;
use jf_relation::PlonkCircuit;
use zk_core::{SystemError, Verdict};

use crate::setup::{Bn254, Keys, Snark, os_seeded_rng};

/// Where the flip attack lands: byte 504, the least significant byte of `w₀(ζ)`.
///
/// jellyfish's `Proof` is written with arkworks' compressed `CanonicalSerialize`, field by field:
/// six wire commitments (a vector: an 8-byte length, then 32 bytes per BN254 G1 point), the
/// permutation commitment, six split-quotient commitments, the two opening proofs, and then the
/// evaluations at `ζ`, whose first vector holds the six wire evaluations as 32 little-endian bytes
/// each. `8 + 6·32 + 32 + 8 + 6·32 + 32 + 32 + 8 = 504` is the first byte of `w₀(ζ)`. Flipping its low
/// bit moves the claimed evaluation by one and the scalar stays canonical, so the proof still
/// decodes; the verifier absorbs `w₀(ζ)` into the transcript before drawing the batching challenge
/// and uses it in the linearisation, so the pairing check itself fails, not the decoder.
pub const TAMPER_OFFSET: usize = 504;

/// Proves with jellyfish's `PlonkKzgSnark::prove` over a Merlin transcript and serializes the proof.
///
/// The adapter does not check the witness. jellyfish's prover does not either, but in round 3
/// (`Prover::split_quotient_polynomial`) it refuses a quotient whose degree is not the one a
/// satisfied circuit yields, `SnarkError::WrongQuotientPolyDegree`; that refusal, any other error
/// and a panic are all the prover's, and are reported as [`SystemError::Unsatisfied`].
pub(crate) fn create(
    keys: &Keys,
    circuit: &PlonkCircuit<ark_bn254::Fr>,
) -> Result<Vec<u8>, SystemError> {
    let mut rng = os_seeded_rng()?;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        Snark::prove::<_, _, StandardTranscript>(&mut rng, circuit, &keys.proving, None)
    }));
    let proof = match outcome {
        Ok(Ok(proof)) => proof,
        Ok(Err(error)) => {
            return Err(SystemError::Unsatisfied(format!("jellyfish prove: {error}")));
        }
        Err(panic) => {
            return Err(SystemError::Unsatisfied(panic_message("prover", panic.as_ref())));
        }
    };
    let mut bytes = Vec::with_capacity(proof.compressed_size());
    proof
        .serialize_compressed(&mut bytes)
        .map_err(|e| SystemError::Failed(format!("serializing the jellyfish proof: {e}")))?;
    Ok(bytes)
}

/// Decodes `bytes` and runs jellyfish's `PlonkKzgSnark::verify`.
///
/// arkworks' checked decoder refuses scalars at or above the modulus and points off the curve or
/// outside the prime-order subgroup, but reads a `&[u8]` without complaining about bytes left over.
/// Those, and any other encoding that is not the one `serialize_compressed` writes, are malformed.
/// `PlonkError::WrongProof` is the pairing check failing, a rejection; the verifier's other errors
/// are about the proof's shape (a missing lookup part, wrong vector lengths) and count as malformed.
pub(crate) fn check(
    keys: &Keys,
    public: &[ark_bn254::Fr],
    bytes: &[u8],
) -> Result<Verdict, SystemError> {
    let proof = match Proof::<Bn254>::deserialize_compressed(bytes) {
        Ok(proof) => proof,
        Err(error) => {
            return Ok(Verdict::Malformed(format!(
                "jellyfish Proof::deserialize_compressed: {error}"
            )));
        }
    };
    let mut canonical = Vec::with_capacity(bytes.len());
    if proof.serialize_compressed(&mut canonical).is_err() || canonical != bytes {
        return Ok(Verdict::Malformed("not the canonical encoding of a jellyfish proof".into()));
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        Snark::verify::<StandardTranscript>(&keys.verifying, public, &proof, None)
    }));
    match outcome {
        Ok(Ok(())) => Ok(Verdict::Accepted),
        Ok(Err(PlonkError::WrongProof)) => Ok(Verdict::Rejected),
        Ok(Err(error)) => Ok(Verdict::Malformed(format!("jellyfish verify: {error}"))),
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
    format!("jellyfish {who} panicked: {text}")
}
