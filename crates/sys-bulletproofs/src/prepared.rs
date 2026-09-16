//! One example with its generators derived: prove, verify and the attack hooks.

use bulletproofs::r1cs::{ConstraintSystem, Verifier};
use bulletproofs::{BulletproofGens, PedersenGens};
use curve25519_dalek::scalar::Scalar;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, RistrettoScalar};
use crate::proof::{self, Statement};
use crate::synthesis::{synthesize, transcript};

/// A circuit lowered to R1CS, with the generators its multipliers need.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<RistrettoScalar>,
    r1cs: R1cs<RistrettoScalar>,
    pedersen: PedersenGens,
    generators: BulletproofGens,
    multipliers: usize,
}

impl Ready {
    /// Builds the circuit, lowers it, and derives the generators: Bulletproofs' whole setup.
    ///
    /// Nothing secret is drawn. `PedersenGens::default` and `BulletproofGens::new` hash fixed labels
    /// to curve points, so anyone recomputes the same generators and nobody knows their discrete logs.
    /// The vector generators come in pairs `G_i`, `H_i`, one per multiplier, rounded up to a power of
    /// two because the inner-product argument halves the vectors each round.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<RistrettoScalar>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = R1cs::from_circuit(&circuit);
        control.checkpoint()?;
        let multipliers = count_multipliers(example, &r1cs)?;
        let generators = BulletproofGens::new(multipliers.next_power_of_two(), 1);
        let pedersen = PedersenGens::default();
        Ok(Self { example, circuit, r1cs, pedersen, generators, multipliers })
    }

    fn statement(&self) -> Statement<'_> {
        Statement {
            example: self.example,
            r1cs: &self.r1cs,
            pedersen: &self.pedersen,
            generators: &self.generators,
        }
    }

    /// Evaluates `assignment` without checking it and hands every value to the upstream prover.
    fn prove_values(
        &self,
        assignment: &Assignment<RistrettoScalar>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let z = self.r1cs.assignment(&evaluation.values);
        let public: Vec<Scalar> = assignment.public.iter().map(|value| value.0).collect();
        control.checkpoint()?;
        let bytes = self.statement().prove(&public, &z)?;
        let encode =
            |values: &[RistrettoScalar]| values.iter().map(|v| field::encode(&v.0)).collect();
        Ok(Proven {
            proof: bytes,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }

    fn decode_public(&self, public: &[FieldBytes]) -> Result<Vec<Scalar>, String> {
        let expected = self.r1cs.num_public_inputs();
        if public.len() != expected {
            return Err(format!("expected {expected} public inputs, got {}", public.len()));
        }
        public.iter().map(|bytes| field::decode(bytes)).collect()
    }
}

/// Multipliers the constraint system allocates, read from upstream's own `metrics` after replaying
/// the verifier's side once, so the generator count follows the upstream allocation rules exactly.
fn count_multipliers(
    example: ExampleId,
    r1cs: &R1cs<RistrettoScalar>,
) -> Result<usize, SystemError> {
    let zeros = vec![Scalar::ZERO; r1cs.num_public_inputs()];
    let mut verifier = Verifier::new(transcript(example, &zeros));
    synthesize(&mut verifier, r1cs, &zeros, None)
        .map_err(|e| SystemError::Failed(format!("bulletproofs constraint system: {e}")))?;
    Ok(verifier.metrics().multipliers)
}

impl Prepared for Ready {
    /// The lowered R1CS as every R1CS system counts it, plus Bulletproofs' own sizes: the multipliers
    /// the proof commits to and the `G`, `H` generators derived for them.
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![
                ("constraints".into(), self.r1cs.num_constraints() as u64),
                ("variables".into(), self.r1cs.num_variables() as u64),
                ("multipliers".into(), self.multipliers as u64),
                ("generators".into(), 2 * self.generators.gens_capacity as u64),
            ],
        }
    }

    /// Nothing: the setup is transparent, and the generators are recomputed from public labels rather
    /// than stored, so there is no proving or verifying material to count. `shape` reports how many.
    fn setup_bytes(&self) -> Option<u64> {
        None
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with generators for {built}"
            )));
        }
        let assignment =
            instance.example.instance::<RistrettoScalar>(instance.kind, &instance.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: evaluated without checks, so a false assignment still reaches the upstream prover.
    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let decode_all = |values: &[FieldBytes]| -> Result<Vec<RistrettoScalar>, SystemError> {
            values
                .iter()
                .map(|bytes| field::decode(bytes).map(RistrettoScalar).map_err(SystemError::Failed))
                .collect()
        };
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_values(&assignment, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        match self.decode_public(public) {
            Ok(inputs) => self.statement().check(&inputs, proof),
            Err(why) => Ok(Verdict::Malformed(why)),
        }
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Scalar::ONE)))
    }

    /// See [`crate::TAMPER_OFFSET`].
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        proof::tamper_offset(proof)
    }
}
