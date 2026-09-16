//! Stand-in systems. The "proof" of the sound mock is a hash of the public inputs, made only after
//! the circuit accepts the witness: not zero-knowledge in any real sense, but its verifier rejects
//! exactly what a sound verifier must, which is what the harness needs to be tested against.

use core::marker::PhantomData;

use sha2::{Digest, Sha256};
use zk_circuit::{Circuit, ZkField};
use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Interaction, Prepared, ProofSystem,
    Proven, ShapeForm, Support, SystemError, Verdict,
};

pub const TAMPER_OFFSET: u64 = 5;
pub const ROUNDS: u32 = 12;
pub const UNSUPPORTED: &str = "mock-no-pool";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    Sound,
    AcceptsAnything,
    Leaky,
    Interactive,
}

pub struct Mock<F> {
    behaviour: Behaviour,
    field: PhantomData<F>,
}

impl<F> Mock<F> {
    pub fn new(behaviour: Behaviour) -> Self {
        Self { behaviour, field: PhantomData }
    }
}

const fn meta(mode: Mode) -> SystemMeta {
    SystemMeta {
        id: "mock",
        name: "Mock",
        shelf: Shelf::Homemade,
        year: 2026,
        authors: &[],
        paper: None,
        trusted_setup: TrustedSetup::None,
        zero_knowledge: ZeroKnowledge::No,
        assumptions: &[Assumption::Hash],
        proof_size: ProofSize::Constant,
        recursion: Recursion::None,
        mode,
        field: "BN254 scalar",
        implementation: Implementation::Homemade,
        status: Status::Experimental,
        status_as_of: "2026-09",
        status_sources: &[],
        deployments: &[],
    }
}

static NON_INTERACTIVE: SystemMeta = meta(Mode::NonInteractive);
static INTERACTIVE: SystemMeta = meta(Mode::Interactive);

impl<F: ZkField> ProofSystem for Mock<F> {
    fn meta(&self) -> &'static SystemMeta {
        if self.behaviour == Behaviour::Interactive { &INTERACTIVE } else { &NON_INTERACTIVE }
    }

    fn support(&self, example: ExampleId) -> Support {
        if example == ExampleId::PoolSpend {
            Support::Unsupported { reason: UNSUPPORTED }
        } else {
            Support::Full
        }
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        control.checkpoint()?;
        let circuit = example.circuit::<F>().map_err(|e| SystemError::Failed(format!("{e:?}")))?;
        Ok(Box::new(Ready { behaviour: self.behaviour, circuit }))
    }
}

struct Ready<F> {
    behaviour: Behaviour,
    circuit: Circuit<F>,
}

fn digest(public: &[FieldBytes]) -> Vec<u8> {
    let mut hasher = Sha256::new_with_prefix(b"mock");
    for value in public {
        hasher.update(value);
    }
    hasher.finalize().to_vec()
}

impl<F: ZkField> Ready<F> {
    fn encode(values: &[F]) -> Vec<FieldBytes> {
        values.iter().map(|v| v.to_le_bytes()).collect()
    }
}

impl<F: ZkField> Prepared for Ready<F> {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::Native,
            counts: vec![("gates".into(), self.circuit.gates().len() as u64)],
        }
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let assignment = instance.example.instance::<F>(instance.kind, &instance.seed);
        let public = Self::encode(&assignment.public);
        let secrets = Self::encode(&assignment.private);
        if self.behaviour == Behaviour::Sound {
            self.circuit
                .evaluate(&assignment)
                .map_err(|e| SystemError::Unsatisfied(format!("{e:?}")))?;
        }
        let mut proof = digest(&public);
        if self.behaviour == Behaviour::Leaky {
            proof.extend(secrets.iter().flatten());
        }
        Ok(Proven { proof, public, secrets })
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        _: &Control,
    ) -> Result<Verdict, SystemError> {
        if self.behaviour == Behaviour::AcceptsAnything {
            return Ok(Verdict::Accepted);
        }
        let expected = digest(public);
        Ok(if proof.get(..expected.len()) == Some(&expected[..]) {
            Verdict::Accepted
        } else {
            Verdict::Rejected
        })
    }

    fn interact(
        &mut self,
        instance: &Instance,
        public: Option<&[FieldBytes]>,
        _: &Control,
    ) -> Result<Interaction, SystemError> {
        let assignment = instance.example.instance::<F>(instance.kind, &instance.seed);
        let own = Self::encode(&assignment.public);
        let convinced =
            self.circuit.evaluate(&assignment).is_ok() && public.is_none_or(|p| p == own);
        Ok(Interaction {
            verdict: if convinced { Verdict::Accepted } else { Verdict::Rejected },
            transcript: vec![0xAA; ROUNDS as usize],
            rounds: ROUNDS,
            public: own,
            secrets: Self::encode(&assignment.private),
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let mut bumped = value.clone();
        bumped[0] = bumped[0].wrapping_add(1);
        Ok(bumped)
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        TAMPER_OFFSET as usize
    }
}
