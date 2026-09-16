//! Computing every wire from input values.

use crate::circuit::{Circuit, Gate, Hint, Visibility};
use crate::error::EvalError;
use crate::field::ZkField;
use crate::lc::Wire;

/// Input values in the circuit's declaration order.
///
/// Proof-system adapters never compute a witness; they receive this, the circuit evaluates it,
/// and the lowering turns the wires into whatever shape the prover consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment<F> {
    /// Public input values, in [`Circuit::public_inputs`] order.
    pub public: Vec<F>,
    /// Private input values, in [`Circuit::private_inputs`] order.
    pub private: Vec<F>,
}

/// The value of every wire, indexed by [`Wire::index`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireValues<F> {
    values: Vec<F>,
}

impl<F: ZkField> WireValues<F> {
    /// Value of one wire.
    pub fn get(&self, wire: Wire) -> F {
        self.values[wire.index()]
    }

    /// All values, indexed by wire.
    pub fn as_slice(&self) -> &[F] {
        &self.values
    }
}

/// Wires computed without stopping at failed assertions.
///
/// Exists so a demonstration can hand a false witness to a real prover and show the verifier
/// rejecting the proof, instead of being stopped by the circuit layer first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncheckedEvaluation<F> {
    /// Every wire's value.
    pub values: WireValues<F>,
    /// Labels of every violated assertion, in gate order; empty for a valid witness.
    pub violated: Vec<String>,
}

impl<F: ZkField> Circuit<F> {
    /// Computes every wire and fails with the label of the first violated assertion.
    pub fn evaluate(&self, assignment: &Assignment<F>) -> Result<WireValues<F>, EvalError> {
        let evaluation = self.evaluate_unchecked(assignment)?;
        match evaluation.violated.into_iter().next() {
            Some(label) => Err(EvalError::AssertionFailed { label }),
            None => Ok(evaluation.values),
        }
    }

    /// Computes every wire and lists violated assertions instead of failing on them.
    pub fn evaluate_unchecked(
        &self,
        assignment: &Assignment<F>,
    ) -> Result<UncheckedEvaluation<F>, EvalError> {
        check_count(Visibility::Public, self.public_inputs.len(), assignment.public.len())?;
        check_count(Visibility::Private, self.private_inputs.len(), assignment.private.len())?;
        let mut values = vec![F::zero(); self.num_wires];
        let mut public = assignment.public.iter();
        let mut private = assignment.private.iter();
        let mut violated = Vec::new();
        for gate in &self.gates {
            match gate {
                Gate::Constant { output, value } => values[output.index()] = *value,
                Gate::Input { output, visibility, .. } => {
                    let source = match visibility {
                        Visibility::Public => public.next(),
                        Visibility::Private => private.next(),
                    };
                    values[output.index()] = source.copied().unwrap_or_else(F::zero);
                }
                Gate::Linear { output, lc } => values[output.index()] = lc.evaluate(&values),
                Gate::Mul { output, left, right } => {
                    values[output.index()] = left.evaluate(&values).mul(right.evaluate(&values));
                }
                Gate::AssertZero { lc, label } => {
                    if lc.evaluate(&values) != F::zero() {
                        violated.push(label.clone());
                    }
                }
                Gate::Hint { outputs, kind, input } => {
                    let results = run_hint(*kind, input.evaluate(&values));
                    for (output, result) in outputs.iter().zip(results) {
                        values[output.index()] = result;
                    }
                }
            }
        }
        Ok(UncheckedEvaluation { values: WireValues { values }, violated })
    }
}

fn check_count(visibility: Visibility, expected: usize, got: usize) -> Result<(), EvalError> {
    if expected == got { Ok(()) } else { Err(EvalError::InputCount { visibility, expected, got }) }
}

/// What a hint outputs. For `Bits(n)` on a value that does not fit, the low `n` bits are still
/// returned, so the recomposition assertion is what fails and names the broken range check.
fn run_hint<F: ZkField>(kind: Hint, input: F) -> Vec<F> {
    match kind {
        Hint::Inverse => vec![input.inverse().unwrap_or_else(F::zero)],
        Hint::Bits(count) => {
            let bytes = input.to_le_bytes();
            (0..count)
                .map(|bit| {
                    let byte = bytes.get((bit / 8) as usize).copied().unwrap_or(0);
                    if (byte >> (bit % 8)) & 1 == 1 { F::one() } else { F::zero() }
                })
                .collect()
        }
    }
}
