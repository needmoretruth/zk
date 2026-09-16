//! The upstream prover and verifier, called with the museum's inputs and classified the museum's way.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use bulletproofs::r1cs::{Prover, R1CSError, R1CSProof, Verifier};
use bulletproofs::{BulletproofGens, PedersenGens};
use curve25519_dalek::scalar::Scalar;
use zk_circuit::lower::r1cs::R1cs;
use zk_core::{ExampleId, SystemError, Verdict};

use crate::field::{ELEMENT_BYTES, RistrettoScalar};
use crate::synthesis::{synthesize, transcript};

/// Where the flip attack lands in the proofs this exhibit makes: byte 257, the lowest byte of `t_x`.
///
/// `R1CSProof::to_bytes` writes a version byte, then 32-byte elements: the commitments `A_I1`, `A_O1`,
/// `S1` (and `A_I2`, `A_O2`, `S2` only when a second phase exists, which the museum's circuits never
/// have), `T_1`, `T_3` … `T_6`, and then the scalars `t_x`, `t_x_blinding`, `e_blinding`, before the
/// inner-product argument. Scalars are little-endian, so byte `1 + 8·32 = 257` is the least
/// significant byte of `t_x`, the claimed evaluation of the polynomial `t(x)`; flipping its low bit
/// changes the value by one and keeps it canonical. The verifier absorbs `t_x` into the transcript
/// before drawing the challenge `w` and uses it in the final multiscalar check, so it is that
/// equation that fails, not the decoder.
pub const TAMPER_OFFSET: usize = 1 + ONE_PHASE_ELEMENTS * ELEMENT_BYTES;

/// Points before `t_x` in a one-phase proof.
const ONE_PHASE_ELEMENTS: usize = 8;

/// Points before `t_x` in a two-phase proof, whose version byte is 1.
const TWO_PHASE_ELEMENTS: usize = 11;

/// The lowest byte of `t_x`, wherever the proof's version byte puts it.
pub(crate) fn tamper_offset(proof: &[u8]) -> usize {
    match proof.first() {
        Some(1) => 1 + TWO_PHASE_ELEMENTS * ELEMENT_BYTES,
        _ => TAMPER_OFFSET,
    }
}

/// Everything proving and verifying one example share: its lowered circuit and the generators.
pub(crate) struct Statement<'a> {
    /// The example, whose ID labels the transcript.
    pub(crate) example: ExampleId,
    /// The lowered circuit both sides replay.
    pub(crate) r1cs: &'a R1cs<RistrettoScalar>,
    /// The two Pedersen generators `B` and `B̃`.
    pub(crate) pedersen: &'a PedersenGens,
    /// The vector generators `G` and `H`, one pair per multiplier.
    pub(crate) generators: &'a BulletproofGens,
}

impl Statement<'_> {
    /// Replays the circuit into `Prover` with the witness `z` and serializes with `to_bytes`.
    ///
    /// The prover never checks that the constraints hold: a false claim still yields bytes for the
    /// verifier to turn down. An error or a panic from inside the upstream crate is its refusal.
    pub(crate) fn prove(
        &self,
        public: &[Scalar],
        z: &[RistrettoScalar],
    ) -> Result<Vec<u8>, SystemError> {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut prover = Prover::new(self.pedersen, transcript(self.example, public));
            synthesize(&mut prover, self.r1cs, public, Some(z))?;
            prover.prove(self.generators)
        }));
        match outcome {
            Ok(Ok(proof)) => Ok(proof.to_bytes()),
            Ok(Err(error)) => Err(SystemError::Unsatisfied(format!("bulletproofs prove: {error}"))),
            Err(panic) => Err(SystemError::Unsatisfied(panic_message("prover", panic.as_ref()))),
        }
    }

    /// Decodes `bytes` with `R1CSProof::from_bytes`, replays the circuit into `Verifier` and verifies.
    ///
    /// Bytes that do not parse (wrong length, unknown version, a non-canonical scalar) are malformed.
    /// Upstream decompresses points only inside the final check and reports a point that does not
    /// decode as a failed verification, so such a proof is rejected rather than malformed.
    ///
    /// Upstream also reads a one-phase proof written in the two-phase layout, with identity points
    /// for the second phase, and that proof verifies. So a proof counts only in the one encoding
    /// `R1CSProof::to_bytes` writes; any other bytes are malformed, not a second valid proof.
    pub(crate) fn check(&self, public: &[Scalar], bytes: &[u8]) -> Result<Verdict, SystemError> {
        let proof = match R1CSProof::from_bytes(bytes) {
            Ok(proof) => proof,
            Err(error) => {
                return Ok(Verdict::Malformed(format!(
                    "bulletproofs R1CSProof::from_bytes: {error}"
                )));
            }
        };
        if proof.to_bytes() != bytes {
            return Ok(Verdict::Malformed(
                "not the canonical encoding of a bulletproofs proof".into(),
            ));
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut verifier = Verifier::new(transcript(self.example, public));
            synthesize(&mut verifier, self.r1cs, public, None)?;
            verifier.verify(&proof, self.pedersen, self.generators)
        }));
        match outcome {
            Ok(Ok(())) => Ok(Verdict::Accepted),
            Ok(Err(R1CSError::VerificationError)) => Ok(Verdict::Rejected),
            Ok(Err(R1CSError::FormatError)) => {
                Ok(Verdict::Malformed(R1CSError::FormatError.to_string()))
            }
            Ok(Err(error)) => Err(SystemError::Failed(format!("bulletproofs verify: {error}"))),
            Err(panic) => Err(SystemError::Failed(panic_message("verifier", panic.as_ref()))),
        }
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(role: &str, payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("bulletproofs {role} panicked: {text}"),
        (_, Some(text)) => format!("bulletproofs {role} panicked: {text}"),
        _ => format!("bulletproofs {role} panicked"),
    }
}
