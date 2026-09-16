//! The two ways a participant can try to cheat, as functions the checks must catch.

use bls12_381::G1Affine;
use group::Curve;
use rand_core::{CryptoRng, RngCore};

use crate::tau::participant::{Contribution, Participant, nonzero_scalar, record};
use crate::tau::srs::Srs;

/// A participant who throws away every earlier turn and starts over from a τ′ of their own.
///
/// If accepted, they alone would know the final τ. Their record is honest for τ′ (a real knowledge
/// proof bound to the previous string, `[τ′]G1` and `[τ′]G2`), and the new string really is a
/// sequence of powers, so checks ①, ② and ④ pass. Check ③ fails: the new `[τ]G1` is not the old
/// one multiplied by the published secret.
///
/// Against [`Srs::initial`] this is an honest turn, because τ = 1 leaves nothing to throw away; the
/// cheat only means something after another participant has contributed.
pub fn ignore_previous<R: RngCore + CryptoRng>(prev: &Srs, rng: &mut R) -> (Srs, Contribution) {
    let own_tau = nonzero_scalar(rng);
    (Srs::from_tau(own_tau), record(prev, own_tau, rng))
}

/// A participant who takes an honest turn, then swaps G1 entry `index` for a random point.
///
/// The record is honest, so checks ① and ② pass. Which check fails depends on the entry: entry 0
/// must be the generator ([`crate::tau::Check::WellFormed`]), entry 1 is `[τ]G1` itself (check ③),
/// and any later entry breaks the sequence of powers without touching ③ (check ④). `None` when
/// `index` is not below [`crate::tau::POWERS`].
pub fn replace_power<R: RngCore + CryptoRng>(
    prev: &Srs,
    index: usize,
    rng: &mut R,
) -> Option<(Srs, Contribution)> {
    let (next, contribution) = Participant::contribute(prev, rng);
    let stranger = (G1Affine::generator() * nonzero_scalar(rng)).to_affine();
    Some((next.with_power(index, stranger)?, contribution))
}
