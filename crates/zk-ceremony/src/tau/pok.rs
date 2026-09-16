//! Check ①: a Schnorr proof that whoever published `[s]G1` knows `s`.
//!
//! The prover draws `k`, publishes `R = [k]G1` and `z = k + c·s`, where
//! `c = SHA-256("zk/ceremony/v1/pok" ‖ digest of the previous string ‖ [s]G1 ‖ R)` read as a
//! little-endian integer and reduced modulo the group order, points compressed. The verifier checks
//! `[z]G1 = R + c·[s]G1`. Hashing the previous string in means a proof made for one chain fails on
//! any other, so nobody can copy someone else's turn.

use bls12_381::{G1Affine, G1Projective, Scalar};
use ff::Field;
use group::Curve;
use rand_core::{CryptoRng, RngCore};
use sha2::{Digest, Sha256};

/// Domain tag of the challenge hash.
const DOMAIN_POK: &[u8] = b"zk/ceremony/v1/pok";

/// A proof of knowledge of the secret behind a contribution's `[s]G1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnowledgeProof {
    commitment: G1Affine,
    response: Scalar,
}

impl KnowledgeProof {
    /// `R = [k]G1`, the commitment the challenge is computed over.
    pub fn commitment(&self) -> &G1Affine {
        &self.commitment
    }

    /// `z = k + c·s`, which reveals nothing about `s` because `k` is fresh and uniform.
    pub fn response(&self) -> &Scalar {
        &self.response
    }
}

/// Proves knowledge of `secret`, where `s_g1 = [secret]G1`, for the string with digest `previous`.
pub(crate) fn prove<R: RngCore + CryptoRng>(
    secret: Scalar,
    s_g1: &G1Affine,
    previous: &[u8; 32],
    rng: &mut R,
) -> KnowledgeProof {
    let nonce = Scalar::random(&mut *rng);
    let commitment = (G1Affine::generator() * nonce).to_affine();
    let challenge = challenge(previous, s_g1, &commitment);
    KnowledgeProof { commitment, response: nonce + challenge * secret }
}

/// Whether `proof` shows knowledge of the logarithm of `s_g1`, made for the string `previous`.
pub(crate) fn verify(proof: &KnowledgeProof, s_g1: &G1Affine, previous: &[u8; 32]) -> bool {
    let challenge = challenge(previous, s_g1, &proof.commitment);
    G1Affine::generator() * proof.response
        == G1Projective::from(proof.commitment) + s_g1 * challenge
}

/// The challenge `c`: 32 hash bytes widened to 64 and reduced, so every byte counts.
fn challenge(previous: &[u8; 32], s_g1: &G1Affine, commitment: &G1Affine) -> Scalar {
    let hash = Sha256::new_with_prefix(DOMAIN_POK)
        .chain_update(previous)
        .chain_update(s_g1.to_compressed())
        .chain_update(commitment.to_compressed())
        .finalize();
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&hash);
    Scalar::from_bytes_wide(&wide)
}
