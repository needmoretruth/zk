//! Part C: KZG commitments over the ceremony's final string, and what knowing its τ allows.
//!
//! A commitment to `p(X) = Σ aᵢ·Xⁱ` is `C = Σ aᵢ·[τⁱ]G1 = [p(τ)]G1`. To show `p(z) = y`, the
//! committer sends `π = [q(τ)]G1` for `q(X) = (p(X) − y)/(X − z)`, a polynomial only when `p(z) = y`.
//! The verifier checks `e(C − y·G1, G2) = e(π, [τ]G2 − z·G2)`, the identity
//! `p(τ) − y = q(τ)·(τ − z)` in the exponent. Whoever knows τ skips the polynomial:
//! `π′ = (τ − z)⁻¹·(C − y′·G1)` satisfies the check for any `y′`.

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use ff::Field;
use group::Curve;

use crate::pairing::pairings_equal;
use crate::tau::{KeptSecret, Srs};

/// Why a polynomial could not be committed to or opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KzgError {
    /// The polynomial has more coefficients than the string has G1 powers.
    TooManyCoefficients {
        /// Coefficients given.
        given: usize,
        /// G1 powers in the string.
        powers: usize,
    },
}

/// `C = Σ aᵢ·[τⁱ]G1` for the coefficients `aᵢ`, lowest degree first.
pub fn commit(srs: &Srs, coefficients: &[Scalar]) -> Result<G1Affine, KzgError> {
    fits(srs, coefficients)?;
    let sum = coefficients
        .iter()
        .zip(srs.g1_powers())
        .fold(G1Projective::identity(), |sum, (coefficient, power)| sum + power * coefficient);
    Ok(sum.to_affine())
}

/// The honest opening at `z`: `y = p(z)` and `π = [q(τ)]G1` with `q(X) = (p(X) − y)/(X − z)`.
pub fn open(srs: &Srs, coefficients: &[Scalar], z: Scalar) -> Result<(Scalar, G1Affine), KzgError> {
    fits(srs, coefficients)?;
    let (quotient, value) = divide_by_linear(coefficients, z);
    Ok((value, commit(srs, &quotient)?))
}

/// Refuses a polynomial of higher degree than the string can commit to.
fn fits(srs: &Srs, coefficients: &[Scalar]) -> Result<(), KzgError> {
    let powers = srs.g1_powers().len();
    if coefficients.len() > powers {
        return Err(KzgError::TooManyCoefficients { given: coefficients.len(), powers });
    }
    Ok(())
}

/// Whether `e(C − y·G1, G2) = e(π, [τ]G2 − z·G2)`: the ordinary KZG verifier, which knows nothing
/// but the string.
pub fn verify(srs: &Srs, commitment: &G1Affine, z: Scalar, y: Scalar, proof: &G1Affine) -> bool {
    let lifted = (G1Projective::from(commitment) - G1Affine::generator() * y).to_affine();
    let shifted = (G2Projective::from(srs.tau_g2()) - G2Affine::generator() * z).to_affine();
    pairings_equal((&lifted, &G2Affine::generator()), (proof, &shifted))
}

/// Synthetic division by `X − z`: the quotient's coefficients and the remainder `p(z)`.
fn divide_by_linear(coefficients: &[Scalar], z: Scalar) -> (Vec<Scalar>, Scalar) {
    let mut quotient = vec![Scalar::ZERO; coefficients.len().saturating_sub(1)];
    let mut running = Scalar::ZERO;
    for (degree, coefficient) in coefficients.iter().enumerate().rev() {
        running = running * z + coefficient;
        if let Some(slot) = degree.checked_sub(1).and_then(|below| quotient.get_mut(below)) {
            *slot = running;
        }
    }
    (quotient, running)
}

/// What a group of participants who all kept their secrets can compute together: a τ.
///
/// Built only from [`KeptSecret`]s, which only a turn taken with
/// [`crate::tau::Participant::contribute_keeping_secret`] returns. If the group holds every
/// participant's secret, the τ is the final string's; with one missing it is some other number and
/// every forgery made with it fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Collusion {
    tau: Scalar,
}

impl Collusion {
    /// `τ = Π sᵢ`: every turn multiplied τ by one secret, starting from 1, in any order.
    pub fn from_secrets(secrets: &[KeptSecret]) -> Self {
        Self { tau: secrets.iter().fold(Scalar::ONE, |product, secret| product * secret.0) }
    }

    /// `π′ = (τ − z)⁻¹·(C − y′·G1)`: an opening of `commitment` at `z` to `claimed`, whatever the
    /// committed polynomial is.
    ///
    /// `None` when `z` equals the group's τ, which has no inverse to divide by.
    pub fn forge_open(
        &self,
        commitment: &G1Affine,
        z: Scalar,
        claimed: Scalar,
    ) -> Option<G1Affine> {
        let inverse = Option::<Scalar>::from((self.tau - z).invert())?;
        let lifted = G1Projective::from(commitment) - G1Affine::generator() * claimed;
        Some((lifted * inverse).to_affine())
    }
}
