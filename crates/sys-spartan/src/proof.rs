//! A Spartan proof as bytes: bincode 1, the encoding Spartan itself uses to measure its proofs.

use bincode::Options;
use libspartan::{SNARK, SNARKGens};
use zk_core::SystemError;

/// Where the flip attack lands: byte 8, the first byte of the first commitment to the witness.
///
/// bincode writes `SNARK` field by field, and its first field is the R1CS satisfiability proof, whose
/// first field is `comm_vars`, the Hyrax commitment to the witness polynomial: a `Vec` of compressed
/// Ristretto points, so an 8-byte little-endian length and then 32 bytes per point. Byte 8 is byte 0
/// of the first point, and its lowest bit is the sign of the encoded field element, which every valid
/// Ristretto encoding has clear. Flipping it leaves bytes that still deserialize but no longer encode
/// a point. The verifier reads `comm_vars` into its transcript before anything else it checks, so the
/// first sum-check round is already verified against different challenges and fails.
pub const TAMPER_OFFSET: usize = 8;

/// The options `bincode::serialize` uses (fixed-width integers, little-endian), refusing trailing
/// bytes on the way back so a proof has exactly one encoding.
fn options() -> impl Options {
    bincode::DefaultOptions::new().with_fixint_encoding().reject_trailing_bytes()
}

/// `bincode::serialize`, the call Spartan makes to report its proof lengths.
pub(crate) fn encode(proof: &SNARK) -> Result<Vec<u8>, SystemError> {
    options().serialize(proof).map_err(|e| {
        SystemError::Failed(format!("bincode serialization of the Spartan proof: {e}"))
    })
}

/// The inverse of [`encode`]; bytes that do not deserialize are reported, not raised.
pub(crate) fn decode(bytes: &[u8]) -> Result<SNARK, String> {
    options().deserialize(bytes).map_err(|e| format!("bincode could not read a Spartan SNARK: {e}"))
}

/// Serialized size of Spartan's public generators, measured the way proofs are.
pub(crate) fn generators_size(generators: &SNARKGens) -> Result<u64, SystemError> {
    options()
        .serialized_size(generators)
        .map_err(|e| SystemError::Failed(format!("bincode could not measure the generators: {e}")))
}
