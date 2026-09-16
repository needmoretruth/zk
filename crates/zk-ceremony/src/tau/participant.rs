//! One participant's turn: multiply a secret into the string and publish the record of it.

use bls12_381::{G1Affine, G2Affine, Scalar};
use ff::Field;
use group::Curve;
use rand_core::{CryptoRng, RngCore};

use crate::tau::pok::{self, KnowledgeProof};
use crate::tau::srs::{Srs, digest};

/// What a participant publishes: `[s]G1`, `[s]G2` and a proof of knowing `s`.
///
/// It says nothing about `s` beyond what the points do, and it is all a verifier needs to check
/// that the new string is the old one with `s` multiplied in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contribution {
    pub(crate) s_g1: G1Affine,
    pub(crate) s_g2: G2Affine,
    pub(crate) proof: KnowledgeProof,
}

impl Contribution {
    /// `[s]G1`, which check ① proves knowledge of and check ② compares with `[s]G2`.
    pub fn s_g1(&self) -> &G1Affine {
        &self.s_g1
    }

    /// `[s]G2`, which check ③ uses to confirm the old `[τ]G1` was multiplied by `s`.
    pub fn s_g2(&self) -> &G2Affine {
        &self.s_g2
    }

    /// The Schnorr proof of check ①.
    pub fn proof(&self) -> &KnowledgeProof {
        &self.proof
    }
}

/// A secret its participant chose to keep, returned only by
/// [`Participant::contribute_keeping_secret`].
///
/// It has no public constructor, so [`crate::kzg::Collusion`] can only be assembled from turns in
/// which people kept their secrets; a turn made with [`Participant::contribute`] leaves nothing to
/// collude with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptSecret(pub(crate) Scalar);

/// The role one person plays in a ceremony.
///
/// It has no fields: an honest participant holds nothing once their turn is over.
#[derive(Clone, Copy, Debug, Default)]
pub struct Participant;

impl Participant {
    /// An honest turn: draws a non-zero secret `s`, returns the string for `τ·s` and the record.
    ///
    /// `s` lives in this call's local variables only and is not stored anywhere once it returns.
    pub fn contribute<R: RngCore + CryptoRng>(prev: &Srs, rng: &mut R) -> (Srs, Contribution) {
        let secret = nonzero_scalar(rng);
        (prev.multiplied(secret), record(prev, secret, rng))
    }

    /// The same turn by someone who keeps `s`: the only way a secret leaves a turn.
    pub fn contribute_keeping_secret<R: RngCore + CryptoRng>(
        prev: &Srs,
        rng: &mut R,
    ) -> (Srs, Contribution, KeptSecret) {
        let secret = nonzero_scalar(rng);
        (prev.multiplied(secret), record(prev, secret, rng), KeptSecret(secret))
    }
}

/// The published record of multiplying `secret` into `prev`.
pub(crate) fn record<R: RngCore + CryptoRng>(
    prev: &Srs,
    secret: Scalar,
    rng: &mut R,
) -> Contribution {
    let s_g1 = (G1Affine::generator() * secret).to_affine();
    let s_g2 = (G2Affine::generator() * secret).to_affine();
    let proof = pok::prove(secret, &s_g1, &digest(prev), rng);
    Contribution { s_g1, s_g2, proof }
}

/// A uniformly random secret other than zero: multiplying by zero would set τ to a value everyone
/// knows.
pub(crate) fn nonzero_scalar<R: RngCore + CryptoRng>(rng: &mut R) -> Scalar {
    loop {
        let candidate = Scalar::random(&mut *rng);
        if !bool::from(candidate.is_zero()) {
            return candidate;
        }
    }
}
