//! One statement with its PLONK setup done: prove, verify and the attack hooks.

use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::default_types::FrElement;
use zk_circuit::lower::plonkish::Plonkish;
use zk_circuit::{Assignment, Circuit, ZkField};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{Fr, decode, encode};
use crate::setup::{self, Keys};
use crate::{layout, proof};

/// A circuit lowered to PLONKish rows, with a reference string and preprocessing made for it.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Fr>,
    table: Plonkish<Fr>,
    keys: Keys,
}

impl Ready {
    /// Builds the circuit, lowers it, and runs both phases of PLONK's setup.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<Fr>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let table = Plonkish::from_circuit(&circuit);
        control.checkpoint()?;
        let keys = setup::generate(&table, control)?;
        Ok(Self { example, circuit, table, keys })
    }

    /// Evaluates the circuit without checking it and hands every cell to lambdaworks' prover.
    fn prove_assigned(
        &self,
        assignment: &Assignment<Fr>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let cells = self.table.cell_values(&evaluation.values);
        let witness = layout::witness(&cells, self.keys.common.n);
        let public: Vec<FrElement> =
            assignment.public.iter().map(|value| value.element()).collect();
        control.checkpoint()?;
        let proof = proof::create(&self.keys, &witness, &public)?;
        Ok(Proven {
            proof,
            public: public.iter().map(encode).collect(),
            secrets: assignment.private.iter().map(|value| value.to_le_bytes()).collect(),
        })
    }

    fn copy_constraints(&self) -> u64 {
        let links: usize = self.table.copy_classes().iter().map(|class| class.len() - 1).sum();
        links as u64
    }
}

// `secret_encodings` keeps the default: a proof holds scalars as canonical big-endian bytes and
// point coordinates as canonical little-endian bytes, never in Montgomery form.
impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::Plonkish,
            counts: vec![
                ("rows".into(), self.table.num_rows() as u64),
                ("public-inputs".into(), self.table.num_public_rows() as u64),
                ("copy-constraints".into(), self.copy_constraints()),
                ("domain-size".into(), self.keys.common.n as u64),
            ],
        }
    }

    /// The reference string as `SRSManager::to_bytes` writes it plus `VerificationKey::as_bytes`.
    ///
    /// The common preprocessed input (selector and permutation polynomials), which lambdaworks'
    /// prover and verifier also read, is not counted: anyone holding the circuit recomputes it.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.keys.srs_bytes + self.keys.vk_bytes)
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if instance.example != self.example {
            let (asked, built) = (instance.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with a setup for {built}"
            )));
        }
        let assignment = instance.example.instance::<Fr>(instance.kind, &instance.seed);
        self.prove_assigned(&assignment, control)
    }

    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_assigned(&assignment, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let expected = self.table.num_public_rows();
        if public.len() != expected {
            let got = public.len();
            return Ok(Verdict::Malformed(format!("expected {expected} public inputs, got {got}")));
        }
        let Some(values) = public.iter().map(|bytes| decode(bytes)).collect::<Option<Vec<_>>>()
        else {
            return Ok(Verdict::Malformed(
                "a public input is not a canonical BLS12-381 scalar".into(),
            ));
        };
        proof::check(&self.keys, &values, proof)
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical BLS12-381 scalar".into())
        })?;
        Ok(encode(&(value + FrElement::one())))
    }

    /// See [`proof::TAMPER_OFFSET`].
    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        proof::TAMPER_OFFSET
    }
}

/// Canonical scalars in the circuit's field, refusing any encoding that is not canonical.
fn decode_all(values: &[FieldBytes]) -> Result<Vec<Fr>, SystemError> {
    values
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            decode(bytes).map(|value| Fr::new(&value)).ok_or_else(|| {
                SystemError::Failed(format!("input {index} is not a canonical BLS12-381 scalar"))
            })
        })
        .collect()
}
