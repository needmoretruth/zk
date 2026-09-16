//! One example with Nova's public parameters and compression keys built: prove, verify and the
//! attack hooks.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use nova_snark::errors::NovaError;
use nova_snark::traits::snark::RelaxedR1CSSNARKTrait;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::config::{
    Compressed, NUM_STEPS, Params, Primary, ProvingKey, Recursive, Secondary, Snark, VerifyingKey,
};
use crate::field::{self, PallasScalar, Scalar};
use crate::proof::{self, TAMPER_OFFSET};
use crate::step::StepFunction;

/// A statement lowered to R1CS, with Nova's public parameters and the compressing SNARK's keys.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<PallasScalar>,
    rows: Arc<R1cs<PallasScalar>>,
    params: Params,
    proving_key: ProvingKey,
    verifying_key: VerifyingKey,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, runs `PublicParams::setup` on its step function and
    /// `CompressedSNARK::setup` on the parameters. The generators are hashed from labels; nothing
    /// here is secret.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<PallasScalar>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let rows = Arc::new(R1cs::from_circuit(&circuit));
        let shape = StepFunction::shape_only(Arc::clone(&rows));
        let params = upstream("PublicParams::setup", || {
            Params::setup(&shape, &*Snark::<Primary>::ck_floor(), &*Snark::<Secondary>::ck_floor())
        })
        .map_err(SystemError::Failed)?;
        control.checkpoint()?;
        let (proving_key, verifying_key) =
            upstream("CompressedSNARK::setup", || Compressed::setup(&params))
                .map_err(SystemError::Failed)?;
        let setup_bytes = proof::size(&params, "the public parameters")?
            + proof::size(&proving_key, "the prover key")?
            + proof::size(&verifying_key, "the verifier key")?;
        Ok(Self { example, circuit, rows, params, proving_key, verifying_key, setup_bytes })
    }

    /// Evaluates `assignment` without checking it, runs the two-step chain on it and compresses.
    ///
    /// nova-snark checks no witness on the way: `SatisfyingAssignment` records whatever values the
    /// step circuit allocates, folding combines instances without testing them, and Spartan proves
    /// whatever relaxed instance it is given. A false claim therefore still becomes a proof, and it is
    /// the verifier that turns it down. Any error or panic from those calls is reported as the
    /// prover's refusal all the same.
    fn prove_values(
        &self,
        assignment: &Assignment<PallasScalar>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let z = self.rows.assignment(&evaluation.values);
        let advice =
            z.iter().skip(1 + self.rows.num_public_inputs()).map(|value| value.0).collect();
        let step = StepFunction::with_advice(Arc::clone(&self.rows), advice);
        let z0: Vec<Scalar> = assignment.public.iter().map(|value| value.0).collect();
        control.checkpoint()?;
        let mut chain =
            upstream("RecursiveSNARK::new", || Recursive::new(&self.params, &step, &z0))
                .map_err(SystemError::Unsatisfied)?;
        for _ in 0..NUM_STEPS {
            control.checkpoint()?;
            upstream("RecursiveSNARK::prove_step", || chain.prove_step(&self.params, &step))
                .map_err(SystemError::Unsatisfied)?;
        }
        control.checkpoint()?;
        let compressed = upstream("CompressedSNARK::prove", || {
            Compressed::prove(&self.params, &self.proving_key, &chain)
        })
        .map_err(SystemError::Unsatisfied)?;
        let encode = |values: &[PallasScalar]| values.iter().map(|v| field::encode(&v.0)).collect();
        Ok(Proven {
            proof: proof::encode(&compressed)?,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }

    fn decode_public(&self, public: &[FieldBytes]) -> Result<Vec<Scalar>, String> {
        let expected = self.rows.num_public_inputs();
        if public.len() != expected {
            return Err(format!("expected {expected} public inputs, got {}", public.len()));
        }
        public.iter().map(|bytes| field::decode(bytes)).collect()
    }
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        let count = |value: usize| value as u64;
        let (primary, secondary) = self.params.num_constraints();
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![
                ("constraints".into(), count(self.rows.num_constraints())),
                ("variables".into(), count(self.rows.num_variables())),
                ("augmented-constraints".into(), count(primary)),
                ("secondary-constraints".into(), count(secondary)),
                ("steps".into(), count(NUM_STEPS)),
            ],
        }
    }

    /// bincode length of the public parameters (both commitment keys and both augmented circuits'
    /// shapes), the prover key and the verifier key, which repeats the padded primary and secondary
    /// shapes and the generators the inner-product argument opens against.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.setup_bytes)
    }

    fn prove(&mut self, claim: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if claim.example != self.example {
            let (asked, built) = (claim.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with public parameters for {built}"
            )));
        }
        let assignment = claim.example.instance::<PallasScalar>(claim.kind, &claim.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: evaluated without checks, so a false assignment still reaches Nova's prover.
    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let decode_all = |values: &[FieldBytes]| -> Result<Vec<PallasScalar>, SystemError> {
            values
                .iter()
                .map(|bytes| field::decode(bytes).map(PallasScalar).map_err(SystemError::Failed))
                .collect()
        };
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_values(&assignment, control)
    }

    /// Runs `CompressedSNARK::verify` for [`NUM_STEPS`] steps from `z0` = the public inputs, and
    /// accepts only if it succeeds and returns `z_n = z0`, the state an unchanged chain must end in.
    ///
    /// Every error Nova returns is a check that failed. Some checks are assertions instead
    /// (`batch_eval_verify` asserts the length of the proof's `evals_batch`), and a panic there is the
    /// verifier refusing the proof too, so both count as a rejection.
    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let z0 = match self.decode_public(public) {
            Ok(z0) => z0,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let compressed = match proof::decode(proof) {
            Ok(compressed) => compressed,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let checked = upstream("CompressedSNARK::verify", || {
            compressed.verify(&self.verifying_key, NUM_STEPS, &z0)
        });
        Ok(match checked {
            Ok(zn) if zn == z0 => Verdict::Accepted,
            Ok(_) | Err(_) => Verdict::Rejected,
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Scalar::from(1u64))))
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET
    }
}

/// Runs one nova-snark call, turning its error or its panic into a message that names the call.
fn upstream<T>(call: &str, run: impl FnOnce() -> Result<T, NovaError>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(format!("{call}: {error}")),
        Err(panic) => Err(format!("{call} panicked: {}", panic_text(panic.as_ref()))),
    }
}

/// The text a panic carried, when it carried text.
fn panic_text(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => (*text).to_string(),
        (_, Some(text)) => text.clone(),
        _ => "no message".to_string(),
    }
}
