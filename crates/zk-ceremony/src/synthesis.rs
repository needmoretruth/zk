//! A lowered R1CS handed to bellman's key generator, row for row.

use bellman::{Circuit, ConstraintSystem, LinearCombination, SynthesisError, Variable};
use bls12_381::Scalar;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::CircuitField;

/// A lowered circuit as a `bellman::Circuit`, without any assignment.
///
/// The ceremony only generates keys and never proves honestly, so no witness exists; every value
/// closure reports it missing, which bellman's key generator never asks for. Replaying the lowered
/// R1CS unchanged keeps the constraints identical to those the Groth16 exhibit proves.
pub(crate) struct Synthesis<'a> {
    r1cs: &'a R1cs<CircuitField>,
}

impl<'a> Synthesis<'a> {
    /// Wraps the R1CS whose keys are about to be generated.
    pub(crate) fn new(r1cs: &'a R1cs<CircuitField>) -> Self {
        Self { r1cs }
    }
}

impl Circuit<Scalar> for Synthesis<'_> {
    /// Variable 0 is bellman's own ONE input; `z[1..=n]` become `alloc_input`, the rest `alloc`, and
    /// every R1CS row becomes one `enforce`.
    fn synthesize<CS: ConstraintSystem<Scalar>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        let public = self.r1cs.num_public_inputs();
        let mut variables = Vec::with_capacity(self.r1cs.num_variables());
        variables.push(CS::one());
        for index in 1..=public {
            variables.push(cs.alloc_input(|| format!("public input {index}"), missing)?);
        }
        for index in public + 1..self.r1cs.num_variables() {
            variables.push(cs.alloc(|| format!("variable {index}"), missing)?);
        }
        for (row, constraint) in self.r1cs.constraints().iter().enumerate() {
            cs.enforce(
                || format!("row {row}"),
                |lc| extend(lc, &constraint.a, &variables),
                |lc| extend(lc, &constraint.b, &variables),
                |lc| extend(lc, &constraint.c, &variables),
            );
        }
        Ok(())
    }
}

/// The value of any variable during key generation: there is none.
fn missing() -> Result<Scalar, SynthesisError> {
    Err(SynthesisError::AssignmentMissing)
}

/// Adds `Σ coefficient · variable` to `lc`. Indices come from the lowering, which only emits indices
/// below `num_variables`, the length of `variables`.
fn extend(
    lc: LinearCombination<Scalar>,
    terms: &SparseLc<CircuitField>,
    variables: &[Variable],
) -> LinearCombination<Scalar> {
    terms.iter().fold(lc, |lc, (index, coefficient)| lc + (coefficient.0, variables[*index]))
}
