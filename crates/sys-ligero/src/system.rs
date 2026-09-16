//! Ligero in the museum's harness: prepare lowers the statement and fixes the code, prove encodes
//! and commits, verify checks the opened columns.

use zk_circuit::{Assignment, ZkField};
use zk_core::catalog::SystemMeta;
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, ProofSystem, Proven,
    ShapeForm, SystemError, Verdict,
};

use crate::field::Fp;
use crate::meta::META;
use crate::params::OPENED_COLUMNS;
use crate::proof::tamper_offset;
use crate::prover::prove;
use crate::statement::Statement;
use crate::verifier::verify;

/// The Ligero exhibit; stateless, since there is no setup beyond the circuit's shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ligero;

impl ProofSystem for Ligero {
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

/// One example's statement with its code fixed.
struct Ready {
    example: ExampleId,
    statement: Statement,
}

fn decode_all(values: &[FieldBytes]) -> Result<Vec<Fp>, String> {
    values.iter().map(|bytes| Fp::decode(bytes)).collect()
}

impl Ready {
    /// Proves one assignment, true or not; a false one is left for the verifier to turn down.
    fn prove_assigned(
        &self,
        assignment: &Assignment<Fp>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let claim = self.statement.claim(assignment)?;
        let proof = prove(&self.statement, &claim, control)?;
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
        let params = statement.params();
        let counts = [
            ("constraints", statement.r1cs().num_constraints()),
            ("variables", statement.r1cs().num_variables()),
            ("quadratic-constraints", statement.quadratic_constraints()),
            ("linear-constraints", statement.linear_constraints()),
            ("rows", params.rows()),
            ("message-length", params.message_length),
            ("columns", params.length),
            ("opened-columns", OPENED_COLUMNS),
        ];
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: counts.iter().map(|(name, count)| ((*name).into(), *count as u64)).collect(),
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

    /// See [`tamper_offset`]: the first byte of the first opened column's first entry, which the
    /// verifier hashes into that column's Merkle leaf and compares, through the siblings, with the
    /// committed root.
    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        tamper_offset(&self.statement.params())
    }
}
