//! Wires and linear combinations of wires.

use core::ops::{Add, Neg, Sub};

use crate::field::ZkField;

/// A handle to one value computed by a circuit.
///
/// Wires are numbered in the order gates create them, so a value can only be read after the gate
/// that defines it; lowerings and evaluation rely on that order instead of sorting a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wire(usize);

impl Wire {
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    /// Position of this wire in [`crate::WireValues`] and in creation order.
    pub fn index(self) -> usize {
        self.0
    }
}

/// `Σ coefficient·wire + constant`.
///
/// Additions are free in every proof system, so the IR carries them as data instead of as gates:
/// R1CS inlines them, PLONKish chains them through rows, and the wide AIR folds them into single
/// constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearCombination<F> {
    terms: Vec<(Wire, F)>,
    constant: F,
}

impl<F: ZkField> LinearCombination<F> {
    /// The empty sum, the starting point for building a combination term by term.
    pub fn zero() -> Self {
        Self { terms: Vec::new(), constant: F::zero() }
    }

    /// A combination with no wires, for comparing a value against a fixed number.
    pub fn constant(value: F) -> Self {
        Self { terms: Vec::new(), constant: value }
    }

    /// The `(wire, coefficient)` terms, possibly with repeats; see [`Self::normalized`].
    pub fn terms(&self) -> &[(Wire, F)] {
        &self.terms
    }

    /// The constant term.
    pub fn constant_term(&self) -> F {
        self.constant
    }

    /// Appends `coefficient·wire`, for weighted sums such as bit recomposition.
    pub fn add_term(mut self, wire: Wire, coefficient: F) -> Self {
        self.terms.push((wire, coefficient));
        self
    }

    /// Adds a number to the constant term.
    pub fn add_constant(mut self, value: F) -> Self {
        self.constant = self.constant.add(value);
        self
    }

    /// Multiplies every coefficient and the constant by `factor`.
    pub fn scale(mut self, factor: F) -> Self {
        for (_, coefficient) in &mut self.terms {
            *coefficient = coefficient.mul(factor);
        }
        self.constant = self.constant.mul(factor);
        self
    }

    /// The same combination with repeated wires merged, zero coefficients dropped, and terms
    /// sorted by wire, so lowerings emit sparse rows without duplicates.
    pub fn normalized(&self) -> Self {
        let mut sorted = self.terms.clone();
        sorted.sort_by_key(|(wire, _)| *wire);
        let mut terms: Vec<(Wire, F)> = Vec::with_capacity(sorted.len());
        for (wire, coefficient) in sorted {
            match terms.last_mut() {
                Some((last, sum)) if *last == wire => *sum = sum.add(coefficient),
                _ => terms.push((wire, coefficient)),
            }
        }
        terms.retain(|(_, coefficient)| *coefficient != F::zero());
        Self { terms, constant: self.constant }
    }

    /// Value of the combination given every wire's value, indexed by wire.
    pub(crate) fn evaluate(&self, values: &[F]) -> F {
        self.terms.iter().fold(self.constant, |acc, (wire, coefficient)| {
            acc.add(values[wire.index()].mul(*coefficient))
        })
    }
}

impl<F: ZkField> From<Wire> for LinearCombination<F> {
    fn from(wire: Wire) -> Self {
        Self { terms: vec![(wire, F::one())], constant: F::zero() }
    }
}

impl<F: ZkField> Add for LinearCombination<F> {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self.terms.extend(rhs.terms);
        self.constant = self.constant.add(rhs.constant);
        self
    }
}

impl<F: ZkField> Sub for LinearCombination<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl<F: ZkField> Add<Wire> for LinearCombination<F> {
    type Output = Self;

    fn add(self, rhs: Wire) -> Self {
        self.add_term(rhs, F::one())
    }
}

impl<F: ZkField> Sub<Wire> for LinearCombination<F> {
    type Output = Self;

    fn sub(self, rhs: Wire) -> Self {
        self.add_term(rhs, F::one().neg())
    }
}

impl<F: ZkField> Neg for LinearCombination<F> {
    type Output = Self;

    fn neg(self) -> Self {
        self.scale(F::one().neg())
    }
}
