//! One example indexed against a fresh universal SRS: prove, verify and the attack hooks.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use ark_bls12_381::Fr as Scalar;
use ark_ff::One;
use ark_marlin::{IndexVerifierKey, Proof};
use ark_serialize::CanonicalDeserialize;
use rand_core::OsRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, Fr};
use crate::prover::{self, ProverThread, panic_message};
use crate::setup::{Kzg, SRS_BYTES_COUNT, Upstream};

/// Where the flip attack lands: byte 577, the least significant byte of the proof's first claimed
/// evaluation, `a_denom(γ)`.
///
/// arkworks serializes a Marlin proof as `commitments ‖ evaluations ‖ prover_messages ‖ pc_proof`,
/// each `Vec` behind an 8-byte length. The commitments are three rounds of four, three and two
/// `MarlinKZG10` commitments: a 48-byte compressed G1 point and a one-byte option flag, plus a second
/// 48-byte point for the two degree-bounded ones (`g_1`, `g_2`). That is
/// `8 + (8 + 4·49) + (8 + 3·49 + 48) + (8 + 2·49 + 48) = 569` bytes for any circuit, then 8 bytes of
/// length, so the evaluations start at 577. They are sorted by label, and `a_denom` comes first.
///
/// A byte there, unlike one inside a curve point, still decodes: a canonical little-endian scalar
/// with its low bit flipped is another canonical scalar. So the flipped proof reaches the whole
/// verifier: the evaluation enters the Fiat–Shamir transcript that draws the opening challenge, the
/// inner sum-check's linear combination, and finally the batched KZG pairing check, which refuses a
/// value the committed polynomials do not take at `γ`.
pub const TAMPER_OFFSET: usize = 577;

/// A circuit lowered to R1CS with Marlin index keys derived for it from a fresh universal SRS.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Fr>,
    r1cs: Arc<R1cs<Fr>>,
    prover: ProverThread,
    verifier_key: IndexVerifierKey<Scalar, Kzg>,
    srs_bytes: u64,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, lowers it, and runs both of Marlin's setup steps with OS randomness.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<Fr>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = Arc::new(R1cs::from_circuit(&circuit));
        control.checkpoint()?;
        let indexed = prover::start(Arc::clone(&r1cs), control)?;
        control.checkpoint()?;
        Ok(Self {
            example,
            circuit,
            r1cs,
            prover: indexed.prover,
            verifier_key: indexed.verifier_key,
            srs_bytes: indexed.srs_bytes,
            setup_bytes: indexed.index_bytes,
        })
    }

    /// Evaluates `assignment` without checking it and hands every wire to ark-marlin's prover.
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
        let proof = self.prover.prove(z)?;
        let encode = |values: &[Fr]| values.iter().map(|value| field::encode(&value.0)).collect();
        Ok(Proven {
            proof,
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
                (SRS_BYTES_COUNT.into(), self.srs_bytes),
            ],
        }
    }

    /// Compressed `IndexProverKey::serialized_size`. ark-marlin's prover key carries the verifier
    /// key inside it, so this counts the index's proving and verifying material, each once. The
    /// universal SRS they were trimmed from is reported in the shape instead ([`SRS_BYTES_COUNT`]).
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
    /// claim: evaluated without checks, so a false assignment still reaches ark-marlin's prover.
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

    /// Decodes and runs `Marlin::verify`. ark-marlin indexes the proof's rounds and messages without
    /// checking how many there are, so a proof that decodes but lacks Marlin's shape panics inside
    /// the verifier; that panic is caught and reported as a proof that could not be read.
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
        let decoded = match Proof::<Scalar, Kzg>::deserialize(&mut reader) {
            Ok(decoded) => decoded,
            Err(why) => {
                return Ok(Verdict::Malformed(format!("ark-marlin Proof::deserialize: {why}")));
            }
        };
        if !reader.is_empty() {
            return Ok(Verdict::Malformed(format!("{} bytes follow the proof", reader.len())));
        }
        let verifier_key = &self.verifier_key;
        let checked = catch_unwind(AssertUnwindSafe(|| {
            Upstream::verify(verifier_key, &inputs, &decoded, &mut OsRng)
        }));
        Ok(match checked {
            Ok(Ok(true)) => Verdict::Accepted,
            Ok(Ok(false)) => Verdict::Rejected,
            Ok(Err(why)) => Verdict::Malformed(format!("ark-marlin verify: {why:?}")),
            Err(panic) => Verdict::Malformed(panic_message(panic.as_ref())),
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
