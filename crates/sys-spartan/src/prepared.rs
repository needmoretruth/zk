//! One example with Spartan's generators derived and its matrices committed: prove, verify and the
//! attack hooks.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use curve25519_dalek::scalar::Scalar;
use libspartan::{
    ComputationCommitment, ComputationDecommitment, InputsAssignment, Instance, SNARK, SNARKGens,
    VarsAssignment,
};
use merlin::Transcript;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance as Claim, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, RistrettoScalar};
use crate::layout::Layout;
use crate::proof::{self, TAMPER_OFFSET};

/// The label both sides start their Merlin transcript with; prover and verifier must agree on it.
const TRANSCRIPT_LABEL: &[u8] = b"zk museum / spartan / snark";

/// A circuit lowered to R1CS, laid out for Spartan, with generators and a matrix commitment.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<RistrettoScalar>,
    r1cs: R1cs<RistrettoScalar>,
    layout: Layout,
    instance: Instance,
    generators: SNARKGens,
    commitment: ComputationCommitment,
    decommitment: ComputationDecommitment,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, hands the matrices to `Instance::new`, derives `SNARKGens` and runs
    /// `SNARK::encode`, Spartan's public preprocessing. Nothing here is secret or random.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<RistrettoScalar>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = R1cs::from_circuit(&circuit);
        let layout = Layout::of(&r1cs);
        let (cons, vars, inputs) = (layout.padded_cons, layout.padded_vars, layout.num_inputs);
        let instance = Instance::new(cons, vars, inputs, &layout.a, &layout.b, &layout.c)
            .map_err(|e| SystemError::Failed(format!("spartan Instance::new: {e:?}")))?;
        control.checkpoint()?;
        let generators = SNARKGens::new(cons, vars, inputs, layout.max_nonzero());
        control.checkpoint()?;
        let (commitment, decommitment) = SNARK::encode(&instance, &generators);
        let setup_bytes = proof::generators_size(&generators)?;
        Ok(Self {
            example,
            circuit,
            r1cs,
            layout,
            instance,
            generators,
            commitment,
            decommitment,
            setup_bytes,
        })
    }

    /// Evaluates `assignment` without checking it and hands every wire to `SNARK::prove`.
    fn prove_values(
        &self,
        assignment: &Assignment<RistrettoScalar>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let (vars, inputs) = self.layout.split(&self.r1cs.assignment(&evaluation.values));
        let upstream = |e| SystemError::Failed(format!("spartan Assignment::new: {e:?}"));
        let vars = VarsAssignment::new(&vars).map_err(upstream)?;
        let inputs = InputsAssignment::new(&inputs).map_err(upstream)?;
        control.checkpoint()?;
        let proved = catch_unwind(AssertUnwindSafe(|| {
            let mut transcript = Transcript::new(TRANSCRIPT_LABEL);
            SNARK::prove(
                &self.instance,
                &self.commitment,
                &self.decommitment,
                vars,
                &inputs,
                &self.generators,
                &mut transcript,
            )
        }));
        let snark = proved.map_err(|panic| SystemError::Unsatisfied(panic_message(&*panic)))?;
        let encode =
            |values: &[RistrettoScalar]| values.iter().map(|v| field::encode(&v.0)).collect();
        Ok(Proven {
            proof: proof::encode(&snark)?,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }

    fn decode_public(&self, public: &[FieldBytes]) -> Result<InputsAssignment, String> {
        let expected = self.layout.num_inputs;
        if public.len() != expected {
            return Err(format!("expected {expected} public inputs, got {}", public.len()));
        }
        let canonical = public
            .iter()
            .map(|bytes| field::decode(bytes).map(|scalar| scalar.to_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        InputsAssignment::new(&canonical).map_err(|e| format!("spartan Assignment::new: {e:?}"))
    }
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        let count = |value: usize| value as u64;
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![
                ("constraints".into(), count(self.layout.num_cons)),
                ("variables".into(), count(self.r1cs.num_variables())),
                ("padded-constraints".into(), count(self.layout.padded_cons)),
                ("padded-witness-variables".into(), count(self.layout.padded_vars)),
            ],
        }
    }

    /// Length of `SNARKGens` in bincode: the public generators, hashed from fixed labels. The matrix
    /// commitment `SNARK::encode` makes is not counted.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.setup_bytes)
    }

    fn prove(&mut self, claim: &Claim, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if claim.example != self.example {
            let (asked, built) = (claim.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with generators for {built}"
            )));
        }
        let assignment = claim.example.instance::<RistrettoScalar>(claim.kind, &claim.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: evaluated without checks, so a false assignment still reaches Spartan's prover.
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

    /// Runs `SNARK::verify`. Spartan's verifier returns an error for most failed checks but panics
    /// for some (a point that does not decompress is `unwrap`ped, a product-circuit claim is
    /// `assert_eq!`ed); either way the verifier refused the proof, so both are a rejection.
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
        let snark = match proof::decode(proof) {
            Ok(snark) => snark,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let checked = catch_unwind(AssertUnwindSafe(|| {
            let mut transcript = Transcript::new(TRANSCRIPT_LABEL);
            snark.verify(&self.commitment, &inputs, &mut transcript, &self.generators)
        }));
        Ok(match checked {
            Ok(Ok(())) => Verdict::Accepted,
            Ok(Err(_)) | Err(_) => Verdict::Rejected,
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Scalar::ONE)))
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET
    }

    /// Only the Montgomery form, the one form in which Spartan's serializer writes a field element.
    ///
    /// Every scalar in a Spartan proof is bincode's copy of Spartan's Montgomery limbs; nothing in
    /// the proof is a canonical scalar encoding, so the default canonical patterns could only match by
    /// coincidence, and for the museum's small secrets they do. The canonical encoding of a value from
    /// 2 to 16 is that byte beside 31 zeros. The part of the proof about the public matrices, which
    /// never sees the witness, holds zero scalars (32 zero bytes); one is preceded by the top byte of
    /// another scalar, at most 16 because `ℓ` is just above `2^252`, and some are followed by a small
    /// bincode length prefix. Such windows occur in every example's proof, for true and false claims
    /// alike, so searching for them would report the proof's layout as a leak.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        match field::decode(secret) {
            Ok(scalar) => vec![field::montgomery_bytes(&scalar)],
            Err(_) => vec![secret.clone()],
        }
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("spartan panicked: {text}"),
        (_, Some(text)) => format!("spartan panicked: {text}"),
        _ => "spartan panicked".to_string(),
    }
}
