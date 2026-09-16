//! One statement ready to prove: the circle STARK behind the museum's contract.

use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::air::CircuitAir;
use crate::config::{self, Config};
use crate::field::{self, Mersenne31, Val};
use crate::stark::{self, TRACE_HEIGHT};

/// A statement's circuit, its AIR and the STARK configuration.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Mersenne31>,
    air: CircuitAir,
    config: Config,
}

impl Ready {
    /// Builds the circuit over Mersenne31, lowers it to a wide AIR, and builds the configuration.
    ///
    /// A STARK has no keys: the "setup" is choosing the hash, commitment and challenger, which
    /// anyone can reproduce. Without hiding there is not even private randomness to draw.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<Mersenne31>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let air = CircuitAir::new(WideAir::from_circuit(&circuit));
        control.checkpoint()?;
        Ok(Self { example, circuit, air, config: config::build() })
    }

    /// Evaluates `assignment` without checking it and hands the row to the real prover.
    fn prove_values(
        &self,
        assignment: &Assignment<Mersenne31>,
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
            |values: &[Mersenne31]| values.iter().map(|value| field::encode(value.0)).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

/// Decodes canonical field bytes, naming which input failed.
fn decode_all(values: &[FieldBytes], what: &str) -> Result<Vec<Mersenne31>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            field::decode(bytes).map(Mersenne31).ok_or_else(|| {
                format!("{what} input {index} is not a canonical Mersenne31 element")
            })
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
        let assignment = instance.example.instance::<Mersenne31>(instance.kind, &instance.seed);
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
            SystemError::Failed("a public input is not a canonical Mersenne31 element".into())
        })?;
        Ok(field::encode(value + Val::new(1)))
    }

    /// The low byte of the first opened trace value, `trace_local[0]` at the out-of-domain point.
    ///
    /// The verifier reads it twice: the circle FRI opening argument checks it against the trace
    /// commitment, and the constraint check at ζ uses it. It is stored as the canonical
    /// little-endian form of its first Mersenne31 coefficient, so flipping the lowest bit moves
    /// the value by one and it still decodes, except for the single value `p − 1`, which becomes
    /// `p` and fails to decode (a malformed proof, still a rejection). The proof parses, and the
    /// opening argument fails at its query proof-of-work check (measured on all seven statements:
    /// `InvalidPowWitness`): the changed value enters the Fiat–Shamir transcript before the
    /// prover's 16-bit grinding, so the witness no longer fits. When the bytes do not decode at
    /// all the middle byte is used, which only happens for bytes this crate did not produce.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        stark::trace_opening_offset(proof).unwrap_or(proof.len() / 2)
    }

    /// Only the form an opened trace value takes in the proof: a challenge-field element whose
    /// first coefficient is the secret's canonical 4 bytes and whose other two are zero.
    ///
    /// The bare canonical 4-byte forms of the default are left out: on a 31-bit field those
    /// forms of small secrets match structural bytes such as lengths and counts, and the
    /// 12-byte form is the one in which the opening at ζ writes the wire value.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        field::decode(secret).and_then(field::serialized).into_iter().collect()
    }
}
