//! One example with its GM17 proving key generated: prove, verify and the attack hooks.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ark_bls12_381::{Bls12_381, Fr as Scalar};
use ark_ff::One;
use ark_gm17::{
    PreparedVerifyingKey, Proof, ProvingKey, create_random_proof, generate_random_parameters,
    prepare_verifying_key, verify_proof,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::OsRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, Fr};
use crate::synthesis::Synthesis;

/// Where the flip attack lands: byte 0, the first byte of the proof's first element `A`.
///
/// arkworks serializes a GM17 proof as `A ‖ B ‖ C`, compressed: `A` and `C` are 48-byte G1 points
/// and `B` a 96-byte G2 point, each an x-coordinate written little-endian with the infinity and
/// y-sign flags in the top two bits of the element's last byte. Byte 0 is the least significant
/// byte of `A`'s x-coordinate, so flipping its low bit changes `x` by one and touches no flag.
/// `CanonicalDeserialize` then has to find a curve point at the new `x` and check that it lies in
/// the prime-order subgroup; a neighbouring `x` passes both only with negligible probability, so
/// the verifier refuses the bytes while decoding `A`, the first thing it reads.
pub const TAMPER_OFFSET: usize = 0;

/// A circuit lowered to R1CS with a GM17 proving key generated for it by ark-gm17.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Fr>,
    r1cs: R1cs<Fr>,
    proving_key: ProvingKey<Bls12_381>,
    verifying_key: PreparedVerifyingKey<Bls12_381>,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, lowers it, and runs ark-gm17's per-circuit setup with OS randomness.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<Fr>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = R1cs::from_circuit(&circuit);
        control.checkpoint()?;
        let proving_key = generate_random_parameters::<Bls12_381, _, _>(
            Synthesis::without_witness(&r1cs),
            &mut OsRng,
        )
        .map_err(|e| SystemError::Failed(format!("ark-gm17 parameter generation: {e}")))?;
        control.checkpoint()?;
        let verifying_key = prepare_verifying_key(&proving_key.vk);
        let setup_bytes = proving_key.serialized_size() as u64;
        Ok(Self { example, circuit, r1cs, proving_key, verifying_key, setup_bytes })
    }

    /// Evaluates `assignment` without checking it and hands every wire to ark-gm17's prover.
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
        let proving_key = &self.proving_key;
        // ark-gm17 0.3 never checks the witness (neither its prover nor ark-relations' constraint
        // system calls `is_satisfied`), so a false claim still becomes a proof; an error or a panic
        // here would be arkworks' own refusal and is reported as one.
        let proved = catch_unwind(AssertUnwindSafe(|| {
            create_random_proof::<Bls12_381, _, _>(synthesis, proving_key, &mut OsRng)
        }));
        let proof = match proved {
            Ok(Ok(proof)) => proof,
            Ok(Err(refusal)) => return Err(SystemError::Unsatisfied(refusal.to_string())),
            Err(panic) => return Err(SystemError::Unsatisfied(panic_message(panic.as_ref()))),
        };
        let mut bytes = Vec::new();
        proof
            .serialize(&mut bytes)
            .map_err(|e| SystemError::Failed(format!("ark-gm17 proof serialization: {e}")))?;
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

    /// Compressed `ProvingKey::serialized_size`. arkworks' proving key carries the verifying key
    /// inside it, so this counts the proving and the verifying material, each once.
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
    /// claim: evaluated without checks, so a false assignment still reaches ark-gm17's prover.
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
        let decoded = match Proof::<Bls12_381>::deserialize(&mut reader) {
            Ok(decoded) => decoded,
            Err(why) => {
                return Ok(Verdict::Malformed(format!("ark-gm17 Proof::deserialize: {why}")));
            }
        };
        if !reader.is_empty() {
            return Ok(Verdict::Malformed(format!("{} bytes follow the proof", reader.len())));
        }
        Ok(match verify_proof(&self.verifying_key, &decoded, &inputs) {
            Ok(true) => Verdict::Accepted,
            Ok(false) => Verdict::Rejected,
            Err(why) => Verdict::Malformed(format!("ark-gm17 verify_proof: {why}")),
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Scalar::one())))
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("ark-gm17 panicked: {text}"),
        (_, Some(text)) => format!("ark-gm17 panicked: {text}"),
        _ => "ark-gm17 panicked".to_string(),
    }
}
