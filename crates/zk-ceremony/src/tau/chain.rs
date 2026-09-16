//! Anyone's check of a whole ceremony: every turn must be a multiplication by a secret its
//! participant knows.

use bls12_381::{G1Affine, G1Projective, G2Affine, Scalar};
use ff::Field;
use group::Curve;
use rand_core::OsRng;

use crate::pairing::pairings_equal;
use crate::tau::participant::Contribution;
use crate::tau::pok;
use crate::tau::srs::{POWERS, Srs, digest};

/// One of the verifier's checks on a turn, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Check {
    /// Before ①: the new string has [`POWERS`] entries, its first is the G1 generator, and `[s]G1`
    /// is not the identity. Without it a secret of zero passes ①–④ and sets τ to 0, which everyone
    /// knows, and a string scaled by a constant passes too.
    WellFormed,
    /// ① The Schnorr proof: the participant knows `s`, and made the proof for this chain's previous
    /// string.
    KnowledgeProof,
    /// ② `e([s]G1, G2) = e(G1, [s]G2)`: both published points carry the same `s`.
    SameSecret,
    /// ③ `e(new [τ]G1, G2) = e(old [τ]G1, [s]G2)`: the new τ is the old τ times `s`.
    BuildsOnPrevious,
    /// ④ `e(Σ rᵢ·[τⁱ⁺¹]G1, G2) = e(Σ rᵢ·[τⁱ]G1, [τ]G2)` for fresh random `rᵢ`: every entry is the one
    /// before it times the same τ that the G2 entry holds.
    ConsistentPowers,
}

/// The first turn that failed, and the check it failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChainError {
    /// Zero-based position of the turn in the slice given to [`verify_chain`].
    pub step: usize,
    /// The first check that turn failed.
    pub check: Check,
}

/// Checks every turn of a ceremony that started from `initial`, in order, and stops at the first
/// failure.
///
/// Each step is a participant's [`Contribution`] with the string they published after it. The
/// random coefficients of check ④ come from the operating system at the moment of checking, after
/// every string is fixed, so no participant can know them in advance.
pub fn verify_chain(initial: &Srs, steps: &[(Contribution, Srs)]) -> Result<(), ChainError> {
    let mut previous = initial;
    for (step, (contribution, next)) in steps.iter().enumerate() {
        verify_step(previous, contribution, next).map_err(|check| ChainError { step, check })?;
        previous = next;
    }
    Ok(())
}

/// Runs the checks on one turn in [`Check`] order.
fn verify_step(previous: &Srs, contribution: &Contribution, next: &Srs) -> Result<(), Check> {
    let (Some(old_tau_g1), Some(new_tau_g1)) =
        (previous.g1_powers().get(1), next.g1_powers().get(1))
    else {
        return Err(Check::WellFormed);
    };
    if !well_formed(contribution, next) {
        return Err(Check::WellFormed);
    }
    if !pok::verify(&contribution.proof, &contribution.s_g1, &digest(previous)) {
        return Err(Check::KnowledgeProof);
    }
    let (g1, g2) = (G1Affine::generator(), G2Affine::generator());
    if !pairings_equal((&contribution.s_g1, &g2), (&g1, &contribution.s_g2)) {
        return Err(Check::SameSecret);
    }
    if !pairings_equal((new_tau_g1, &g2), (old_tau_g1, &contribution.s_g2)) {
        return Err(Check::BuildsOnPrevious);
    }
    if !consistent_powers(next) {
        return Err(Check::ConsistentPowers);
    }
    Ok(())
}

/// The shape a turn must have before any pairing is worth computing.
fn well_formed(contribution: &Contribution, next: &Srs) -> bool {
    next.g1_powers().len() == POWERS
        && next.g1_powers().first() == Some(&G1Affine::generator())
        && !bool::from(contribution.s_g1.is_identity())
}

/// Check ④ with one fresh random coefficient per adjacent pair of entries.
///
/// If entry `i + 1` differs from τ times entry `i` by an error `eᵢ` (in the exponent), the two
/// sides differ by `Σ rᵢ·eᵢ`. With any `eᵢ ≠ 0`, that sum is zero for at most one value of `rᵢ` out
/// of the group order (about 2²⁵⁵) once the others are drawn, so a string that is not a sequence of
/// powers passes with probability at most 2⁻²⁵⁴.
fn consistent_powers(srs: &Srs) -> bool {
    let mut shifted = G1Projective::identity();
    let mut base = G1Projective::identity();
    for pair in srs.g1_powers().windows(2) {
        let coefficient = Scalar::random(OsRng);
        shifted += pair[1] * coefficient;
        base += pair[0] * coefficient;
    }
    let g2 = G2Affine::generator();
    pairings_equal((&shifted.to_affine(), &g2), (&base.to_affine(), srs.tau_g2()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use rand_core::OsRng;

    use super::*;
    use crate::tau::participant::{Participant, nonzero_scalar, record};

    #[test]
    fn a_secret_of_zero_is_not_well_formed() {
        let initial = Srs::initial();
        let (first, contribution) = Participant::contribute(&initial, &mut OsRng);
        let zero = record(&first, Scalar::ZERO, &mut OsRng);
        let steps = [(contribution, first.clone()), (zero, first.multiplied(Scalar::ZERO))];
        let error = verify_chain(&initial, &steps).unwrap_err();
        assert_eq!(error, ChainError { step: 1, check: Check::WellFormed });
    }

    #[test]
    fn halves_from_two_different_secrets_fail_check_two() {
        let initial = Srs::initial();
        let (next, honest) = Participant::contribute(&initial, &mut OsRng);
        let other = record(&initial, nonzero_scalar(&mut OsRng), &mut OsRng);
        let mixed = Contribution { s_g2: other.s_g2, ..honest };
        let error = verify_chain(&initial, &[(mixed, next)]).unwrap_err();
        assert_eq!(error, ChainError { step: 0, check: Check::SameSecret });
    }
}
