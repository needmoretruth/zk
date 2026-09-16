//! Where things sit inside a serialized Miden proof, read with Miden's own decoders.
//!
//! An `ExecutionProof` is written by `miden-core` as a format byte, the verifier roots it is
//! compatible with, a discriminant, the VM proof — the STARK proof's bytes, a hash-function tag
//! and the precompile root — and finally the precompile proof, absent for these programs. The
//! STARK proof's bytes are a wincode encoding of `StarkProofData`: per-AIR log trace heights, then
//! the transcript's field elements, then its commitments.

use core::mem::size_of;

use miden_core::Felt;
use miden_core::field::QuadFelt;
use miden_core::proof::{HashFunction, MAX_STARK_PROOF_BYTES, PrecompileProof};
use miden_core::serde::Serializable;
use miden_crypto::stark::StarkConfig;
use miden_crypto::stark::lmcs::Lmcs;
use miden_crypto::stark::proof::StarkProofData;
use miden_prover::ExecutionProof;
use miden_prover::config::Blake3Config;
use miden_serde_utils::deserialize_schema_exact;
use serde_wincode::SerdeCompat;
use serde_wincode::wincode::config::Configuration;

/// The STARK proof for the default prover, whose commitments and Fiat–Shamir hash are Blake3.
type Blake3Proof = StarkProofData<Felt, QuadFelt, Blake3Config>;

/// One commitment as the Blake3 configuration writes it: a 32-byte digest, raw.
type Commitment = <<Blake3Config as StarkConfig<Felt, QuadFelt>>::Lmcs as Lmcs>::Commitment;

/// A decoded proof and where its STARK bytes end inside the whole encoding.
struct Located {
    stark: Blake3Proof,
    stark_end: usize,
}

/// Decodes `bytes` down to the STARK proof, exactly as `miden-verifier` would.
fn locate(bytes: &[u8]) -> Option<Located> {
    let proof = ExecutionProof::read_from_bytes(bytes).ok()?;
    let vm = proof.vm();
    if proof.has_precompiles() || vm.proof.hash_fn() != HashFunction::Blake3_256 {
        return None;
    }
    let config = Configuration::default().with_preallocation_size_limit::<MAX_STARK_PROOF_BYTES>();
    let stark =
        deserialize_schema_exact::<SerdeCompat<Blake3Proof>, _>(vm.proof.bytes(), config).ok()?;
    // What follows the STARK bytes, measured with the encoder that wrote it.
    let suffix = vm.proof.hash_fn().to_bytes().len()
        + vm.precompile_root.to_bytes().len()
        + Option::<PrecompileProof>::None.to_bytes().len();
    let stark_end = bytes.len().checked_sub(suffix)?;
    Some(Located { stark, stark_end })
}

/// Where the first commitment in the STARK transcript starts: the commitment to the main trace.
///
/// The transcript's commitments are the last thing in the STARK bytes, in the order the prover
/// sent them, and the lifted STARK verifier receives the main trace commitment first.
pub(crate) fn main_trace_commitment_offset(bytes: &[u8]) -> Option<usize> {
    let located = locate(bytes)?;
    let commitments = located.stark.num_commitments().checked_mul(size_of::<Commitment>())?;
    let offset = located.stark_end.checked_sub(commitments)?;
    (located.stark.num_commitments() > 0).then_some(offset)
}

/// Rows in the tallest trace the proof covers: the VM's execution trace padded to a power of two.
pub(crate) fn trace_rows(bytes: &[u8]) -> Option<u64> {
    let located = locate(bytes)?;
    let log_height = located.stark.log_trace_heights().iter().copied().max()?;
    1u64.checked_shl(u32::from(log_height))
}
