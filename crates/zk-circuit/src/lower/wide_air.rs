//! A wide, single-row AIR: one column per wire, one constraint per gate.

use std::collections::BTreeMap;

use crate::circuit::{Circuit, Gate};
use crate::eval::WireValues;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};
use crate::lower::constants::ConstantWires;
use crate::lower::{Violation, check_length, check_public_count};

/// `Σ coeff·col + Σ coeff·col_i·col_j + constant = 0`, over the columns of one row.
///
/// `linear` is sorted by column; `quadratic` is sorted by `(i, j)` with `i ≤ j`. Neither has
/// repeats or zero coefficients.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirConstraint<F> {
    /// Degree-1 terms `(column, coefficient)`.
    pub linear: Vec<(usize, F)>,
    /// Degree-2 terms `(column i, column j, coefficient)`.
    pub quadratic: Vec<(usize, usize, F)>,
    /// Degree-0 term.
    pub constant: F,
}

impl<F: ZkField> AirConstraint<F> {
    /// Value of the constraint on one row.
    pub fn evaluate(&self, row: &[F]) -> F {
        let linear = self.linear.iter().fold(self.constant, |acc, (column, coefficient)| {
            acc.add(row[*column].mul(*coefficient))
        });
        self.quadratic.iter().fold(linear, |acc, (i, j, coefficient)| {
            acc.add(row[*i].mul(row[*j]).mul(*coefficient))
        })
    }

    /// 1 or 2 (0 for a constraint with no columns).
    pub fn degree(&self) -> usize {
        if !self.quadratic.is_empty() { 2 } else { usize::from(!self.linear.is_empty()) }
    }
}

/// A circuit lowered to one AIR row.
///
/// Why this shape: every constraint applies uniformly to every row, so no selector columns are
/// needed and any STARK prover can take the circuit as it is. Each non-constant wire is a column,
/// in wire order; each linear, multiplication and assert-zero gate is one constraint of degree at
/// most 2 over that single row (`output − lc`, `output − left·right`, `lc`); constant wires are
/// folded into the constraints; input, constant and hint gates add no constraint. Public inputs
/// are boundary pairs `(column, value)` in declaration order, from [`Self::boundary`]. A STARK
/// prover repeats the one row to a power-of-two trace height.
///
/// This is not the shape STARKs are designed for. They shine on a small transition applied over
/// many rows (a register machine stepping through a long computation); a trace where every column
/// is constant spends none of that strength. It also means that without hiding (random masking of
/// the trace), the out-of-domain evaluation of a constant column equals the wire value itself, so
/// such a proof reveals every private wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WideAir<F> {
    column_wires: Vec<Wire>,
    constraints: Vec<AirConstraint<F>>,
    public_columns: Vec<usize>,
}

impl<F: ZkField> WideAir<F> {
    /// Lowers `circuit`.
    pub fn from_circuit(circuit: &Circuit<F>) -> Self {
        let constants = ConstantWires::of(circuit);
        let column_wires: Vec<Wire> = circuit
            .gates()
            .iter()
            .flat_map(|gate| gate.outputs().iter().copied())
            .filter(|wire| !constants.is_constant(wire.index()))
            .collect();
        let mut column_of = vec![0; circuit.num_wires()];
        for (column, wire) in column_wires.iter().enumerate() {
            column_of[wire.index()] = column;
        }
        let builder = ConstraintBuilder { column_of: &column_of, constants: &constants };
        let constraints =
            circuit.gates().iter().filter_map(|gate| builder.constraint(gate)).collect();
        let public_columns =
            circuit.public_inputs().iter().map(|input| column_of[input.wire.index()]).collect();
        Self { column_wires, constraints, public_columns }
    }

    /// The constraints, all of degree at most 2.
    pub fn constraints(&self) -> &[AirConstraint<F>] {
        &self.constraints
    }

    /// Number of constraints.
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// Number of columns.
    pub fn num_columns(&self) -> usize {
        self.column_wires.len()
    }

    /// Highest constraint degree.
    pub fn max_degree(&self) -> usize {
        self.constraints.iter().map(AirConstraint::degree).max().unwrap_or(0)
    }

    /// Column of each public input, in declaration order.
    pub fn public_columns(&self) -> &[usize] {
        &self.public_columns
    }

