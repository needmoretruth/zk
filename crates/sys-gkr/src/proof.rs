//! A Remainder proof as bytes: bincode 1, the encoding Remainder's own provers write proofs in.

use bincode::Options;
use hyrax::gkr::HyraxProof;
use shared_types::Bn256Point;
use shared_types::config::ProofConfig;
use shared_types::pedersen::PedersenCommitter;
use zk_core::SystemError;

use crate::field::ELEMENT_BYTES;

/// What the prover sends: the Hyrax proof and the configuration it was made under, which the
/// verifier compares with its own before reading anything else.
pub(crate) type Sent = (HyraxProof<Bn256Point>, ProofConfig);

/// Bytes before the first element of a bincode `Vec`: its length as a `u64`.
const LENGTH_BYTES: usize = 8;
/// Bytes in a serialized `LayerId`: a `u32` variant and a `u64` index.
const LAYER_ID_BYTES: usize = 12;

/// The options `bincode::serialize` uses (fixed-width integers, little-endian), refusing trailing
/// bytes and any length the input cannot hold on the way back.
fn options() -> impl Options {
    bincode::DefaultOptions::new().with_fixint_encoding().reject_trailing_bytes()
}

/// `bincode::serialize`, the call Remainder's World ID provers make to write their proofs.
pub(crate) fn encode(sent: &Sent) -> Result<Vec<u8>, SystemError> {
    options()
        .serialize(sent)
        .map_err(|e| SystemError::Failed(format!("bincode serialization of the proof: {e}")))
}

/// The inverse of [`encode`]; bytes that do not deserialize are reported, not raised.
pub(crate) fn decode(bytes: &[u8]) -> Result<Sent, String> {
    let limit = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    options()
        .with_limit(limit)
        .deserialize(bytes)
        .map_err(|e| format!("bincode could not read a Hyrax proof: {e}"))
}

/// Where the flip attack lands: the first byte of the commitment to the first sum-check message of
/// the first layer proof, or the middle of the proof if it does not decode.
///
/// bincode writes the proof field by field. First come the public input tables, which the verifier
/// compares with its own; then the circuit proof, whose first field is the list of layer proofs.
/// Each entry is a `LayerId` (a 4-byte variant and an 8-byte index) and a layer proof, whose first
/// field is the proof of sum-check: the commitment to the claimed sum (32 bytes), then the list of
/// Pedersen commitments to the round messages, each a 32-byte compressed point. The first layer
/// proof belongs to the output's gate layer, the first layer the verifier checks. Flipping the lowest
/// bit of a compressed point's first byte changes its x-coordinate: either no point has it and the
/// proof no longer decodes, or it names a different point, the transcript's challenges change, and
/// the proof of dot product that ties the messages together fails.
pub(crate) fn tamper_offset(bytes: &[u8]) -> usize {
    let Ok((proof, _)) = decode(bytes) else { return bytes.len() / 2 };
    let Ok(public) = options().serialized_size(&proof.public_inputs) else {
        return bytes.len() / 2;
    };
    let offset = usize::try_from(public)
        .unwrap_or(usize::MAX)
        .saturating_add(LENGTH_BYTES + LAYER_ID_BYTES + ELEMENT_BYTES + LENGTH_BYTES);
    let has_message = proof
        .circuit_proof
        .layer_proofs
        .first()
        .is_some_and(|(_, layer)| !layer.proof_of_sumcheck.messages.is_empty());
    if has_message && offset.saturating_add(ELEMENT_BYTES) <= bytes.len() {
        offset
    } else {
        bytes.len() / 2
    }
}

/// Serialized size of the public Pedersen generators, measured the way proofs are. The precomputed
/// doublings Remainder keeps beside them are a cache, not setup material, and are not counted.
pub(crate) fn generators_size(
    committer: &PedersenCommitter<Bn256Point>,
) -> Result<u64, SystemError> {
    options()
        .serialized_size(&(&committer.generators, &committer.blinding_generator))
        .map_err(|e| SystemError::Failed(format!("bincode could not measure the generators: {e}")))
}
