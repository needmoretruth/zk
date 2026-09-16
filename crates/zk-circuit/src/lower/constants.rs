//! Folding constant wires into constant terms, shared by every lowering.

use crate::circuit::{Circuit, Gate};
use crate::field::ZkField;
use crate::lc::LinearCombination;

/// The value of every wire defined by a constant gate.
///
/// Lowerings do not spend a variable, cell or column on a number the verifier already knows.
pub(crate) struct ConstantWires<F> {
    values: Vec<Option<F>>,
}

impl<F: ZkField> ConstantWires<F> {
    pub(crate) fn of(circuit: &Circuit<F>) -> Self {
        let mut values = vec![None; circuit.num_wires()];
        for gate in circuit.gates() {
            if let Gate::Constant { output, value } = gate {
                values[output.index()] = Some(*value);
            }
        }
        Self { values }
    }

    pub(crate) fn is_constant(&self, index: usize) -> bool {
        self.values[index].is_some()
    }

    /// `lc` normalized, with every constant wire moved into the constant term.
    pub(crate) fn fold(&self, lc: &LinearCombination<F>) -> LinearCombination<F> {
        let mut folded = LinearCombination::constant(lc.constant_term());
        for (wire, coefficient) in lc.terms() {
            folded = match self.values[wire.index()] {
                Some(value) => folded.add_constant(value.mul(*coefficient)),
                None => folded.add_term(*wire, *coefficient),
            };
        }
        folded.normalized()
    }
}
