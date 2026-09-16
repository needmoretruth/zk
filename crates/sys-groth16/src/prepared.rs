//! One example with its Groth16 parameters generated: prove, verify and the attack hooks.

use std::any::Any;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use bellman::VerificationError;
use bellman::groth16::{
    Parameters, PreparedVerifyingKey, Proof, create_random_proof, generate_random_parameters,
    prepare_verifying_key, verify_proof,
};
use bls12_381::{Bls12, Scalar};
use ff::Field;
use rand_core::OsRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, Fr};
use crate::synthesis::Synthesis;

/// Where the flip attack lands: byte 47, the last byte of the proof's first element `A`.
///
/// bellman writes a proof as `A ‖ B ‖ C`, compressed: `A` and `C` are 48-byte G1 points and `B` a
/// 96-byte G2 point, each a big-endian x-coordinate whose top three bits of byte 0 are flags. Byte 47
/// is the least significant byte of `A`'s x-coordinate, so flipping its low bit changes `x` by one
/// and touches no flag. `Proof::read` then has to find a point of the prime-order subgroup at the new
/// `x`; a neighbouring `x` gives one only with negligible probability, so the verifier refuses the
/// bytes while decoding `A`, the first thing it reads.
pub const TAMPER_OFFSET: usize = 47;

/// A circuit lowered to R1CS with Groth16 parameters generated for it by bellman.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Fr>,
    r1cs: R1cs<Fr>,
    parameters: Parameters<Bls12>,
    verifying_key: PreparedVerifyingKey<Bls12>,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, lowers it, and runs bellman's per-circuit setup with OS randomness.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<Fr>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = R1cs::from_circuit(&circuit);
        control.checkpoint()?;
        let parameters = generate_random_parameters::<Bls12, _, _>(
            Synthesis::without_witness(&r1cs),
            &mut OsRng,
        )
        .map_err(|e| SystemError::Failed(format!("bellman parameter generation: {e}")))?;
        control.checkpoint()?;
        let verifying_key = prepare_verifying_key(&parameters.vk);
        let mut counter = ByteCounter(0);
        parameters
            .write(&mut counter)
            .map_err(|e| SystemError::Failed(format!("bellman parameter serialization: {e}")))?;
        Ok(Self { example, circuit, r1cs, parameters, verifying_key, setup_bytes: counter.0 })
    }

    /// Evaluates `assignment` without checking it and hands every wire to bellman's prover.
    fn prove_values(
        &self,
        assignment: &Assignment<Fr>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let z = self.r1cs.assignment(&evaluation.values);
        control.checkpoint()?;
        let synthesis = Synthesis::with_witness(&self.r1cs, &z);
        let parameters = &self.parameters;
        let proved = catch_unwind(AssertUnwindSafe(|| {
            create_random_proof::<Bls12, _, _, _>(synthesis, parameters, &mut OsRng)
        }));
        let proof = match proved {
            Ok(Ok(proof)) => proof,
            Ok(Err(refusal)) => return Err(SystemError::Unsatisfied(refusal.to_string())),
            Err(panic) => return Err(SystemError::Unsatisfied(panic_message(panic.as_ref()))),
        };
        let mut bytes = Vec::new();
        proof
            .write(&mut bytes)
            .map_err(|e| SystemError::Failed(format!("bellman proof serialization: {e}")))?;
        let encode = |values: &[Fr]| values.iter().map(|value| field::encode(&value.0)).collect();
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

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![
                ("constraints".into(), self.r1cs.num_constraints() as u64),
                ("variables".into(), self.r1cs.num_variables() as u64),
            ],
        }
    }

    /// Length of `Parameters::write`, which holds the verifying key followed by the proving queries.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.setup_bytes)
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with keys for {built}"
            )));
        }
        let assignment = instance.example.instance::<Fr>(instance.kind, &instance.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: evaluated without checks, so a false assignment still reaches bellman's prover.
    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let decode_all = |values: &[FieldBytes]| -> Result<Vec<Fr>, SystemError> {
            values
                .iter()
                .map(|bytes| field::decode(bytes).map(Fr).map_err(SystemError::Failed))
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
        let inputs = match self.decode_public(public) {
            Ok(inputs) => inputs,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let mut reader = proof;
        let decoded = match Proof::<Bls12>::read(&mut reader) {
            Ok(decoded) => decoded,
            Err(why) => return Ok(Verdict::Malformed(format!("bellman Proof::read: {why}"))),
        };
        if !reader.is_empty() {
            return Ok(Verdict::Malformed(format!("{} bytes follow the proof", reader.len())));
        }
        Ok(match verify_proof(&self.verifying_key, &decoded, &inputs) {
            Ok(()) => Verdict::Accepted,
            Err(VerificationError::InvalidProof) => Verdict::Rejected,
            Err(VerificationError::InvalidVerifyingKey) => {
                Verdict::Malformed("public inputs do not fit the verifying key".to_string())
            }
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Scalar::ONE)))
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET
    }
}

/// Counts what `Parameters::write` emits without keeping megabytes of points in memory.
struct ByteCounter(u64);

impl Write for ByteCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("bellman panicked: {text}"),
        (_, Some(text)) => format!("bellman panicked: {text}"),
        _ => "bellman panicked".to_string(),
    }
}
