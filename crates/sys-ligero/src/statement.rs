//! A circuit prepared for Ligero: lowered to R1CS, split into the paper's constraints, measured.

use p3_goldilocks::Goldilocks;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};
use zk_circuit::{Assignment, Circuit, ZkField};
use zk_core::ExampleId;

use crate::constraints::ConstraintSystem;
use crate::digest::circuit_digest;
use crate::error::LigeroError;
use crate::field::Fp;
use crate::hash::Bytes32;
use crate::params::Params;

/// The prover's extended witness (§4.4), one vector per block (module `constraints` defines them).
///
/// The prover encodes whatever these vectors hold; it never checks them against the constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedWitness {
    /// The R1CS variables after the public inputs.
    pub w: Vec<Fp>,
    /// Each quadratic row's left factor.
    pub x: Vec<Fp>,
    /// Each quadratic row's right factor.
    pub y: Vec<Fp>,
    /// Each quadratic row's product.
    pub z: Vec<Fp>,
}

/// Public inputs and the extended witness for them, true or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Public inputs in declaration order.
    pub public: Vec<Fp>,
    /// What the prover encodes.
    pub witness: ExtendedWitness,
}

/// One statement ready to prove and verify: nothing to set up beyond the circuit's own shape.
#[derive(Clone, Debug)]
pub struct Statement {
    id: String,
    circuit: Circuit<Fp>,
    digest: Bytes32,
    r1cs: R1cs<Fp>,
    system: ConstraintSystem,
}

impl Statement {
    /// Builds and lowers one of the museum's examples.
    pub fn new(example: ExampleId) -> Result<Self, LigeroError> {
        let circuit =
            example.circuit::<Fp>().map_err(|e| LigeroError::Circuit(format!("{e:?}")))?;
        Self::from_circuit(example.id(), circuit)
    }

    /// Lowers any circuit; `id` is bound into the challenges alongside the circuit digest.
    pub fn from_circuit(id: &str, circuit: Circuit<Fp>) -> Result<Self, LigeroError> {
        let r1cs = R1cs::from_circuit(&circuit);
        let system = ConstraintSystem::from_r1cs(&r1cs)?;
        let digest = circuit_digest(&circuit);
        Ok(Self { id: id.to_string(), circuit, digest, r1cs, system })
    }

    /// The ID the challenges bind.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The circuit as written.
    pub fn circuit(&self) -> &Circuit<Fp> {
        &self.circuit
    }

    /// The R1CS lowering the constraints come from.
    pub fn r1cs(&self) -> &R1cs<Fp> {
        &self.r1cs
    }

    /// The code and layout this circuit's size fixes.
    pub fn params(&self) -> Params {
        self.system.params
    }

    /// Quadratic constraints: the triples.
    pub fn quadratic_constraints(&self) -> usize {
        self.system.triples.len()
    }

    /// Linear constraints: three per triple plus one per linear R1CS row.
    pub fn linear_constraints(&self) -> usize {
        self.system.linear.len()
    }

    pub(crate) fn system(&self) -> &ConstraintSystem {
        &self.system
    }

    pub(crate) fn digest(&self) -> &Bytes32 {
        &self.digest
    }

    /// Refuses a public input list of the wrong length.
    pub(crate) fn check_public(&self, public: &[Fp]) -> Result<(), LigeroError> {
        let expected = self.system.num_public;
        if public.len() == expected {
            Ok(())
        } else {
            Err(LigeroError::PublicInputCount { expected, got: public.len() })
        }
    }

    /// Computes the extended witness for `assignment` without checking a single constraint, so a
    /// false claim reaches the verifier.
    pub fn claim(&self, assignment: &Assignment<Fp>) -> Result<Claim, LigeroError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| LigeroError::Circuit(format!("{e:?}")))?;
        let z = self.r1cs.assignment(&evaluation.values);
        let rows = self.r1cs.constraints();
        let triples = || self.system.triples.iter().map(|i| &rows[*i]);
        let witness = ExtendedWitness {
            w: z[1 + self.system.num_public..].to_vec(),
            x: triples().map(|row| dot(&row.a, &z)).collect(),
            y: triples().map(|row| dot(&row.b, &z)).collect(),
            z: triples().map(|row| dot(&row.c, &z)).collect(),
        };
        Ok(Claim { public: assignment.public.clone(), witness })
    }

    /// The witness blocks laid into rows of `ℓ` entries, zero-padded, in row order `w, x, y, z`.
    pub(crate) fn messages(
        &self,
        witness: &ExtendedWitness,
    ) -> Result<Vec<Goldilocks>, LigeroError> {
        let params = self.params();
        let ell = params.message_length;
        let triples = self.system.triples.len();
        let blocks = [
            ("w", &witness.w, self.system.witness_len, params.witness_rows),
            ("x", &witness.x, triples, params.triple_rows),
            ("y", &witness.y, triples, params.triple_rows),
            ("z", &witness.z, triples, params.triple_rows),
        ];
        let mut messages = Vec::with_capacity(params.tested_rows() * ell);
        for (block, values, expected, rows) in blocks {
            if values.len() != expected {
                return Err(LigeroError::WitnessLength { block, expected, got: values.len() });
            }
            messages.extend(values.iter().map(|value| value.0));
            messages.resize(messages.len() + rows * ell - expected, Goldilocks::default());
        }
        Ok(messages)
    }
}

fn dot(lc: &SparseLc<Fp>, z: &[Fp]) -> Fp {
    lc.iter()
        .fold(Fp::zero(), |acc, (variable, coefficient)| acc.add(z[*variable].mul(*coefficient)))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use zk_core::InstanceKind;

    use super::*;

    /// Whether every linear constraint and every triple holds, as the tests are meant to find out.
    fn satisfied(statement: &Statement, claim: &Claim) -> (bool, bool) {
        let messages = statement.messages(&claim.witness).unwrap();
        let public: Vec<_> = claim.public.iter().map(|value| value.0).collect();
        let linear = statement.system.linear.iter().all(|constraint| {
            let lhs: Goldilocks =
                constraint.terms.iter().map(|(position, c)| messages[*position] * *c).sum();
            lhs == constraint.rhs(&public)
        });
        let w = &claim.witness;
        let quadratic = (0..w.x.len()).all(|i| w.x[i].mul(w.y[i]) == w.z[i]);
        (linear, quadratic)
    }

    #[test]
    fn an_honest_claim_satisfies_every_constraint_and_a_false_one_does_not() {
        for example in ExampleId::ALL {
            let statement = Statement::new(example).unwrap();
            let seed = [3; 32];
            let honest = statement.claim(&example.instance(InstanceKind::Honest, &seed)).unwrap();
            assert_eq!(satisfied(&statement, &honest), (true, true), "{}", example.id());
            let false_claim =
                statement.claim(&example.instance(InstanceKind::Dishonest, &seed)).unwrap();
            assert_ne!(satisfied(&statement, &false_claim), (true, true), "{}", example.id());
        }
    }
}
