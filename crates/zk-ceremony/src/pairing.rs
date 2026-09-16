//! The one pairing comparison every check in the ceremony is made of.

use bls12_381::{G1Affine, G2Affine, G2Prepared, Gt, multi_miller_loop};

/// Whether `e(a, b) = e(c, d)`.
///
/// Computed as `e(a, b) · e(−c, d) = 1` with one Miller loop and one final exponentiation, the way
/// bellman's own verifier does it.
pub(crate) fn pairings_equal(left: (&G1Affine, &G2Affine), right: (&G1Affine, &G2Affine)) -> bool {
    let negated = -right.0;
    let (b, d) = (G2Prepared::from(*left.1), G2Prepared::from(*right.1));
    multi_miller_loop(&[(left.0, &b), (&negated, &d)]).final_exponentiation() == Gt::identity()
}
