//! The Reed–Solomon code: a row is a polynomial of degree below `k`, committed as its `n`
//! evaluations.
//!
//! - `ζ_c = ω_ℓ^c` for `c < ℓ`, the subgroup of order `ℓ`: where a row carries its message
//!   (Definition 4.2).
//! - `η_j = g·ω_n^j` for `j < n`, with `g = 7` the field's multiplicative generator: a coset that no
//!   power-of-two subgroup meets, so the evaluation points avoid every `ζ` as §4.7 requires and no
//!   column ever holds a message entry.
//! - Encoding a witness row (§4.7, "random codewords subject to `Decode_ζ(U) = w`"): fix its values
//!   on the subgroup `H_k`, the message at `ω_k^{c·k/ℓ} = ζ_c` and fresh randomness at the other
//!   `k − ℓ` points, interpolate with the inverse NTT and evaluate on the coset with the NTT.
//! - Blinding rows (§4.6): a random codeword for the proximity test; a random polynomial of degree
//!   below `k + ℓ − 1` whose values on `ζ` sum to zero for the linear test; a random polynomial of
//!   degree below `2k − 1` vanishing on every `ζ` for the quadratic test.
//!
//! All transforms are `p3-dft`'s `Radix2Dit`. A matrix column is one polynomial: row `i` of a
//! coefficient matrix is the coefficient of `X^i`, row `x` of an evaluation matrix the value at
//! `ω^x` (or `g·ω^x` on the coset). The unit tests check both conventions against plain evaluation.

use p3_dft::{Radix2Dit, TwoAdicSubgroupDft};
use p3_field::{Field, PrimeCharacteristicRing, TwoAdicField};
use p3_goldilocks::Goldilocks;
use p3_matrix::dense::RowMajorMatrix;

use crate::coins::Coins;
use crate::error::LigeroError;
use crate::params::Params;

type F = Goldilocks;

fn dft() -> Radix2Dit<F> {
    Radix2Dit::default()
}

fn log2(size: usize) -> usize {
    size.trailing_zeros() as usize
}

/// `η_j`, the point column `j` evaluates rows at.
pub(crate) fn eta(length: usize, j: usize) -> F {
    F::GENERATOR * F::two_adic_generator(log2(length)).exp_u64(j as u64)
}

/// Every `ζ_c`, in order.
pub(crate) fn zetas(message_length: usize) -> Vec<F> {
    F::two_adic_generator(log2(message_length)).powers().take(message_length).collect()
}

/// `f(point)` for coefficients listed from `X^0` up (Horner's rule).
pub(crate) fn evaluate(coefficients: &[F], point: F) -> F {
    coefficients.iter().rev().fold(F::ZERO, |acc, c| acc * point + *c)
}

/// Every column polynomial of a coefficient matrix evaluated at `point`.
pub(crate) fn evaluate_columns(coefficients: &RowMajorMatrix<F>, point: F) -> Vec<F> {
    let mut values = vec![F::ZERO; coefficients.width];
    let mut power = F::ONE;
    for row in coefficients.row_slices() {
        for (value, coefficient) in values.iter_mut().zip(row) {
            *value += *coefficient * power;
        }
        power *= point;
    }
    values
}

/// Row `x` of a matrix.
pub(crate) fn row(matrix: &RowMajorMatrix<F>, x: usize) -> &[F] {
    &matrix.values[x * matrix.width..(x + 1) * matrix.width]
}

/// The same polynomials with zero coefficients appended up to `height`.
fn padded(mut coefficients: RowMajorMatrix<F>, height: usize) -> RowMajorMatrix<F> {
    coefficients.values.resize(height * coefficients.width, F::ZERO);
    coefficients
}

/// Each column polynomial's values on the subgroup of order `height`.
pub(crate) fn on_subgroup(coefficients: RowMajorMatrix<F>, height: usize) -> RowMajorMatrix<F> {
    dft().dft_batch(padded(coefficients, height))
}

/// Each column polynomial's values on the coset `η` of order `length`.
pub(crate) fn on_coset(coefficients: RowMajorMatrix<F>, length: usize) -> RowMajorMatrix<F> {
    dft().coset_dft_batch(padded(coefficients, length), F::GENERATOR)
}

/// Coefficients of the polynomials whose values on a subgroup the columns list.
pub(crate) fn interpolate(values: RowMajorMatrix<F>) -> RowMajorMatrix<F> {
    dft().idft_batch(values)
}

/// Values on the subgroup of order `values.height() << added_bits` of the polynomials whose values
/// on the smaller subgroup the columns list.
pub(crate) fn extend(values: RowMajorMatrix<F>, height: usize) -> RowMajorMatrix<F> {
    let added_bits = log2(height) - log2(values.values.len() / values.width);
    dft().lde_batch(values, added_bits)
}

/// Coefficients of one polynomial whose values on a subgroup `values` lists.
pub(crate) fn interpolate_one(values: Vec<F>) -> Vec<F> {
    dft().idft(values)
}

