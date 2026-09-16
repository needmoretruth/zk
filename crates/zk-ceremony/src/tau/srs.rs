//! The structured reference string and its fingerprint.

use bls12_381::{G1Affine, G1Projective, G2Affine, Scalar};
use ff::Field;
use group::Curve;
use sha2::{Digest, Sha256};

/// How many G1 powers the museum's string holds: `[τ⁰]G1` to `[τ⁶³]G1`, enough to commit to a
/// polynomial of degree 63.
pub const POWERS: usize = 64;

/// How many participants the ceremony activity runs unless told otherwise.
pub const DEFAULT_PARTICIPANTS: usize = 5;

/// Domain tag of [`digest`], so a string's fingerprint can never equal another hash in the museum.
const DOMAIN_SRS: &[u8] = b"zk/ceremony/v1/srs";

/// A Powers of Tau string: `[τⁱ]G1` for `i = 0..POWERS` and `[τ]G2`.
///
/// The fields are private: a string is made only by [`Srs::initial`], by a participant's turn, or by
/// one of the cheats, so every string has [`POWERS`] entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Srs {
    g1_powers: Vec<G1Affine>,
    tau_g2: G2Affine,
}

impl Srs {
    /// The string for τ = 1: every entry is a generator. Everyone knows this τ, so the string is
    /// worthless until someone multiplies in a secret.
    pub fn initial() -> Self {
        Self { g1_powers: vec![G1Affine::generator(); POWERS], tau_g2: G2Affine::generator() }
    }

    /// `[τⁱ]G1` for `i = 0..POWERS`, what a committer multiplies polynomial coefficients by.
    pub fn g1_powers(&self) -> &[G1Affine] {
        &self.g1_powers
    }

    /// `[τ]G2`, what a verifier pairs an opening proof with.
    pub fn tau_g2(&self) -> &G2Affine {
        &self.tau_g2
    }

    /// The string for a τ someone knows outright, built from the generators.
    pub(crate) fn from_tau(tau: Scalar) -> Self {
        Self::initial().multiplied(tau)
    }

    /// Entry `i` of G1 times `sⁱ` and the G2 entry times `s`: the same string for `τ·s`.
    pub(crate) fn multiplied(&self, s: Scalar) -> Self {
        let mut factor = Scalar::ONE;
        let scaled: Vec<G1Projective> = self
            .g1_powers
            .iter()
            .map(|power| {
                let point = power * factor;
                factor *= s;
                point
            })
            .collect();
        let mut g1_powers = vec![G1Affine::identity(); scaled.len()];
        G1Projective::batch_normalize(&scaled, &mut g1_powers);
        Self { g1_powers, tau_g2: (self.tau_g2 * s).to_affine() }
    }

    /// The same string with G1 entry `index` replaced by `point`; `None` past the last entry.
    pub(crate) fn with_power(mut self, index: usize, point: G1Affine) -> Option<Self> {
        *self.g1_powers.get_mut(index)? = point;
        Some(self)
    }
}

/// A 32-byte fingerprint of a string, short enough to show and to bind a knowledge proof to.
///
/// `SHA-256("zk/ceremony/v1/srs" ‖ [τ⁰]G1 ‖ … ‖ [τ⁶³]G1 ‖ [τ]G2)`, every point in its standard
/// compressed form (48 bytes in G1, 96 in G2).
pub fn digest(srs: &Srs) -> [u8; 32] {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_SRS);
    for power in &srs.g1_powers {
        hasher.update(power.to_compressed());
    }
    hasher.update(srs.tau_g2.to_compressed());
    hasher.finalize().into()
}
