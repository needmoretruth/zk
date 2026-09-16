//! One statement ready to prove: Plonky3's STARK behind the museum's contract.

use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::air::CircuitAir;
use crate::config::{self, Config};
use crate::field::{self, BabyBear, Val};
use crate::stark::{self, TRACE_HEIGHT};

/// A statement's circuit, its AIR and a STARK configuration with fresh hiding randomness.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<BabyBear>,
    air: CircuitAir,
    config: Config,
}

impl Ready {
    /// Builds the circuit over BabyBear, lowers it to a wide AIR, and builds the configuration.
    ///
    /// A STARK has no keys: the "setup" is choosing the hash, commitment and challenger, which
    /// anyone can reproduce, plus drawing the prover's private masking randomness.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<BabyBear>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let air = CircuitAir::new(WideAir::from_circuit(&circuit));
        control.checkpoint()?;
        let config = config::build()?;
        Ok(Self { example, circuit, air, config })
    }

    /// Evaluates `assignment` without checking it and hands the row to the real prover.
    fn prove_values(
        &self,
        assignment: &Assignment<BabyBear>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let row: Vec<Val> =
            self.air.wide().row(&evaluation.values).iter().map(|value| value.0).collect();
        let public: Vec<Val> = assignment.public.iter().map(|value| value.0).collect();
        control.checkpoint()?;
        let proof = stark::prove(&self.config, &self.air, &row, &public)?;
        let encode =
            |values: &[BabyBear]| values.iter().map(|value| field::encode(value.0)).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

/// Decodes canonical field bytes, naming which input failed.
fn decode_all(values: &[FieldBytes], what: &str) -> Result<Vec<BabyBear>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            field::decode(bytes)
                .map(BabyBear)
                .ok_or_else(|| format!("{what} input {index} is not a canonical BabyBear element"))
        })
        .collect()
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::WideAir,
            counts: vec![
                ("constraints".into(), self.air.num_asserted() as u64),
                ("columns".into(), self.air.wide().num_columns() as u64),
                ("rows".into(), TRACE_HEIGHT as u64),
            ],
        }
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with the AIR of {built}"
            )));
        }
        let assignment = instance.example.instance::<BabyBear>(instance.kind, &instance.seed);
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
        let expected = self.air.wide().public_columns().len();
        if values.len() != expected {
            let got = values.len();
            return Ok(Verdict::Malformed(format!("expected {expected} public inputs, got {got}")));
        }
        let values: Vec<Val> = values.iter().map(|value| value.0).collect();
        Ok(stark::verify(&self.config, &self.air, proof, &values))
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = field::decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical BabyBear element".into())
        })?;
        Ok(field::encode(value + Val::new(1)))
    }

    /// The low byte of the first opened trace value, `trace_local[0]` at the out-of-domain point.
    ///
    /// The verifier reads it twice: the FRI opening argument checks it against the trace
    /// commitment, and the constraint check at ζ uses it. It is stored as the little-endian
    /// Montgomery form of its first BabyBear coefficient, so flipping the lowest bit moves the
    /// value by one and it still decodes. The proof parses, and the opening argument fails at its
    /// query proof-of-work check (measured: `InvalidPowWitness`): the changed value enters the
    /// Fiat–Shamir transcript before the prover's 16-bit grinding, so the witness no longer fits.
    /// When the bytes do not decode at all the middle byte is used, which only happens for bytes
    /// this crate did not produce.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        stark::trace_opening_offset(proof).unwrap_or(proof.len() / 2)
    }

    /// Only the form an opened trace value takes in the proof: a challenge-field element whose
    /// first limb is the secret in Montgomery form and whose other three limbs are zero.
    ///
    /// The canonical 4-byte forms of the default are left out on purpose. Nothing in this proof
    /// format writes a field element canonically, and on a 31-bit field those forms of small
    /// secrets match structural bytes. In a hiding `sudoku` proof, `00 00 00 04` is three absent
    /// optional openings followed by the quotient-chunk count, not a sudoku cell.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        field::decode(secret).and_then(field::serialized).into_iter().collect()
    }
}