    /// Boundary pairs `(column, value)` for the verifier's public inputs, in declaration order.
    pub fn boundary(&self, public: &[F]) -> Result<Vec<(usize, F)>, Violation> {
        check_public_count(self.public_columns.len(), public.len())?;
        Ok(self.public_columns.iter().copied().zip(public.iter().copied()).collect())
    }

    /// The row of column values for evaluated wires.
    pub fn row(&self, wires: &WireValues<F>) -> Vec<F> {
        self.column_wires.iter().map(|wire| wires.get(*wire)).collect()
    }

    /// Checks the boundary pairs and every constraint on `row`.
    pub fn check(&self, public: &[F], row: &[F]) -> Result<(), Violation> {
        let boundary = self.boundary(public)?;
        check_length(self.column_wires.len(), row.len())?;
        if let Some(index) = boundary.iter().position(|(column, value)| row[*column] != *value) {
            return Err(Violation::PublicInput { index });
        }
        match self.constraints.iter().position(|c| c.evaluate(row) != F::zero()) {
            Some(index) => Err(Violation::Constraint { index }),
            None => Ok(()),
        }
    }
}

struct ConstraintBuilder<'a, F> {
    column_of: &'a [usize],
    constants: &'a ConstantWires<F>,
}

impl<F: ZkField> ConstraintBuilder<'_, F> {
    fn constraint(&self, gate: &Gate<F>) -> Option<AirConstraint<F>> {
        let mut sum = Sum::default();
        match gate {
            Gate::Constant { .. } | Gate::Input { .. } | Gate::Hint { .. } => return None,
            Gate::Linear { output, lc } => {
                // output − lc = 0
                let lc = self.constants.fold(lc);
                sum.linear(self.column_of[output.index()], F::one());
                self.add_terms(&mut sum, &lc, F::one().neg());
                sum.constant = lc.constant_term().neg();
            }
            Gate::Mul { output, left, right } => {
                // output − (Σ a_i·x_i + a_0)(Σ b_j·y_j + b_0) = 0, expanded.
                let (left, right) = (self.constants.fold(left), self.constants.fold(right));
                let (a0, b0) = (left.constant_term(), right.constant_term());
                sum.linear(self.column_of[output.index()], F::one());
                self.add_terms(&mut sum, &left, b0.neg());
                self.add_terms(&mut sum, &right, a0.neg());
                sum.constant = a0.mul(b0).neg();
                for (x, a) in left.terms() {
                    for (y, b) in right.terms() {
                        let (i, j) = (self.column_of[x.index()], self.column_of[y.index()]);
                        sum.quadratic(i.min(j), i.max(j), a.mul(*b).neg());
                    }
                }
            }
            Gate::AssertZero { lc, .. } => {
                let lc = self.constants.fold(lc);
                self.add_terms(&mut sum, &lc, F::one());
                sum.constant = lc.constant_term();
            }
        }
        Some(sum.finish())
    }

    /// Adds `factor · coefficient · column` for every wire term of `lc` (not its constant).
    fn add_terms(&self, sum: &mut Sum<F>, lc: &LinearCombination<F>, factor: F) {
        for (wire, coefficient) in lc.terms() {
            sum.linear(self.column_of[wire.index()], coefficient.mul(factor));
        }
    }
}

struct Sum<F> {
    linear: BTreeMap<usize, F>,
    quadratic: BTreeMap<(usize, usize), F>,
    constant: F,
}

impl<F: ZkField> Default for Sum<F> {
    fn default() -> Self {
        Self { linear: BTreeMap::new(), quadratic: BTreeMap::new(), constant: F::zero() }
    }
}

impl<F: ZkField> Sum<F> {
    fn linear(&mut self, column: usize, coefficient: F) {
        let entry = self.linear.entry(column).or_insert_with(F::zero);
        *entry = entry.add(coefficient);
    }

    fn quadratic(&mut self, i: usize, j: usize, coefficient: F) {
        let entry = self.quadratic.entry((i, j)).or_insert_with(F::zero);
        *entry = entry.add(coefficient);
    }

    fn finish(self) -> AirConstraint<F> {
        let nonzero = |coefficient: &F| *coefficient != F::zero();
        AirConstraint {
            linear: self.linear.into_iter().filter(|(_, c)| nonzero(c)).collect(),
            quadratic: self
                .quadratic
                .into_iter()
                .filter(|(_, c)| nonzero(c))
                .map(|((i, j), c)| (i, j, c))
                .collect(),
            constant: self.constant,
        }
    }
}
