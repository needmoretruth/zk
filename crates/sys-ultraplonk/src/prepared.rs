//! One statement with its UltraPlonk setup done: prove, verify and the attack hooks.

use ark_ff::One;
use jf_relation::{Arithmetization, Circuit as _};
use zk_circuit::{Assignment, Circuit, ZkField};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{Fr, decode, encode};
use crate::ranges::RangePlan;
use crate::setup::{self, Keys};
use crate::{proof, translate};

/// A circuit translated into jellyfish's gates, with a reference string and preprocessing made for it.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Fr>,
    plan: RangePlan,
    keys: Keys,
    shape: CircuitShape,
}

impl Ready {
    /// Builds the circuit, translates it (over an all-zero assignment, since only its shape matters
    /// here), and runs both phases of the setup.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<Fr>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let plan = RangePlan::find(&circuit);
        let zeros = Assignment {
            public: vec![Fr::zero(); circuit.public_inputs().len()],
            private: vec![Fr::zero(); circuit.private_inputs().len()],
        };
        let built = translated(&circuit, &plan, &zeros)?;
        let counts = |name: &str, value: usize| (name.to_string(), value as u64);
        let domain = built.circuit.eval_domain_size().map_err(jellyfish_failed)?;
        let shape = CircuitShape {
            form: ShapeForm::Plonkish,
            counts: vec![
                counts("gates", built.gates),
                counts("variables", built.circuit.num_vars()),
                counts("public-inputs", built.circuit.num_inputs()),
                ("range-lookups".into(), plan.lookups()),
                ("range-checks".into(), plan.checks()),
                counts("domain-size", domain),
            ],
        };
        control.checkpoint()?;
        let keys = setup::generate(&built.circuit, control)?;
        Ok(Self { example, circuit, plan, keys, shape })
    }

    /// Evaluates the circuit without checking it, rebuilds jellyfish's circuit with those values and
    /// hands it to jellyfish's prover.
    fn prove_assigned(
        &self,
        assignment: &Assignment<Fr>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let built = translated(&self.circuit, &self.plan, assignment)?;
        control.checkpoint()?;
        let proof = proof::create(&self.keys, &built.circuit)?;
        Ok(Proven {
            proof,
            public: assignment.public.iter().map(|value| encode(&value.0)).collect(),
            secrets: assignment.private.iter().map(|value| value.to_le_bytes()).collect(),
        })
    }
}

/// `evaluate_unchecked` and the translation, errors mapped to the museum's.
fn translated(
    circuit: &Circuit<Fr>,
    plan: &RangePlan,
    assignment: &Assignment<Fr>,
) -> Result<translate::Built, SystemError> {
    let evaluation =
        circuit.evaluate_unchecked(assignment).map_err(|e| SystemError::Failed(e.to_string()))?;
    translate::build(circuit, plan, &evaluation.values).map_err(jellyfish_failed)
}

fn jellyfish_failed(error: jf_relation::CircuitError) -> SystemError {
    SystemError::Failed(format!("jellyfish circuit: {error}"))
}

// `secret_encodings` keeps the default: arkworks writes scalars as canonical little-endian bytes and
// points compressed, never in Montgomery form. The scan can still report a secret below 2^8 by
// coincidence: a proof carries seven evaluations of lookup polynomials that are zero in these circuits,
// and a random byte beside those zeros spells the 32-byte encoding of a one-byte value.
impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        self.shape.clone()
    }

    /// Compressed `CanonicalSerialize` size of jellyfish's proving key plus its verifying key.
    ///
    /// The proving key embeds a copy of the verifying key and the commitment key (the reference
    /// string trimmed to this circuit), so both are counted as jellyfish stores them.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.keys.bytes)
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with a setup for {built}"
            )));
        }
        let assignment = instance.example.instance::<Fr>(instance.kind, &instance.seed);
        self.prove_assigned(&assignment, control)
    }

    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_assigned(&assignment, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let expected = self.circuit.public_inputs().len();
        if public.len() != expected {
            let got = public.len();
            return Ok(Verdict::Malformed(format!("expected {expected} public inputs, got {got}")));
        }
        let Some(values) = public.iter().map(|bytes| decode(bytes)).collect::<Option<Vec<_>>>()
        else {
            return Ok(Verdict::Malformed("a public input is not a canonical BN254 scalar".into()));
        };
        proof::check(&self.keys, &values, proof)
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical BN254 scalar".into())
        })?;
        Ok(encode(&(value + ark_bn254::Fr::one())))
    }

    /// See [`proof::TAMPER_OFFSET`].
    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        proof::TAMPER_OFFSET
    }
}

/// Canonical scalars in the circuit's field, refusing any encoding that is not canonical.
fn decode_all(values: &[FieldBytes]) -> Result<Vec<Fr>, SystemError> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            decode(bytes).map(Fr).ok_or_else(|| {
                SystemError::Failed(format!("input {index} is not a canonical BN254 scalar"))
            })
        })
        .collect()
}
