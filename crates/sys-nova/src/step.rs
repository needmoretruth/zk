//! The museum's R1CS as Nova's step function `F`: every row enforced, the public state passed through.

use std::sync::Arc;

use nova_snark::frontend::num::AllocatedNum;
use nova_snark::frontend::{ConstraintSystem, LinearCombination, SynthesisError, Variable};
use nova_snark::traits::circuit::StepCircuit;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::{PallasScalar, Scalar};

/// One step of the chain: the statement's rows over `z_in` and the private advice, with `z_out = z_in`.
///
/// The museum's assignment is `z = [1, public inputs, private inputs and internal wires]`. Nova hands
/// a step circuit its incoming state as already allocated variables, so the public inputs are that
/// state (`arity` = number of public inputs) and the rest of `z` is allocated as non-deterministic
/// advice. Every lowered row becomes one `enforce` over those variables. The state leaves the step
/// unchanged, so after any number of steps the verifier's `z_n` must equal its `z_0`: the public
/// inputs it was asked about.
#[derive(Clone)]
pub(crate) struct StepFunction {
    rows: Arc<R1cs<PallasScalar>>,
    advice: Option<Arc<[Scalar]>>,
}

impl StepFunction {
    /// The circuit without values, for `PublicParams::setup`, which only records its shape.
    pub(crate) fn shape_only(rows: Arc<R1cs<PallasScalar>>) -> Self {
        Self { rows, advice: None }
    }

    /// The circuit with `advice` = `z` after the constant one and the public inputs.
    pub(crate) fn with_advice(rows: Arc<R1cs<PallasScalar>>, advice: Vec<Scalar>) -> Self {
        Self { rows, advice: Some(advice.into()) }
    }

    /// Variables in the museum's `z` that are neither the constant one nor a public input.
    fn num_advice(&self) -> usize {
        self.rows.num_variables() - 1 - self.rows.num_public_inputs()
    }

    /// Allocates the advice as private variables; without values only the shape is recorded.
    fn allocate_advice<CS: ConstraintSystem<Scalar>>(
        &self,
        cs: &mut CS,
    ) -> Result<Vec<Variable>, SynthesisError> {
        (0..self.num_advice())
            .map(|index| {
                let value = self.advice.as_ref().and_then(|advice| advice.get(index).copied());
                cs.alloc(
                    || format!("advice {index}"),
                    || value.ok_or(SynthesisError::AssignmentMissing),
                )
            })
            .collect()
    }
}

impl StepCircuit<Scalar> for StepFunction {
    fn arity(&self) -> usize {
        self.rows.num_public_inputs()
    }

    fn synthesize<CS: ConstraintSystem<Scalar>>(
        &self,
        cs: &mut CS,
        z: &[AllocatedNum<Scalar>],
    ) -> Result<Vec<AllocatedNum<Scalar>>, SynthesisError> {
        if z.len() != self.arity() {
            return Err(SynthesisError::IncompatibleLengthVector(format!(
                "the step state has {} elements, the statement {} public inputs",
                z.len(),
                self.arity()
            )));
        }
        let advice = self.allocate_advice(cs)?;
        let variables: Vec<Variable> = core::iter::once(CS::one())
            .chain(z.iter().map(AllocatedNum::get_variable))
            .chain(advice)
            .collect();
        for (index, row) in self.rows.constraints().iter().enumerate() {
            let (a, b, c) = (
                combine(&row.a, &variables),
                combine(&row.b, &variables),
                combine(&row.c, &variables),
            );
            cs.enforce(|| format!("row {index}"), |_| a, |_| b, |_| c);
        }
        Ok(z.to_vec())
    }
}

/// A sparse row over the museum's variable indices as Nova's linear combination.
fn combine(terms: &SparseLc<PallasScalar>, variables: &[Variable]) -> LinearCombination<Scalar> {
    terms.iter().fold(LinearCombination::zero(), |lc, (index, coefficient)| {
        lc + (coefficient.0, variables[*index])
    })
}
