//! A Nova proof as bytes: bincode 2 with its legacy configuration, the encoding nova-snark's examples
//! use for `CompressedSNARK`.

use bincode::config;
use serde::Serialize;
use zk_core::SystemError;

use crate::config::Compressed;

/// Where the flip attack lands: byte 1080, the first byte of the first coefficient of the first round
/// polynomial in the primary Spartan proof's outer sum-check.
///
/// bincode's legacy configuration writes `CompressedSNARK` field by field with no framing: a scalar
/// or a compressed Pasta point is 32 bytes, a `Vec` is an 8-byte little-endian length and its items.
/// Every field before `snark_primary` has a fixed size whatever the statement — the running and
/// random relaxed instances (two points, two scalars of public IO with their length, `u`: 168 bytes
/// each), the incoming secondary instance (104), the three folding messages (32 each), the two
/// `ri` scalars and the four derandomization blinds — which adds up to 1064. `snark_primary` then
/// opens with `sc_proof_outer`, a `Vec` of round polynomials, each a `Vec` of scalars: two lengths,
/// and at 1080 the constant term of round one.
///
/// Flipping its lowest bit moves that coefficient by one, which leaves it canonical unless it was
/// `p − 1`. The proof still decodes, the instance hashes still match, the three folds are recomputed
/// unchanged, and the verifier reaches the sum-check: the round polynomial it absorbs changes every
/// later challenge, and the final claim no longer matches the relaxed R1CS evaluation.
pub const TAMPER_OFFSET: usize = 1080;

/// `bincode::serde::encode_into_std_write(.., config::legacy())`, as the examples call it, into a
/// `Vec`. The examples then deflate the bytes to print a size; the museum keeps them as the verifier
/// reads them, so the flip attack and the secret scan see real elements.
pub(crate) fn encode(proof: &Compressed) -> Result<Vec<u8>, SystemError> {
    bincode::serde::encode_to_vec(proof, config::legacy()).map_err(|e| {
        SystemError::Failed(format!("bincode could not write the compressed SNARK: {e}"))
    })
}

/// The inverse of [`encode`], refusing bytes left over after the proof so a proof has one encoding.
///
/// halo2curves refuses a non-canonical scalar and a point that does not decompress, so every element
/// that comes back is one the verifier can compute with.
pub(crate) fn decode(bytes: &[u8]) -> Result<Compressed, String> {
    let (proof, read) = bincode::serde::decode_from_slice::<Compressed, _>(bytes, config::legacy())
        .map_err(|e| format!("bincode could not read a compressed Nova SNARK: {e}"))?;
    if read != bytes.len() {
        return Err(format!("{} bytes follow the compressed SNARK", bytes.len() - read));
    }
    Ok(proof)
}

/// Serialized length of `value` in the same encoding, counted without holding the bytes.
pub(crate) fn size<T: Serialize>(value: &T, what: &str) -> Result<u64, SystemError> {
    bincode::serde::encode_into_std_write(value, &mut std::io::sink(), config::legacy())
        .map(|written| written as u64)
        .map_err(|e| SystemError::Failed(format!("bincode could not measure {what}: {e}")))
}
