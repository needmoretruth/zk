//! One statement ready to prove: a compiled Miden program behind the museum's contract.

use std::collections::BTreeMap;

use miden_core::Felt;
use miden_processor::{Program, ProgramInfo};
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, Goldilocks};
use crate::{layout, program, vm};

/// Most values a program can receive on its operand stack, and so most public inputs.
const MAX_STACK_INPUTS: usize = 16;

/// A statement's circuit, its program and what the verifier needs of the program.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Goldilocks>,
    program: Program,
    info: ProgramInfo,
    labels: BTreeMap<u64, String>,
    operations: u64,
    trace_rows: Option<u64>,
}

impl Ready {
    /// Builds the circuit over Goldilocks, writes it as Miden Assembly and compiles it.
    ///
    /// Miden has no keys. The setup is compiling the program; the verifier keeps only
    /// `ProgramInfo`, the program's MAST root and its (empty) kernel.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<Goldilocks>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let public = circuit.public_inputs().len();
        if public > MAX_STACK_INPUTS {
            return Err(SystemError::Failed(format!(
                "{public} public inputs do not fit on Miden's {MAX_STACK_INPUTS}-element stack"
            )));
        }
        let source = program::write(&circuit);
        control.checkpoint()?;
        let program = vm::assemble(&source.text)?;
        let info = program.to_info();
        let operations = vm::operations(&program);
        control.checkpoint()?;
        let labels = source.labels;
        Ok(Self { example, circuit, program, info, labels, operations, trace_rows: None })
    }

    /// Evaluates `assignment` without checking it and hands the values to the real VM and prover.
    fn prove_values(
        &mut self,
        assignment: &Assignment<Goldilocks>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let advice = program::advice(&self.circuit, &evaluation.values);
        let public: Vec<Felt> = assignment.public.iter().map(|value| value.0).collect();
        control.checkpoint()?;
        let proof = vm::prove(&self.program, &public, advice, &self.labels)?;
        self.trace_rows = layout::trace_rows(&proof).or(self.trace_rows);
        let encode =
            |values: &[Goldilocks]| values.iter().map(|value| field::encode(value.0)).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

/// Decodes canonical field bytes, naming which input failed.
fn decode_all(values: &[FieldBytes], what: &str) -> Result<Vec<Goldilocks>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            field::decode(bytes).map(Goldilocks).ok_or_else(|| {
                format!("{what} input {index} is not a canonical Goldilocks element")
            })
        })
        .collect()
}

impl Prepared for Ready {
    /// The compiled program's operation count, and once a proof exists, the trace rows it covers.
    ///
    /// A straight-line program runs the same cycles whatever the witness, so the trace height of
    /// any proof of this program is the height of every proof of it; it is only known after
    /// execution, which is why it appears once something has been proved.
    fn shape(&self) -> CircuitShape {
        let mut counts = vec![("program-operations".to_string(), self.operations)];
        if let Some(rows) = self.trace_rows {
            counts.push(("trace-rows".to_string(), rows));
        }
        CircuitShape { form: ShapeForm::Program, counts }
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with the program of {built}"
            )));
        }
        let assignment = instance.example.instance::<Goldilocks>(instance.kind, &instance.seed);
        self.prove_values(&assignment, control)
    }

    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let assignment = Assignment {
            public: decode_all(public, "public").map_err(SystemError::Failed)?,
            private: decode_all(private, "private").map_err(SystemError::Failed)?,
        };
        self.prove_values(&assignment, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let values = match decode_all(public, "public") {
            Ok(values) => values,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let expected = self.circuit.public_inputs().len();
        if values.len() != expected {
            let got = values.len();
            return Ok(Verdict::Malformed(format!("expected {expected} public inputs, got {got}")));
        }
        let values: Vec<Felt> = values.iter().map(|value| value.0).collect();
        Ok(vm::verify(&self.info, &values, proof))
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = field::decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical Goldilocks element".into())
        })?;
        Ok(field::encode(value + Felt::ONE))
    }

    /// The low byte of the commitment to the main execution trace, the first commitment in the
    /// STARK transcript.
    ///
    /// The lifted STARK verifier receives this commitment before anything else in the proof and
    /// absorbs it into the Fiat–Shamir transcript, so every challenge after it — the auxiliary
    /// randomness, the constraint folding, the out-of-domain point, the DEEP and FRI challenges —
    /// is drawn from it, and the opening of the main trace is checked against it as the Merkle
    /// root. It is a raw Blake3 digest, so a flipped bit always still decodes and the proof
    /// reaches the cryptographic checks. When the bytes do not decode down to the STARK proof the
    /// middle byte is used, which only happens for bytes this crate did not produce.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        layout::main_trace_commitment_offset(proof).unwrap_or(proof.len() / 2)
    }
}
