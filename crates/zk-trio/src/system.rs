//! Trio in the museum's harness: prepare builds the statement, prove makes the 109-round
//! non-interactive proof, verify checks it.

use zk_circuit::{Assignment, ZkField};
use zk_core::catalog::SystemMeta;
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, ProofSystem, Proven,
    ShapeForm, SystemError, Verdict,
};

use crate::coins::Coins;
use crate::field::Fp;
use crate::meta::META;
use crate::program::Statement;
use crate::proof::{TAMPER_OFFSET, prove, verify};

/// The Trio exhibit; stateless, since a statement is all it needs and there are no keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct Trio;

impl ProofSystem for Trio {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        control.checkpoint()?;
        Ok(Box::new(Ready { example, statement: Statement::new(example)? }))
    }
}

/// One example's statement, compiled for the three friends.
struct Ready {
    example: ExampleId,
    statement: Statement,
}

fn decode_all(values: &[FieldBytes]) -> Result<Vec<Fp>, String> {
    values.iter().map(|bytes| Fp::decode(bytes)).collect()
}

impl Ready {
    /// Proves one assignment of the prepared statement, true or not; a false one is left for the
    /// verifier to turn down.
    fn prove_assigned(
        &self,
        assignment: &Assignment<Fp>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let claim = self.statement.claim(assignment)?;
        let proof = prove(&self.statement, &claim, &mut Coins::os(), control)?;
        let encode = |values: &[Fp]| values.iter().map(|v| v.to_le_bytes()).collect();
        Ok(Proven {
            proof,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        let statement = &self.statement;
        CircuitShape {
            form: ShapeForm::Native,
            counts: vec![
                ("gates".into(), statement.circuit().gates().len() as u64),
                ("multiplications".into(), statement.multiplications() as u64),
                ("assertions".into(), statement.assertions() as u64),
                ("witness values".into(), statement.witness_values() as u64),
            ],
        }
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        if instance.example != self.example {
            let (prepared, asked) = (self.example.id(), instance.example.id());
            return Err(SystemError::Failed(format!("prepared for {prepared}, asked for {asked}")));
        }
        let assignment = instance.example.instance::<Fp>(instance.kind, &instance.seed);
        self.prove_assigned(&assignment, control)
    }

    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let decoded = decode_all(public).and_then(|public| Ok((public, decode_all(private)?)));
        let (public, private) = decoded.map_err(SystemError::Failed)?;
        self.prove_assigned(&Assignment { public, private }, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        match decode_all(public) {
            Ok(public) => Ok(verify(&self.statement, &public, proof, control)?),
            Err(why) => Ok(Verdict::Malformed(why)),
        }
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = Fp::decode(value).map_err(SystemError::Failed)?;
        Ok(value.add(Fp::one()).to_le_bytes())
    }

    /// See [`TAMPER_OFFSET`]: the first byte of the first round's opening, a card seed the
    /// verifier hashes and compares with its commitment.
    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET
    }
}
