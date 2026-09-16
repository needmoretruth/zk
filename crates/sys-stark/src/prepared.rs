//! One statement ready to prove: lambdaworks' STARK behind the museum's contract.

use std::sync::Arc;

use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::air::Statement;
use crate::field::{self, Stark252, Val};
use crate::stark::{self, TRACE_LENGTH};

/// A statement's circuit and its wide AIR. A STARK has no keys, so nothing else is kept.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Stark252>,
    wide: Arc<WideAir<Stark252>>,
}

impl Ready {
    /// Builds the circuit over Stark252 and lowers it to a wide AIR.
    ///
    /// A STARK has no setup: the "setup" is fixing the hash and protocol parameters, which anyone
    /// can reproduce, so this only builds and lowers the circuit.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<Stark252>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let wide = WideAir::from_circuit(&circuit);
        control.checkpoint()?;
        Ok(Self { example, circuit, wide: Arc::new(wide) })
    }

    /// Evaluates `assignment` without checking it and hands the row to the real prover.
    fn prove_values(
        &self,
        assignment: &Assignment<Stark252>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let row: Vec<Val> = self.wide.row(&evaluation.values).iter().map(|v| v.0).collect();
        let public: Vec<Val> = assignment.public.iter().map(|v| v.0).collect();
        control.checkpoint()?;
        let proof = stark::prove(&Statement::new(Arc::clone(&self.wide), public), &row)?;
        let encode = |values: &[Stark252]| values.iter().map(|v| field::encode(&v.0)).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

/// Decodes canonical field bytes, naming which input failed.
fn decode_all(values: &[FieldBytes], what: &str) -> Result<Vec<Stark252>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            field::decode(bytes)
                .map(Stark252)
                .ok_or_else(|| format!("{what} input {index} is not a canonical Stark252 element"))
        })
        .collect()
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::WideAir,
            counts: vec![
                ("constraints".into(), self.wide.num_constraints() as u64),
                ("columns".into(), self.wide.num_columns() as u64),
                ("rows".into(), TRACE_LENGTH as u64),
                ("public-inputs".into(), self.wide.public_columns().len() as u64),
            ],
        }
    }

    /// No setup material exists: a STARK proof needs only the public parameters in `stark`.
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
        let assignment = instance.example.instance::<Stark252>(instance.kind, &instance.seed);
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
        Ok(stark::verify(&Statement::new(Arc::clone(&self.wide), values), proof))
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = field::decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical Stark252 element".into())
        })?;
        Ok(field::encode(&(value + Val::one())))
    }

    /// The lowest byte of the first out-of-domain trace value, the first column at the point `z`.
    ///
    /// The verifier reads it three times: it hashes it into the transcript that draws the DEEP and
    /// FRI challenges, it evaluates the constraints on the out-of-domain row and compares the result
    /// with the composition polynomial's claimed value at `z`, and it subtracts it from every opened
    /// trace value of that column when it rebuilds the DEEP composition polynomial. It is stored as
    /// the element's Montgomery form in 32 big-endian bytes, which lambdaworks reads back without
    /// comparing it to the modulus, so flipping the lowest bit moves the stored integer by one and
    /// the proof still decodes. When the bytes do not decode at all the middle byte is used, which
    /// only happens for bytes this crate did not produce.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        stark::ood_trace_offset(proof).unwrap_or(proof.len() / 2)
    }

    /// The default canonical forms plus lambdaworks' serialized form, the Montgomery form in 32
    /// big-endian bytes, which is where a trace value appears in this proof: an opened trace row or
    /// an out-of-domain value of a constant column.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        let mut big_endian = secret.clone();
        big_endian.reverse();
        let mut forms = vec![secret.clone(), big_endian];
        if let Some(upstream) = field::decode(secret).map(|value| field::serialized(&value))
            && !forms.contains(&upstream)
        {
            forms.push(upstream);
        }
        forms
    }
}
