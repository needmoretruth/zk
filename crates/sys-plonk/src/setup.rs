//! PLONK's two-phase setup: a universal KZG reference string, then preprocessing for one circuit.
//!
//! The reference string is made here, in this process, by one party: a fresh random `τ` is drawn,
//! `[τ^i]G₁` and `[τ]G₂` are computed, and `τ` goes out of scope when [`universal_srs`] returns. It is
//! never stored or serialized, but its memory is not wiped either, and nobody else took part. A real
//! deployment uses a multi-party ceremony, where one honest participant is enough; this one-person
//! setup is only as trustworthy as this machine at this moment.

use lambdaworks_crypto::commitments::kzg::{KateZaveruchaGoldberg, StructuredReferenceString};
use lambdaworks_math::cyclic_group::IsGroup;
use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::curve::BLS12381Curve;
use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::default_types::{
    FrElement, FrField,
};
use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::pairing::BLS12381AtePairing;
use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::twist::BLS12381TwistCurve;
use lambdaworks_math::elliptic_curve::short_weierstrass::point::ShortWeierstrassJacobianPoint;
use lambdaworks_math::elliptic_curve::traits::IsEllipticCurve;
use lambdaworks_math::traits::{AsBytes, IsRandomFieldElementGenerator};
use lambdaworks_plonk::prover::Prover;
use lambdaworks_plonk::setup::{CommonPreprocessedInput, VerificationKey, setup};
use lambdaworks_plonk::srs::SRSManager;
use lambdaworks_plonk::test_utils::utils::SecureRandomFieldGenerator;
use lambdaworks_plonk::verifier::Verifier;
use zk_circuit::lower::plonkish::Plonkish;
use zk_core::{Control, SystemError};

use crate::field::Fr;
use crate::layout;

/// KZG over BLS12-381, the commitment scheme PLONK's paper instantiates.
pub(crate) type Kzg = KateZaveruchaGoldberg<FrField, BLS12381AtePairing>;
/// A G1 point: every commitment in a proof and in the verifying key.
pub(crate) type G1 = ShortWeierstrassJacobianPoint<BLS12381Curve>;
type G2 = ShortWeierstrassJacobianPoint<BLS12381TwistCurve>;

/// How many powers of `τ` to compute between two cancellation checks.
const POWERS_PER_CHECKPOINT: usize = 1024;

/// Everything proving and verifying one statement needs.
pub(crate) struct Keys {
    /// Selector and permutation polynomials, domain and coset generator; both sides read it.
    pub(crate) common: CommonPreprocessedInput<FrField>,
    /// Commitments to the selector and permutation polynomials.
    pub(crate) vk: VerificationKey<G1>,
    /// lambdaworks' prover over the reference string, blinding with OS randomness.
    pub(crate) prover: Prover<FrField, Kzg, SecureRandomFieldGenerator>,
    /// lambdaworks' verifier over the same reference string.
    pub(crate) verifier: Verifier<FrField, Kzg>,
    /// `SRSManager::to_bytes` length of the reference string.
    pub(crate) srs_bytes: u64,
    /// `VerificationKey::as_bytes` length.
    pub(crate) vk_bytes: u64,
}

/// Preprocesses `table`, makes a reference string just large enough for it, and commits to the
/// circuit's polynomials to obtain the verifying key.
pub(crate) fn generate(table: &Plonkish<Fr>, control: &Control) -> Result<Keys, SystemError> {
    let common = layout::preprocess(table)?;
    control.checkpoint()?;
    // A blinded wire polynomial has `n + 2` coefficients and `z` and the quotient parts `n + 3`, so
    // `n + 3` powers of `τ` in G1 commit to every polynomial the prover makes for this domain.
    let srs = universal_srs(common.n + 3, control)?;
    let srs_bytes = SRSManager::to_bytes(&srs).len() as u64;
    control.checkpoint()?;
    let kzg = Kzg::new(srs);
    let vk = setup::<FrField, Kzg>(&common, &kzg);
    let vk_bytes = vk.as_bytes().len() as u64;
    let prover = Prover::new(kzg.clone(), SecureRandomFieldGenerator);
    let verifier = Verifier::new(kzg);
    Ok(Keys { common, vk, prover, verifier, srs_bytes, vk_bytes })
}

/// `[1, τ, τ², …, τ^(powers−1)]·G₁` and `[1, τ]·G₂` for a fresh random `τ`, the KZG reference string.
///
/// `τ` comes from lambdaworks' `SecureRandomFieldGenerator`, the operating-system randomness its
/// prover blinds with, and is dropped on return (see the module documentation).
fn universal_srs(
    powers: usize,
    control: &Control,
) -> Result<StructuredReferenceString<G1, G2>, SystemError> {
    let tau: FrElement = SecureRandomFieldGenerator.generate();
    let g1 = BLS12381Curve::generator();
    let g2 = BLS12381TwistCurve::generator();
    let mut powers_main_group = Vec::with_capacity(powers);
    let mut tau_power = FrElement::one();
    for index in 0..powers {
        if index % POWERS_PER_CHECKPOINT == 0 {
            control.checkpoint()?;
        }
        powers_main_group.push(g1.operate_with_self(tau_power.canonical()));
        tau_power *= &tau;
    }
    let powers_secondary_group = [g2.clone(), g2.operate_with_self(tau.canonical())];
    Ok(StructuredReferenceString::new(&powers_main_group, &powers_secondary_group))
}
