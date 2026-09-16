//! The lowered circuit as a Winterfell AIR.

use std::sync::Arc;

use winter_air::{
    Air, AirContext, Assertion, EvaluationFrame, ProofOptions, TraceInfo,
    TransitionConstraintDegree,
};
use winter_math::{FieldElement, ToElements};
use zk_circuit::lower::wide_air::{AirConstraint, WideAir};

use crate::field::{F128, Val};

/// Rows in every trace this exhibit proves: Winterfell's minimum, `TraceInfo::MIN_TRACE_LENGTH`.
pub(crate) const TRACE_LENGTH: usize = TraceInfo::MIN_TRACE_LENGTH;

/// Wire columns a proof can carry and still be read back, next to the step counter.
///
/// `TraceInfo` 0.13.1 writes the width as one byte, and its reader refuses a width of 255 or more
/// even though the constructor allows exactly 255, so 254 columns survive a round trip through
/// bytes; one of them is the step counter.
pub(crate) const MAX_WIRE_COLUMNS: usize = TraceInfo::MAX_TRACE_WIDTH - 2;

/// Trace columns for a lowered circuit: every wire, then the step counter.
pub(crate) fn trace_width(wide: &WideAir<F128>) -> usize {
    wide.num_columns() + 1
}

/// Transition constraints for a lowered circuit: gates, wire copies, then the step counter.
pub(crate) fn num_transitions(wide: &WideAir<F128>) -> usize {
    wide.num_constraints() + wide.num_columns() + 1
}

/// What Winterfell's `Air::new` receives besides the trace shape: the statement and its claim.
///
/// `Air::new` is called by the prover and, separately, by `winter_verifier::verify` with nothing
/// but these inputs, so the lowered circuit has to travel here. Only the claimed public values
/// become field elements (`ToElements`), which Winterfell hashes into the seed of its public coin.
#[derive(Clone, Debug)]
pub(crate) struct Statement {
    wide: Arc<WideAir<F128>>,
    public: Vec<Val>,
}

impl Statement {
    /// A claim about `wide`'s public inputs.
    pub(crate) fn new(wide: Arc<WideAir<F128>>, public: Vec<Val>) -> Self {
        Self { wide, public }
    }
}

impl ToElements<Val> for Statement {
    fn to_elements(&self) -> Vec<Val> {
        self.public.clone()
    }
}

/// A circuit lowered to one wide row, repeated over the trace, plus a step counter.
///
/// Winterfell only expresses rules over a pair of consecutive rows and needs at least eight of
/// them. Every gate constraint is asserted on the current row, and every wire column must equal
/// itself on the next row, so all rows carry the same wire values. Winterfell exempts the last
/// transition (from the last row back to the first), which the equalities cover: the last row
/// equals the one before it. Public inputs are assertions on step 0.
///
/// Two things differ from the museum's plain wide AIR, both forced by `winter-prover` 0.13.1 and
/// measured:
/// - A last column counts steps (`next = current + 1`). When every column is constant the DEEP
///   composition polynomial is zero, and the prover's `assert_eq!(trace_length - 2,
///   deep_composition_poly.degree())`, which is not a debug assertion, panicked on every honest
///   proof in a release build. One column that climbs 0..7 has degree 7 and satisfies it; the
///   wire columns, and so the leak, are untouched.
/// - Every constraint declares degree 1. In builds with debug assertions the prover compares each
///   declared degree with the degree the constraint shows over the trace; over constant wire
///   columns every gate shows degree 0, which is what degree 1 predicts, while the true degree 2
///   of a multiplication predicts 7, and the prover panicked on every honest proof. Outside that
///   check 0.13.1 reads the degrees only to size the constraint evaluation domain and the number
///   of composition columns, which are the same for 1 and 2 at eight rows (pinned by a test
///   below). A release build declaring the true degrees wrote byte-identical proofs, honest and
///   false, and the verifier reached the same verdicts.
pub(crate) struct CircuitAir {
    context: AirContext<Val>,
    statement: Statement,
}

impl Air for CircuitAir {
    type BaseField = Val;
    type PublicInputs = Statement;

    fn new(trace_info: TraceInfo, statement: Statement, options: ProofOptions) -> Self {
        let degrees = declared_degrees(num_transitions(&statement.wide));
        let context = AirContext::new(trace_info, degrees, statement.public.len(), options);
        Self { context, statement }
    }

    fn context(&self) -> &AirContext<Val> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement<BaseField = Val>>(
        &self,
        frame: &EvaluationFrame<E>,
        _periodic_values: &[E],
        result: &mut [E],
    ) {
        let (current, next) = (frame.current(), frame.next());
        let wide = &self.statement.wide;
        let (gates, rest) = result.split_at_mut(wide.num_constraints());
        for (slot, constraint) in gates.iter_mut().zip(wide.constraints()) {
            *slot = folded(constraint, current);
        }
        let (copies, step) = rest.split_at_mut(wide.num_columns());
        for ((slot, now), then) in copies.iter_mut().zip(current).zip(next) {
            *slot = *then - *now;
        }
        let counter = wide.num_columns();
        if let Some(slot) = step.first_mut() {
            *slot = next[counter] - current[counter] - E::ONE;
        }
    }

    fn get_assertions(&self) -> Vec<Assertion<Val>> {
        let columns = self.statement.wide.public_columns();
        columns
            .iter()
            .zip(&self.statement.public)
            .map(|(column, value)| Assertion::single(*column, 0, *value))
            .collect()
    }
}

/// Degree 1 for each of `count` constraints (see [`CircuitAir`] for why not the true degree).
fn declared_degrees(count: usize) -> Vec<TransitionConstraintDegree> {
    (0..count).map(|_| TransitionConstraintDegree::new(1)).collect()
}

/// `Σ c·col + Σ c·col_i·col_j + constant` over one row, in whatever field Winterfell evaluates in.
fn folded<E: FieldElement<BaseField = Val>>(constraint: &AirConstraint<F128>, row: &[E]) -> E {
    let linear = constraint
        .linear
        .iter()
        .fold(E::from(constraint.constant.0), |acc, (column, c)| acc + row[*column].mul_base(c.0));
    constraint
        .quadratic
        .iter()
        .fold(linear, |acc, (i, j, c)| acc + (row[*i] * row[*j]).mul_base(c.0))
}

#[cfg(test)]
mod tests {
    use winter_air::{AirContext, TraceInfo, TransitionConstraintDegree};

    use super::{TRACE_LENGTH, declared_degrees};
    use crate::field::Val;
    use crate::stark::OPTIONS;

    /// Fails if declaring every constraint's true degree (2 for a multiplication) would change the
    /// constraint evaluation domain or the composition columns, the only verifier-side quantities
    /// Winterfell 0.13.1 derives from declared degrees.
    #[test]
    fn declared_degrees_size_the_verifier_like_the_true_degrees() {
        let info = TraceInfo::new(4, TRACE_LENGTH);
        let true_degrees =
            vec![TransitionConstraintDegree::new(2), TransitionConstraintDegree::new(1)];
        let declared = AirContext::<Val>::new(info.clone(), declared_degrees(2), 1, OPTIONS);
        let actual = AirContext::<Val>::new(info, true_degrees, 1, OPTIONS);
        assert_eq!(declared.ce_domain_size(), actual.ce_domain_size());
        assert_eq!(
            declared.num_constraint_composition_columns(),
            actual.num_constraint_composition_columns()
        );
    }
}
