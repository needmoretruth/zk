//! The museum's PLONKish table laid out as the circuit description lambdaworks' prover reads.
//!
//! lambdaworks' high-level `ConstraintSystem` only emits the gate shapes its own helpers build
//! (`c = q_L·a + q_R·b + q_C`, `c = a·b`, …): its `Constraint` has private fields, so an arbitrary
//! selector row such as `q_L·a + q_R·b + q_O·c + q_C = 0` with `q_O ≠ −1` cannot be handed to it.
//! This module therefore builds the lower-level `CommonPreprocessedInput` the prover and verifier
//! actually consume: the five selector polynomials, the three permutation polynomials `S_σ1..3`,
//! the domain and the coset generator. Upstream functions build the parts that encode meaning —
//! `get_permutation` turns cell labels into the copy permutation exactly as `ConstraintSystem`
//! would, and `generate_permutation_coefficients` maps it onto `H ∪ k₁H ∪ k₂H`.

use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::default_types::{
    FrElement, FrField,
};
use lambdaworks_math::field::traits::IsFFTField;
use lambdaworks_math::polynomial::Polynomial;
use lambdaworks_plonk::constraint_system::get_permutation;
use lambdaworks_plonk::setup::{CommonPreprocessedInput, Witness, validate_coset_generator};
use lambdaworks_plonk::test_utils::utils::{
    ORDER_R_MINUS_1_ROOT_UNITY, generate_domain, generate_permutation_coefficients,
};
use zk_circuit::lower::plonkish::{Column, Plonkish};
use zk_core::SystemError;

use crate::field::Fr;

/// The smallest domain lambdaworks' quotient split handles: `t` of degree `3n + 5` must fit in the
/// `4n` evaluations round 3 interpolates from, which needs `n ≥ 8`.
const MIN_DOMAIN: usize = 8;

/// The table's rows padded to a power of two, as PLONK's multiplicative domain `H` requires.
pub(crate) fn domain_size(table: &Plonkish<Fr>) -> usize {
    table.num_rows().next_power_of_two().max(MIN_DOMAIN)
}

/// Selector and permutation polynomials for `table`, in lambdaworks' `CommonPreprocessedInput`.
///
/// Public-input rows are the one place the table and lambdaworks differ in sign. The museum writes
/// row `i` as `a − x_i = 0` (`q_L = 1`, the paper's `PI(X) = −Σ x_i·L_i(X)`); lambdaworks adds
/// `PI(X) = +Σ x_i·L_i(X)` and writes its own public rows with `q_L = −1`. Every selector of those
/// rows is negated, which multiplies the row equation by −1: the same constraint, `a = x_i`.
pub(crate) fn preprocess(
    table: &Plonkish<Fr>,
) -> Result<CommonPreprocessedInput<FrField>, SystemError> {
    let n = domain_size(table);
    let log_n = u64::from(n.trailing_zeros());
    let omega = FrField::get_primitive_root_of_unity(log_n)
        .map_err(|e| SystemError::Failed(format!("no root of unity of order 2^{log_n}: {e:?}")))?;
    let k1 = ORDER_R_MINUS_1_ROOT_UNITY;
    validate_coset_generator(&k1, n).map_err(|e| SystemError::Failed(e.to_string()))?;

    let [ql, qr, qm, qo, qc] = selector_columns(table, n);
    let permutation = get_permutation(&cell_labels(table, n));
    let sigma = generate_permutation_coefficients(&omega, n, &permutation, &k1);
    let (s1_lagrange, s2_lagrange, s3_lagrange) =
        (sigma[..n].to_vec(), sigma[n..2 * n].to_vec(), sigma[2 * n..].to_vec());
    Ok(CommonPreprocessedInput {
        n,
        domain: generate_domain(&omega, n),
        omega,
        k1,
        ql: interpolate(&ql)?,
        qr: interpolate(&qr)?,
        qo: interpolate(&qo)?,
        qm: interpolate(&qm)?,
        qc: interpolate(&qc)?,
        s1: interpolate(&s1_lagrange)?,
        s2: interpolate(&s2_lagrange)?,
        s3: interpolate(&s3_lagrange)?,
        s1_lagrange,
        s2_lagrange,
        s3_lagrange,
    })
}

/// The wire columns `a`, `b`, `c` of lambdaworks' witness, padding rows holding zero.
pub(crate) fn witness(cells: &[[Fr; 3]], n: usize) -> Witness<FrField> {
    let column = |index: usize| -> Vec<FrElement> {
        let mut values: Vec<FrElement> = cells.iter().map(|row| row[index].element()).collect();
        values.resize(n, FrElement::zero());
        values
    };
    Witness {
        a: column(Column::A.index()),
        b: column(Column::B.index()),
        c: column(Column::C.index()),
    }
}

/// Selector values `[q_L, q_R, q_M, q_O, q_C]` over the whole domain, padding rows all zero.
fn selector_columns(table: &Plonkish<Fr>, n: usize) -> [Vec<FrElement>; 5] {
    let mut columns: [Vec<FrElement>; 5] = Default::default();
    for column in &mut columns {
        column.reserve(n);
    }
    for (index, row) in table.rows().iter().enumerate() {
        let values = [row.q_l, row.q_r, row.q_m, row.q_o, row.q_c];
        let public = index < table.num_public_rows();
        for (column, value) in columns.iter_mut().zip(values) {
            column.push(if public { -value.element() } else { value.element() });
        }
    }
    for column in &mut columns {
        column.resize(n, FrElement::zero());
    }
    columns
}

/// One label per cell in lambdaworks' `L ‖ R ‖ O` order (`column · n + row`), the form
/// `get_permutation` reads: cells of one copy class share the label of the class's first cell,
/// every other cell keeps a label of its own and so maps to itself.
fn cell_labels(table: &Plonkish<Fr>, n: usize) -> Vec<usize> {
    let mut labels: Vec<usize> = (0..3 * n).collect();
    let position = |row: usize, column: Column| column.index() * n + row;
    for class in table.copy_classes() {
        if let Some(first) = class.first() {
            let label = position(first.row, first.column);
            for cell in class {
                labels[position(cell.row, cell.column)] = label;
            }
        }
    }
    labels
}

fn interpolate(values: &[FrElement]) -> Result<Polynomial<FrElement>, SystemError> {
    Polynomial::interpolate_fft::<FrField>(values)
        .map_err(|e| SystemError::Failed(format!("selector interpolation: {e:?}")))
}
