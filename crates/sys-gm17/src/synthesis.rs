//! The museum's R1CS handed to arkworks' constraint-system API, row for row.

use ark_bls12_381::Fr as Scalar;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::Fr;

/// A lowered circuit as an arkworks `ConstraintSynthesizer`, with or without the prover's assignment.
///
/// ark-gm17 builds keys and proofs by replaying a circuit's `generate_constraints` against a fresh
/// `ConstraintSystem`. Replaying the lowered R1CS unchanged, rather than rewriting the statement
/// with arkworks gadgets, keeps the constraints GM17 proves identical to those every other R1CS
/// system in the museum proves.
pub(crate) struct Synthesis<'a> {
    r1cs: &'a R1cs<Fr>,
    z: Option<&'a [Fr]>,
}

impl<'a> Synthesis<'a> {
    /// For key generation: no witness exists, and arkworks' setup mode never asks for a value.
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

impl ConstraintSynthesizer<Scalar> for Synthesis<'_> {
    /// Variable 0 is arkworks' own `Variable::One`; `z[1..=n]` become `new_input_variable`, so the
    /// verifier's input vector is exactly the museum's public inputs; the rest become
    /// `new_witness_variable`, and every R1CS row becomes one `enforce_constraint`.
    fn generate_constraints(self, cs: ConstraintSystemRef<Scalar>) -> Result<(), SynthesisError> {
        let public = self.r1cs.num_public_inputs();
        let mut variables = Vec::with_capacity(self.r1cs.num_variables());
        variables.push(Variable::One);
        for index in 1..=public {
            variables.push(cs.new_input_variable(|| self.value(index))?);
        }
        for index in public + 1..self.r1cs.num_variables() {
            variables.push(cs.new_witness_variable(|| self.value(index))?);
        }
        for constraint in self.r1cs.constraints() {
            cs.enforce_constraint(
                combination(&constraint.a, &variables),
                combination(&constraint.b, &variables),
                combination(&constraint.c, &variables),
            )?;
        }
        Ok(())
    }
}

/// `Σ coefficient · variable` as an arkworks linear combination. Indices come from the lowering,
/// which only emits indices below `num_variables`, the length of `variables`.
fn combination(terms: &SparseLc<Fr>, variables: &[Variable]) -> LinearCombination<Scalar> {
    LinearCombination(
        terms.iter().map(|(index, coefficient)| (coefficient.0, variables[*index])).collect(),
    )
}
