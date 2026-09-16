//! A value as the verifier can rebuild it: published commitments combined with known numbers.

use ff::Field as _;

use crate::field::PallasScalar;
use crate::pedersen::{Generators, Opening};
use crate::wrap::PallasPoint;

/// `Σ coefficient·(committed value) + known`, where the known part comes from public inputs and
/// constants.
///
/// Pedersen commitments are additively homomorphic, so a linear gate needs no proof: the same
/// combination of the commitments commits to the combined value under the combined blinding, and
/// prover and verifier both compute it from this description.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Affine {
    /// `(commitment index, coefficient)`, sorted by index, without repeats or zero coefficients.
    terms: Vec<(usize, PallasScalar)>,
    /// The part every verifier knows.
    known: PallasScalar,
}

impl Affine {
    /// A number the verifier knows: a public input or a constant, `known·G` with blinding 0.
    pub(crate) fn known(value: PallasScalar) -> Self {
        Self { terms: Vec::new(), known: value }
    }

    /// Exactly the value behind commitment `index`.
    pub(crate) fn committed(index: usize) -> Self {
        Self { terms: vec![(index, PallasScalar::ONE)], known: PallasScalar::ZERO }
    }

    /// The value itself when no commitment contributes to it.
    pub(crate) fn known_value(&self) -> Option<PallasScalar> {
        self.terms.is_empty().then_some(self.known)
    }

    /// `Σ weight·part + constant`, merged so a commitment appears once.
    pub(crate) fn combine<'a>(
        parts: impl IntoIterator<Item = (&'a Affine, PallasScalar)>,
        constant: PallasScalar,
    ) -> Self {
        let mut terms = Vec::new();
        let mut known = constant;
        for (part, weight) in parts {
            terms.extend(
                part.terms.iter().map(|(index, coefficient)| (*index, *coefficient * weight)),
            );
            known += part.known * weight;
        }
        Self::normalized(terms, known)
    }

    /// Every coefficient and the known part multiplied by `factor`.
    pub(crate) fn scaled(&self, factor: PallasScalar) -> Self {
        Self::combine([(self, factor)], PallasScalar::ZERO)
    }

    fn normalized(mut terms: Vec<(usize, PallasScalar)>, known: PallasScalar) -> Self {
        terms.sort_by_key(|(index, _)| *index);
        let mut merged: Vec<(usize, PallasScalar)> = Vec::with_capacity(terms.len());
        for (index, coefficient) in terms {
            match merged.last_mut() {
                Some((last, sum)) if *last == index => *sum += coefficient,
                _ => merged.push((index, coefficient)),
            }
        }
        merged.retain(|(_, coefficient)| !coefficient.is_zero_vartime());
        Self { terms: merged, known }
    }

    /// The commitment to this value: `Σ coefficient·C + known·G`. `commitments` must hold every
    /// index the layout handed out.
    pub(crate) fn point(
        &self,
        commitments: &[PallasPoint],
        generators: &Generators,
    ) -> PallasPoint {
        self.terms.iter().fold(generators.g * self.known, |sum, (index, coefficient)| {
            sum + commitments[*index] * *coefficient
        })
    }

    /// The value and blinding behind [`Self::point`], for the prover.
    pub(crate) fn opening(&self, openings: &[Opening]) -> Opening {
        self.terms.iter().fold(
            Opening { value: self.known, blinding: PallasScalar::ZERO },
            |sum, (index, coefficient)| Opening {
                value: sum.value + openings[*index].value * *coefficient,
                blinding: sum.blinding + openings[*index].blinding * *coefficient,
            },
        )
    }
}
