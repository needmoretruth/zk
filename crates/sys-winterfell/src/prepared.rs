//! One statement ready to prove: Winterfell's STARK behind the museum's contract.

use std::sync::Arc;

use winter_math::FieldElement;
use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::air::{self, MAX_WIRE_COLUMNS, Statement, TRACE_LENGTH};
use crate::field::{self, F128, Val};
use crate::stark;

/// A statement's circuit and its wide AIR. A STARK has no keys, so nothing else is kept.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<F128>,
    wide: Arc<WideAir<F128>>,
}

impl Ready {
    /// Builds the circuit over f128 and lowers it to a wide AIR.
    ///
    /// Winterfell has no setup: the "setup" is fixing the hash and protocol parameters, which
    /// anyone can reproduce, so this only builds and lowers the circuit.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<F128>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let wide = WideAir::from_circuit(&circuit);
        if wide.num_columns() > MAX_WIRE_COLUMNS {
            let columns = wide.num_columns();
            return Err(SystemError::Failed(format!(
                "{columns} wire columns exceed the {MAX_WIRE_COLUMNS} a Winterfell proof can carry"
            )));
        }
        control.checkpoint()?;
        Ok(Self { example, circuit, wide: Arc::new(wide) })
    }

    /// Evaluates `assignment` without checking it and hands the row to the real prover.
    fn prove_values(
        &self,
        assignment: &Assignment<F128>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let row: Vec<Val> = self.wide.row(&evaluation.values).iter().map(|v| v.0).collect();
        let public: Vec<Val> = assignment.public.iter().map(|v| v.0).collect();
        control.checkpoint()?;
        let proof = stark::prove(Statement::new(Arc::clone(&self.wide), public), &row)?;
        let encode = |values: &[F128]| values.iter().map(|value| field::encode(value.0)).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

/// Decodes canonical field bytes, naming which input failed.
fn decode_all(values: &[FieldBytes], what: &str) -> Result<Vec<F128>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            field::decode(bytes)
                .map(F128)
                .ok_or_else(|| format!("{what} input {index} is not a canonical f128 element"))
        })
        .collect()
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::WideAir,
            counts: vec![
                ("constraints".into(), air::num_transitions(&self.wide) as u64),
                ("columns".into(), air::trace_width(&self.wide) as u64),
                ("rows".into(), TRACE_LENGTH as u64),
                ("public-inputs".into(), self.wide.public_columns().len() as u64),
            ],
        }
    }

    /// No setup material exists: a Winterfell proof needs only the public parameters above.
    fn setup_bytes(&self) -> Option<u64> {
        None
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with the AIR of {built}"
            )));
        }
        let assignment = instance.example.instance::<F128>(instance.kind, &instance.seed);
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
        let expected = self.wide.public_columns().len();
        if values.len() != expected {
            let got = values.len();
            return Ok(Verdict::Malformed(format!("expected {expected} public inputs, got {got}")));
        }
        let values = values.iter().map(|value| value.0).collect();
        let statement = Statement::new(Arc::clone(&self.wide), values);
        Ok(stark::verify(statement, air::trace_width(&self.wide), proof))
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = field::decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical f128 element".into())
        })?;
        Ok(field::encode(value + Val::ONE))
    }

    /// The low byte of the first out-of-domain trace value, the first column at the point `z`.
    ///
    /// The verifier reads it twice: it evaluates every transition constraint on it and compares the
    /// result with the committed composition polynomial at `z`, and it hashes it into the public
    /// coin that draws the DEEP coefficients. It is stored as 16 little-endian bytes, so flipping
    /// the lowest bit moves the value by one and it still decodes (unless the value was `p − 1`).
    /// When the bytes do not decode at all the middle byte is used, which only happens for bytes
    /// this crate did not produce.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        stark::ood_trace_offset(proof).unwrap_or(proof.len() / 2)
    }

    /// The default canonical forms plus the form Winterfell's serializer writes for an f128
    /// element, which is where a trace value appears in this proof: an opened trace query or an
    /// out-of-domain value of a constant column. For f128 the upstream form equals the canonical
    /// little-endian form, so it is added only if it differs.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        let mut big_endian = secret.clone();
        big_endian.reverse();
        let mut forms = vec![secret.clone(), big_endian];
        if let Some(upstream) = field::decode(secret).map(field::serialized)
            && !forms.contains(&upstream)
        {
            forms.push(upstream);
        }
        forms
    }
}
