//! The museum's R1CS rows as the paper's constraint system: linear constraints `A·x = b` over the
//! extended witness, and quadratic constraints `x ⊙ y − z = 0` on aligned triples.
//!
//! The extended witness has the four blocks of §4.4:
//!
//! - `w`: the R1CS variables after the public inputs, `z[n+1..]` of the lowering (private inputs,
//!   multiplication outputs, hint outputs), in the lowering's order;
//! - `x`, `y`, `z`: one entry per quadratic row `i`, holding `⟨A_i, z⟩`, `⟨B_i, z⟩` and `⟨C_i, z⟩`.
//!
//! A row is **quadratic** when both `A_i` and `B_i` read a variable. It adds the triple and three
//! linear constraints tying the copies to `w`, as the paper's `x = P_x·w`, `y = P_y·w`,
//! `z = P_z·w` do, except that a copy here equals a linear combination rather than a single wire
//! (the linear test accepts any matrix `A`). Otherwise one factor is a constant `α` and the row is
//! **linear**, `α·⟨B_i, z⟩ − ⟨C_i, z⟩ = 0`, and adds one constraint — every assert-zero row
//! `lc · 1 = 0` is of this kind, and so play the part of the paper's addition gates `P_add`.
//!
//! Public inputs never enter the witness: a term on the constant `z[0] = 1` or on a public input
//! `z[1..=n]` moves to the right-hand side `b`, so a different public input is a different `b`.

use std::collections::BTreeMap;

use p3_field::PrimeCharacteristicRing;
use p3_goldilocks::Goldilocks;
use p3_matrix::dense::RowMajorMatrix;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::Fp;
use crate::params::Params;

type F = Goldilocks;

/// `Σ terms·x = constant + Σ coefficient·public`, one row of `A·x = b`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinearConstraint {
    /// `(position in the extended witness, coefficient)`; position `row·ℓ + column`.
    pub terms: Vec<(usize, F)>,
    /// The part of `b` fixed by the circuit.
    pub constant: F,
    /// `(public input index, coefficient)`: the part of `b` the public inputs supply.
    pub public: Vec<(usize, F)>,
}

impl LinearConstraint {
    /// `b` for these public inputs.
    pub(crate) fn rhs(&self, public: &[F]) -> F {
        self.public.iter().fold(self.constant, |acc, (index, c)| acc + public[*index] * *c)
    }
}

/// The circuit as linear constraints and quadratic triples, with the parameters its sizes fix.
#[derive(Clone, Debug)]
pub(crate) struct ConstraintSystem {
    /// Public inputs `n`.
    pub num_public: usize,
    /// Entries of the block `w`.
    pub witness_len: usize,
    /// The R1CS rows that became triples, in order; triple `i` is row `triples[i]`.
    pub triples: Vec<usize>,
    /// Every linear constraint, in R1CS row order (a triple's three copies first `x`, `y`, `z`).
    pub linear: Vec<LinearConstraint>,
    /// Code and layout.
    pub params: Params,
}

/// `Some(α)` when a combination reads no variable, only the constant one.
fn constant_only(lc: &SparseLc<Fp>) -> Option<F> {
    match lc.as_slice() {
        [] => Some(F::ZERO),
        [(0, coefficient)] => Some(coefficient.0),
        _ => None,
    }
}

/// `Σ scale·lc` as one sparse combination over `z`, without zero coefficients.
fn combine(parts: &[(F, &SparseLc<Fp>)]) -> Vec<(usize, F)> {
    let mut sum: BTreeMap<usize, F> = BTreeMap::new();
    for (scale, lc) in parts {
        for (variable, coefficient) in lc.iter() {
            *sum.entry(*variable).or_insert(F::ZERO) += *scale * coefficient.0;
        }
    }
    sum.into_iter().filter(|(_, coefficient)| *coefficient != F::ZERO).collect()
}

/// The constraint `Σ form·z = copy` (or `= 0` without a copy), with `z[0]` and the public inputs
/// moved to the right-hand side.
fn constraint(copy: Option<usize>, form: &[(usize, F)], num_public: usize) -> LinearConstraint {
    let mut terms: Vec<(usize, F)> = copy.map(|position| (position, F::ONE)).into_iter().collect();
    let mut constant = F::ZERO;
    let mut public = Vec::new();
    for (variable, coefficient) in form {
        match *variable {
            0 => constant = *coefficient,
            index if index <= num_public => public.push((index - 1, *coefficient)),
            index => terms.push((index - num_public - 1, -*coefficient)),
        }
    }
    LinearConstraint { terms, constant, public }
}

impl ConstraintSystem {
    /// Splits `r1cs` into triples and linear constraints and picks the parameters.
    pub(crate) fn from_r1cs(r1cs: &R1cs<Fp>) -> Result<Self, crate::error::LigeroError> {
        let num_public = r1cs.num_public_inputs();
        let witness_len = r1cs.num_variables() - 1 - num_public;
        let triples: Vec<usize> = (0..r1cs.num_constraints())
            .filter(|i| {
                let row = &r1cs.constraints()[*i];
                constant_only(&row.a).is_none() && constant_only(&row.b).is_none()
            })
            .collect();
        let params = Params::for_counts(witness_len, triples.len())?;
        let linear = linear_constraints(r1cs, &params);
        Ok(Self { num_public, witness_len, triples, linear, params })
    }

    /// `rᵀA`, what each witness entry is multiplied by in the linear test, as a matrix `ℓ` high with
    /// one column per tested row: the values at `ζ` of that row's multiplier polynomial `r_i(X)`.
    pub(crate) fn multipliers(&self, combiners: &[F]) -> RowMajorMatrix<F> {
        let (rows, ell) = (self.params.tested_rows(), self.params.message_length);
        let mut values = vec![F::ZERO; ell * rows];
        for (constraint, r) in self.linear.iter().zip(combiners) {
            for (position, coefficient) in &constraint.terms {
                values[(position % ell) * rows + position / ell] += *r * *coefficient;
            }
        }
        RowMajorMatrix::new(values, rows)
    }

    /// `rᵀb` for these public inputs.
    pub(crate) fn combine_rhs(&self, combiners: &[F], public: &[F]) -> F {
        self.linear.iter().zip(combiners).map(|(constraint, r)| *r * constraint.rhs(public)).sum()
    }
}

fn linear_constraints(r1cs: &R1cs<Fp>, params: &Params) -> Vec<LinearConstraint> {
    let num_public = r1cs.num_public_inputs();
    let ell = params.message_length;
    let copy = |row: usize, i: usize| Some(row * ell + i);
    let mut linear = Vec::new();
    let mut triple = 0;
    for row in r1cs.constraints() {
        let (a, b, c) = (&row.a, &row.b, &row.c);
        let form = match (constant_only(a), constant_only(b)) {
            (Some(alpha), _) => combine(&[(alpha, b), (-F::ONE, c)]),
            (None, Some(beta)) => combine(&[(beta, a), (-F::ONE, c)]),
            (None, None) => {
                let (row_x, row_y, row_z) = (params.x_row(0), params.y_row(0), params.z_row(0));
                for (block_row, lc) in [(row_x, a), (row_y, b), (row_z, c)] {
                    let form = combine(&[(F::ONE, lc)]);
                    linear.push(constraint(copy(block_row, triple), &form, num_public));
                }
                triple += 1;
                continue;
            }
        };
        linear.push(constraint(None, &form, num_public));
    }
    linear
}
