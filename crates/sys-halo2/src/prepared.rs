//! One statement ready to prove: Halo 2's real prover and verifier behind the museum's contract.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use halo2_proofs::pasta::{EqAffine, Fp};
use halo2_proofs::plonk::{Error, SingleVerifier, create_proof, verify_proof};
use halo2_proofs::transcript::{Blake2bRead, Blake2bWrite, Challenge255};
use rand_core::OsRng;
use zk_circuit::Circuit;
use zk_circuit::lower::plonkish::Plonkish;
use zk_core::{
    CircuitShape, Control, FieldBytes, Instance, Prepared, Proven, ShapeForm, SystemError, Verdict,
};

use crate::circuit::{ADVICE_COLUMNS, FIXED_COLUMNS, INSTANCE_COLUMNS, TableCircuit};
use crate::field::{PastaFp, decode, encode};
use crate::setup::Keys;

/// Bytes of one scalar in the transcript.
const SCALAR_BYTES: usize = 32;

/// A statement's circuit, its table and its keys.
pub(crate) struct Ready {
    circuit: Circuit<PastaFp>,
    table: Arc<Plonkish<PastaFp>>,
    keys: Keys,
}

impl Ready {
    /// Bundles what [`crate::Halo2`]'s `prepare` built.
    pub(crate) fn new(
        circuit: Circuit<PastaFp>,
        table: Arc<Plonkish<PastaFp>>,
        keys: Keys,
    ) -> Self {
        Self { circuit, table, keys }
    }

    /// Runs `create_proof` and returns the transcript bytes, or the prover's refusal.
    fn create(&self, cells: Vec<[PastaFp; 3]>, public: &[Fp]) -> Result<Vec<u8>, SystemError> {
        let circuit = TableCircuit::with_cells(Arc::clone(&self.table), cells);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut transcript = Blake2bWrite::<_, EqAffine, Challenge255<_>>::init(Vec::new());
            let instances: &[&[Fp]] = &[public];
            create_proof(
                &self.keys.params,
                &self.keys.pk,
                &[circuit],
                &[instances],
                OsRng,
                &mut transcript,
            )
            .map(|()| transcript.finalize())
        }));
        match outcome {
            Ok(Ok(proof)) => Ok(proof),
            Ok(Err(error)) => Err(SystemError::Unsatisfied(format!("create_proof: {error}"))),
            Err(panic) => Err(SystemError::Unsatisfied(panic_message(panic.as_ref()))),
        }
    }

    fn copy_constraints(&self) -> u64 {
        let links: usize = self.table.copy_classes().iter().map(|class| class.len() - 1).sum();
        (links + self.table.num_public_rows()) as u64
    }
}

// `secret_encodings` keeps the default: the transcript writes scalars as canonical little-endian
// `to_repr` bytes and points in compressed form, never in Montgomery form.
impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        let columns = ADVICE_COLUMNS + FIXED_COLUMNS + INSTANCE_COLUMNS;
        CircuitShape {
            form: ShapeForm::Plonkish,
            counts: vec![
                ("rows".into(), self.table.num_rows() as u64),
                ("columns".into(), columns as u64),
                ("copy-constraints".into(), self.copy_constraints()),
                ("k".into(), u64::from(self.keys.k)),
            ],
        }
    }

    /// The size of `Params::write`. halo2_proofs 0.3.5 has no serialization for its verifying or
    /// proving key, so neither is counted; both are regenerated from the circuit instead.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.keys.params_bytes)
    }

    /// Evaluates the circuit without checking it and hands every cell to `create_proof`, which
    /// does not check the witness either: a false claim still yields a transcript, and it is the
    /// verifier that turns it down.
    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let assignment = instance.example.instance::<PastaFp>(instance.kind, &instance.seed);
        let evaluation = self
            .circuit
            .evaluate_unchecked(&assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let cells = self.table.cell_values(&evaluation.values);
        let public: Vec<Fp> = assignment.public.iter().map(|value| value.0).collect();
        control.checkpoint()?;
        let proof = self.create(cells, &public)?;
        Ok(Proven {
            proof,
            public: public.into_iter().map(encode).collect(),
            secrets: assignment.private.iter().map(|value| encode(value.0)).collect(),
        })
    }

    /// `verify_proof` with a `SingleVerifier`, classifying its errors as Halo 2 reports them.
    ///
    /// Bytes that do not decode before the opening argument surface as `Error::Transcript` and are
    /// malformed. Inside the opening argument Halo 2 folds decoding failures into `Error::Opening`,
    /// the same error as a failed check, so those count as rejections: the adapter cannot tell
    /// them apart without re-parsing the transcript itself.
    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let Some(values) = public.iter().map(|bytes| decode(bytes)).collect::<Option<Vec<Fp>>>()
        else {
            return Ok(Verdict::Malformed("a public input is not a canonical Pasta Fp".into()));
        };
        if values.len() != self.table.num_public_rows() {
            return Ok(Verdict::Malformed(format!(
                "expected {} public inputs, got {}",
                self.table.num_public_rows(),
                values.len()
            )));
        }
        let strategy = SingleVerifier::new(&self.keys.params);
        let mut transcript = Blake2bRead::<_, EqAffine, Challenge255<_>>::init(proof);
        let instances: &[&[Fp]] = &[&values];
        let vk = self.keys.pk.get_vk();
        match verify_proof(&self.keys.params, vk, strategy, &[instances], &mut transcript) {
            Ok(()) => Ok(Verdict::Accepted),
            Err(Error::ConstraintSystemFailure | Error::Opening) => Ok(Verdict::Rejected),
            Err(Error::Transcript(error)) => Ok(Verdict::Malformed(error.to_string())),
            Err(other) => Err(SystemError::Failed(format!("verify_proof: {other}"))),
        }
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical Pasta Fp".into())
        })?;
        Ok(encode(value + Fp::from(1)))
    }

    /// The first byte of the inner-product argument's collapsed scalar `c`, the second-to-last
    /// element of every transcript (the last is the synthetic blinding factor).
    ///
    /// The verifier reads `c` into its final multi-scalar check, so flipping its lowest bit shifts
    /// it by one while keeping it a canonical scalar: the proof still parses, verification runs to
    /// the end, and the real equation fails. A byte in a curve point would mostly stop at decoding.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        proof.len().saturating_sub(2 * SCALAR_BYTES)
    }
}

fn panic_message(panic: &(dyn Any + Send)) -> String {
    let message = panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the prover panicked".to_string());
    format!("create_proof panicked: {message}")
}
