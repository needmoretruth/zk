//! UltraPlonk's two-phase setup: a universal KZG reference string, then preprocessing for one circuit.
//!
//! jellyfish's `universal_setup` is left unimplemented upstream ("should load from files in
//! practice"); the one that runs is `universal_setup_for_testing`, which jellyfish only compiles
//! behind its `test-srs` feature. It takes the caller's random number generator and draws the
//! trapdoor `β` and the generators from it. Here that generator is ChaCha20 seeded with 32 bytes of
//! operating-system randomness, fresh for every setup, so no two runs share a trapdoor.
//!
//! It is still a one-person setup: `β`, and the seed it was drawn from, exist in this process's
//! memory until setup returns, neither is wiped, and nobody else contributed. Anyone who could read that memory could
//! forge proofs for that run. PLONK-family reference strings are updatable exactly so a real
//! deployment can use a ceremony of many participants instead.

use ark_serialize::{CanonicalSerialize, Compress};
use jf_plonk::proof_system::structs::{ProvingKey, VerifyingKey};
use jf_plonk::proof_system::{PlonkKzgSnark, UniversalSNARK};
use jf_relation::{Arithmetization, PlonkCircuit};
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use zk_core::{Control, SystemError};

/// The pairing-friendly curve the KZG commitments live on.
pub(crate) type Bn254 = ark_bn254::Bn254;
/// jellyfish's PLONK with KZG commitments over BN254.
pub(crate) type Snark = PlonkKzgSnark<Bn254>;

/// Everything proving and verifying one statement needs.
pub(crate) struct Keys {
    /// Selector, permutation and lookup-table polynomials plus the commitment key.
    pub(crate) proving: ProvingKey<Bn254>,
    /// Commitments to those polynomials and the opening key.
    pub(crate) verifying: VerifyingKey<Bn254>,
    /// Compressed `CanonicalSerialize` size of both keys.
    pub(crate) bytes: u64,
}

/// Draws a reference string just large enough for `circuit` and preprocesses the circuit with it.
pub(crate) fn generate(
    circuit: &PlonkCircuit<ark_bn254::Fr>,
    control: &Control,
) -> Result<Keys, SystemError> {
    // `srs_size` is the domain size plus the two extra degrees the prover's blinding needs.
    let degree = circuit.srs_size().map_err(|e| SystemError::Failed(format!("jellyfish: {e}")))?;
    let mut rng = os_seeded_rng()?;
    let srs = Snark::universal_setup_for_testing(degree, &mut rng)
        .map_err(|e| SystemError::Failed(format!("jellyfish universal setup: {e}")))?;
    control.checkpoint()?;
    let (proving, verifying) = Snark::preprocess(&srs, circuit)
        .map_err(|e| SystemError::Failed(format!("jellyfish preprocess: {e}")))?;
    let bytes = proving.serialized_size(Compress::Yes) + verifying.serialized_size(Compress::Yes);
    Ok(Keys { proving, verifying, bytes: bytes as u64 })
}

/// ChaCha20 seeded from the operating system, the generator jellyfish's setup and prover draw from.
pub(crate) fn os_seeded_rng() -> Result<ChaCha20Rng, SystemError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)
        .map_err(|e| SystemError::Failed(format!("no operating-system randomness: {e}")))?;
    Ok(ChaCha20Rng::from_seed(seed))
}