/// The witness rows' polynomials (coefficients, one column per row): `messages` at `ζ`, fresh
/// randomness at the other points of `H_k`.
pub(crate) fn witness_polynomials(
    messages: &[F],
    params: &Params,
    coins: &mut Coins,
) -> Result<RowMajorMatrix<F>, LigeroError> {
    let (rows, k, ell) = (params.tested_rows(), params.dimension, params.message_length);
    let step = k / ell;
    let mut values = coins.elements(k * rows)?;
    for x in (0..k).step_by(step) {
        let c = x / step;
        for r in 0..rows {
            values[x * rows + r] = messages[r * ell + c];
        }
    }
    Ok(interpolate(RowMajorMatrix::new(values, rows)))
}

/// One repetition's blinding rows, as coefficients.
#[derive(Clone, Debug)]
pub(crate) struct Blinds {
    /// Degree below `k`: a random codeword.
    pub code: Vec<F>,
    /// Degree below `k + ℓ − 1`, values on `ζ` summing to zero.
    pub linear: Vec<F>,
    /// Degree below `2k − 1`, zero on every `ζ`.
    pub quadratic: Vec<F>,
}

impl Blinds {
    /// Uniform in each of the three spaces.
    pub(crate) fn sample(params: &Params, coins: &mut Coins) -> Result<Self, LigeroError> {
        let (k, ell) = (params.dimension, params.message_length);
        let code = coins.elements(k)?;
        // Σ_c f(ζ_c) = ℓ·Σ_{i ≡ 0 mod ℓ} f_i, so fixing f_0 makes the sum zero and leaves the rest free.
        let mut linear = coins.elements(k + ell - 1)?;
        let others: F = linear.iter().step_by(ell).skip(1).copied().sum();
        linear[0] = -others;
        // (X^ℓ − 1)·s vanishes exactly on the subgroup of order ℓ; s is uniform of degree < 2k − 1 − ℓ.
        let s = coins.elements(2 * k - 1 - ell)?;
        let quadratic = (0..2 * k - 1)
            .map(|i| {
                let shifted = if i >= ell { s[i - ell] } else { F::ZERO };
                shifted - s.get(i).copied().unwrap_or(F::ZERO)
            })
            .collect();
        Ok(Self { code, linear, quadratic })
    }
}

/// Coefficients of every committed row, `n` high: the witness rows, then each repetition's code,
/// linear and quadratic blinding rows.
pub(crate) fn committed_polynomials(
    witness: &RowMajorMatrix<F>,
    blinds: &[Blinds],
    params: &Params,
) -> RowMajorMatrix<F> {
    let (width, tested) = (params.rows(), params.tested_rows());
    let mut values = vec![F::ZERO; params.length * width];
    for (x, coefficients) in witness.row_slices().enumerate() {
        values[x * width..x * width + tested].copy_from_slice(coefficients);
    }
    for (h, blind) in blinds.iter().enumerate() {
        let at = params.blind_rows(h);
        for (column, polynomial) in
            [(at.code, &blind.code), (at.linear, &blind.linear), (at.quadratic, &blind.quadratic)]
        {
            for (x, coefficient) in polynomial.iter().enumerate() {
                values[x * width + column] = *coefficient;
            }
        }
    }
    RowMajorMatrix::new(values, width)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn params() -> Params {
        Params::for_counts(300, 200).unwrap()
    }

    fn column(matrix: &RowMajorMatrix<F>, r: usize) -> Vec<F> {
        matrix.row_slices().map(|values| values[r]).collect()
    }

    #[test]
    fn a_row_carries_its_message_at_zeta_and_its_columns_are_its_values_at_eta() {
        let params = params();
        let mut coins = Coins::new();
        let messages = coins.elements(params.tested_rows() * params.message_length).unwrap();
        let coefficients = witness_polynomials(&messages, &params, &mut coins).unwrap();
        let committed = on_coset(coefficients.clone(), params.length);
        for r in [0, params.tested_rows() - 1] {
            let polynomial = column(&coefficients, r);
            for (c, zeta) in zetas(params.message_length).into_iter().enumerate() {
                assert_eq!(evaluate(&polynomial, zeta), messages[r * params.message_length + c]);
            }
            for j in [0, 1, params.length - 1] {
                assert_eq!(evaluate(&polynomial, eta(params.length, j)), row(&committed, j)[r]);
            }
        }
    }

    #[test]
    fn the_blinding_rows_have_the_shape_their_tests_rely_on() {
        let params = params();
        let blinds = Blinds::sample(&params, &mut Coins::new()).unwrap();
        let zetas = zetas(params.message_length);
        let sum: F = zetas.iter().map(|zeta| evaluate(&blinds.linear, *zeta)).sum();
        assert_eq!(sum, F::ZERO);
        assert!(zetas.iter().all(|zeta| evaluate(&blinds.quadratic, *zeta) == F::ZERO));
        assert_ne!(evaluate(&blinds.quadratic, eta(params.length, 0)), F::ZERO);
        let lifted =
            on_subgroup(RowMajorMatrix::new_col(blinds.linear.clone()), 2 * params.dimension);
        let back = interpolate(lifted).values;
        assert_eq!(&back[..blinds.linear.len()], blinds.linear.as_slice());
    }
}
