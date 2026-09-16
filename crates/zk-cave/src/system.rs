//! The cave as an exhibit the shared harness can run and attack like every other system.

use zk_circuit::{Circuit, ZkField};
use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, InstanceKind, Interaction, Prepared,
    ProofSystem, Proven, ShapeForm, Stage, SystemError, Verdict,
};

use crate::cast::{OsRng, Prover};
use crate::field::Goldilocks;
use crate::modes::demonstration::{SCENES, demonstrate};
use crate::wall::{MagicWords, Wall};

/// Ali Baba's cave, run by the harness as a live demonstration of [`SCENES`] scenes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cave;

/// The cave's catalog entry.
pub static META: SystemMeta = SystemMeta {
    id: "cave",
    name: "Ali Baba's cave",
    shelf: Shelf::Homemade,
    year: 1989,
    authors: &["Jean-Jacques Quisquater", "Louis Guillou", "Thomas Berson"],
    paper: Some(Paper {
        title: "How to Explain Zero-Knowledge Protocols to Your Children",
        venue: "CRYPTO '89",
        url: "https://doi.org/10.1007/0-387-34805-0_60",
    }),
    trusted_setup: TrustedSetup::TrustedComponent,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::TrustedComponent],
    proof_size: ProofSize::Linear,
    recursion: Recursion::None,
    mode: Mode::Interactive,
    field: "Goldilocks",
    implementation: Implementation::Homemade,
    status: Status::Historical,
    status_as_of: "2026-09",
    status_sources: &["https://doi.org/10.1007/0-387-34805-0_60"],
    deployments: &[],
};

impl ProofSystem for Cave {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    /// Builds the example's circuit, the mechanism of the wall's lock. The lock is set to a claim's
    /// public inputs at each demonstration, because those depend on the instance's salts.
    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<Goldilocks>().map_err(|e| SystemError::Failed(e.to_string()))?;
        Ok(Box::new(PreparedCave { circuit }))
    }
}

struct PreparedCave {
    circuit: Circuit<Goldilocks>,
}

const NO_PROOF_OBJECT: &str = "the cave is a live demonstration and leaves no proof object";

impl Prepared for PreparedCave {
    fn shape(&self) -> CircuitShape {
        let gates = self.circuit.gates().len() as u64;
        CircuitShape { form: ShapeForm::Native, counts: vec![("gates".to_string(), gates)] }
    }

    fn prove(&mut self, _: &Instance, _: &Control) -> Result<Proven, SystemError> {
        Err(SystemError::NotApplicable(NO_PROOF_OBJECT))
    }

    fn verify(&mut self, _: &[FieldBytes], _: &[u8], _: &Control) -> Result<Verdict, SystemError> {
        Err(SystemError::NotApplicable(NO_PROOF_OBJECT))
    }

    /// Plays [`SCENES`] scenes with fresh operating-system randomness. The wall is locked to
    /// `public` when given, else to the instance's own claim; the prover whispers the instance's
    /// private inputs. A false claim, or words for a different claim, keep the wall shut, and the
    /// prover is caught at the first scene where the reporter calls the other side. A changed claim
    /// that the same words still satisfy opens the wall and convinces, as a fresh demonstration of
    /// a true claim must (`age` with the year one later).
    fn interact(
        &mut self,
        instance: &Instance,
        public: Option<&[FieldBytes]>,
        control: &Control,
    ) -> Result<Interaction, SystemError> {
        let claim = instance.example.instance::<Goldilocks>(instance.kind, &instance.seed);
        let secrets: Vec<FieldBytes> = claim.private.iter().map(|v| v.to_le_bytes()).collect();
        let lock = match public {
            None => claim.public,
            Some(bytes) => {
                let Some(values) = decode(bytes) else { return Ok(malformed(bytes, secrets)) };
                values
            }
        };
        let encoded_lock = lock.iter().map(|v| v.to_le_bytes()).collect();
        let wall = Wall::new(self.circuit.clone(), lock);
        let words = MagicWords::new(claim.private);
        let prover = match instance.kind {
            InstanceKind::Honest => Prover::mick_ali(words),
            InstanceKind::Dishonest => Prover::double(Some(words)),
        };
        let played = demonstrate(&wall, &prover, SCENES, &mut OsRng, |round| {
            control.checkpoint()?;
            control.report(Stage::Round { round, of: SCENES });
            Ok::<(), SystemError>(())
        })?;
        Ok(Interaction {
            verdict: if played.convinced() { Verdict::Accepted } else { Verdict::Rejected },
            transcript: played.tape().to_bytes(),
            rounds: u32::try_from(played.scenes.len()).unwrap_or(u32::MAX),
            public: encoded_lock,
            secrets,
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let element = Goldilocks::from_canonical_bytes(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical Goldilocks element".to_string())
        })?;
        Ok(element.add(Goldilocks::one()).to_le_bytes())
    }
}

/// Public inputs from canonical bytes; `None` if any is not a Goldilocks element.
fn decode(public: &[FieldBytes]) -> Option<Vec<Goldilocks>> {
    public.iter().map(|bytes| Goldilocks::from_canonical_bytes(bytes)).collect()
}

/// A demonstration that never started: the claim could not even be read.
fn malformed(public: &[FieldBytes], secrets: Vec<FieldBytes>) -> Interaction {
    Interaction {
        verdict: Verdict::Malformed("a public input is not a canonical Goldilocks element".into()),
        transcript: Vec::new(),
        rounds: 0,
        public: public.to_vec(),
        secrets,
    }
}
