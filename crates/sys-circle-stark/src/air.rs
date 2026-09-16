//! The lowered circuit as a Plonky3 AIR over Mersenne31.

use p3_air::{Air, AirBuilder, BaseAir, WindowAccess};
use zk_circuit::lower::wide_air::{AirConstraint, WideAir};

use crate::field::{Mersenne31, Val};

/// Highest degree of any constraint this AIR asserts.
///
/// Gate constraints are at most quadratic, and a public-input constraint is a degree-1 difference
/// multiplied by the degree-1 first-row selector.
const MAX_CONSTRAINT_DEGREE: usize = 2;

/// A circuit lowered to one wide AIR row, in the form `p3-uni-stark` proves.
///
/// Every WideAir constraint is asserted on every row with no selector: the trace is the one row
/// of wire values repeated, so a constraint that holds on the row holds everywhere. Public inputs
/// are Plonky3 public values tied to their columns on the first row only, which is where
/// Plonky3's own AIRs bind public values; the trace being constant makes one row enough. No
/// constraint reads the next row, so the prover never opens the trace at the next-row point.
pub(crate) struct CircuitAir {
    wide: WideAir<Mersenne31>,
}

impl CircuitAir {
    /// Wraps a lowered circuit.
    pub(crate) fn new(wide: WideAir<Mersenne31>) -> Self {
        Self { wide }
    }

    /// The lowered circuit, for building the trace row and reporting its shape.
    pub(crate) fn wide(&self) -> &WideAir<Mersenne31> {
        &self.wide
    }

    /// Constraints asserted per row: one per gate constraint plus one per public input.
    pub(crate) fn num_asserted(&self) -> usize {
        self.wide.num_constraints() + self.wide.public_columns().len()
    }
}

impl BaseAir<Val> for CircuitAir {
    fn width(&self) -> usize {
        self.wide.num_columns()
    }

    fn main_next_row_columns(&self) -> Vec<usize> {
        Vec::new()
    }

    fn num_constraints(&self) -> Option<usize> {
        Some(self.num_asserted())
    }

    fn max_constraint_degree(&self) -> Option<usize> {
        Some(MAX_CONSTRAINT_DEGREE)
    }

    fn num_public_values(&self) -> usize {
        self.wide.public_columns().len()
    }
}

impl<AB: AirBuilder<F = Val>> Air<AB> for CircuitAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let row = main.current_slice();
        for constraint in self.wide.constraints() {
            builder.assert_zero(folded::<AB>(constraint, row));
        }
        let public: Vec<AB::PublicVar> = builder.public_values().to_vec();
        let mut first_row = builder.when_first_row();
        for (column, value) in self.wide.public_columns().iter().zip(public) {
            first_row.assert_eq(row[*column], value);
        }
    }
}

/// `Σ c·col + Σ c·col_i·col_j + constant` as a builder expression over the current row.
fn folded<AB: AirBuilder<F = Val>>(
    constraint: &AirConstraint<Mersenne31>,
    row: &[AB::Var],
) -> AB::Expr {
    let mut sum = AB::Expr::from(constraint.constant.0);
    for (column, coefficient) in &constraint.linear {
        sum += row[*column] * coefficient.0;
    }
    for (i, j, coefficient) in &constraint.quadratic {
        sum += (row[*i] * row[*j]) * coefficient.0;
    }
    sum
}
