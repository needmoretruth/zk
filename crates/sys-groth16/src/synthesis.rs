//! The museum's R1CS handed to bellman's constraint-system API, row for row.

use bellman::{Circuit, ConstraintSystem, LinearCombination, SynthesisError, Variable};
use bls12_381::Scalar;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::Fr;

/// A lowered circuit as a `bellman::Circuit`, with or without the prover's assignment.
///
/// bellman builds keys and proofs by replaying a circuit's `synthesize` against its own constraint
/// systems. Replaying the lowered R1CS unchanged, rather than rewriting the statement in bellman's
/// gadget style, keeps the constraints bellman proves identical to those every other R1CS system
/// in the museum proves.
pub(crate) struct Synthesis<'a> {
    r1cs: &'a R1cs<Fr>,
    z: Option<&'a [Fr]>,
}

impl<'a> Synthesis<'a> {
    /// For parameter generation: no witness exists, so every value closure reports it missing.
    pub(crate) fn without_witness(r1cs: &'a R1cs<Fr>) -> Self {
        Self { r1cs, z: None }
    }

    /// For proving: `z` is the full assignment vector `[1, public, private, internal]`.
    pub(crate) fn with_witness(r1cs: &'a R1cs<Fr>, z: &'a [Fr]) -> Self {
        Self { r1cs, z: Some(z) }
    }

    fn value(&self, index: usize) -> Result<Scalar, SynthesisError> {
        self.z
            .and_then(|z| z.get(index))
            .map(|value| value.0)
            .ok_or(SynthesisError::AssignmentMissing)
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
            variables
                .push(cs.alloc_input(|| format!("public input {index}"), || self.value(index))?);
        }
        for index in public + 1..self.r1cs.num_variables() {
            variables.push(cs.alloc(|| format!("variable {index}"), || self.value(index))?);
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

/// Adds `Σ coefficient · variable` to `lc`. Indices come from the lowering, which only emits indices
/// below `num_variables`, the length of `variables`.
fn extend(
    lc: LinearCombination<Scalar>,
    terms: &SparseLc<Fr>,
    variables: &[Variable],
) -> LinearCombination<Scalar> {
    terms.iter().fold(lc, |lc, (index, coefficient)| lc + (coefficient.0, variables[*index]))
}
