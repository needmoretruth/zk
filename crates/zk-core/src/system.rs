//! The contract every proof system implements.
//!
//! Thirty-odd systems from incompatible ecosystems have to be run, timed and attacked the same way.
//! They meet at bytes: a proof is `Vec<u8>` and a public input is its field element's canonical
//! little-endian encoding. Everything field-specific — building the circuit, proving, parsing —
//! stays inside the system's own crate, behind [`Prepared`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use zk_examples::{ExampleId, InstanceKind};

use crate::catalog::SystemMeta;
use crate::error::{RunError, SystemError};
use crate::harness;
use crate::report::RunReport;

/// A field element as canonical little-endian bytes, the one encoding every system agrees on.
pub type FieldBytes = Vec<u8>;

/// Which claim to prove, and the seed its fresh secrets come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The statement.
    pub example: ExampleId,
    /// True claim or false claim.
    pub kind: InstanceKind,
    /// Seed for salts ([`ExampleId::instance`]).
    pub seed: [u8; 32],
}

/// What proving produced, in the shared encoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proven {
    /// The proof object exactly as a verifier would receive it.
    pub proof: Vec<u8>,
    /// Public inputs in declaration order.
    pub public: Vec<FieldBytes>,
    /// Private inputs in declaration order, kept only so the harness can search the proof for them.
    pub secrets: Vec<FieldBytes>,
}

/// What a live conversation between prover and verifier produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interaction {
    /// The verifier's decision at the end of the conversation.
    pub verdict: Verdict,
    /// Everything a camera would have recorded, serialized; it convinces nobody else.
    pub transcript: Vec<u8>,
    /// Rounds actually played (a cheat is usually caught before the last).
    pub rounds: u32,
    /// Public inputs in declaration order.
    pub public: Vec<FieldBytes>,
    /// Private inputs in declaration order, for the transcript search.
    pub secrets: Vec<FieldBytes>,
}

/// A verifier's decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "detail")]
pub enum Verdict {
    /// The proof checks out.
    Accepted,
    /// The verifier ran and said no.
    Rejected,
    /// The proof or an input could not even be decoded; counts as a rejection.
    Malformed(String),
}

impl Verdict {
    /// Whether the verifier was convinced.
    pub fn accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// Whether a system proves an example as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Support {
    /// The very circuit every other system proves.
    Full,
    /// A reduced version of the statement; the key names a phrase explaining how it differs.
    Adapted {
        /// Phrase key for the explanation.
        note: &'static str,
    },
    /// Not proved; the key names a phrase explaining why.
    Unsupported {
        /// Phrase key for the reason.
        reason: &'static str,
    },
}

/// The form a circuit took inside a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeForm {
    /// Rank-1 constraints.
    R1cs,
    /// PLONKish gate rows with copy constraints.
    Plonkish,
    /// One-row AIR.
    WideAir,
    /// Noir's ACIR opcodes.
    Acir,
    /// A program for a virtual machine.
    Program,
    /// The system's own representation.
    Native,
}

/// How big the circuit was in the form the system consumed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitShape {
    /// The form.
    pub form: ShapeForm,
    /// Named counts, such as `("constraints", 34)`; the names are phrase keys.
    pub counts: Vec<(String, u64)>,
}

/// A step the harness is in, for a spinner that says what is happening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Building keys or parameters.
    Setup,
    /// Proving the honest claim.
    Prove,
    /// Verifying the honest proof.
    Verify,
    /// Running one of the attacks.
    Attack(crate::report::AttackKind),
    /// One round of a live conversation.
    Round {
        /// Round number, from 1.
        round: u32,
        /// Rounds planned.
        of: u32,
    },
}

/// Cancellation and progress, shared between the screen and a run on another thread.
#[derive(Clone, Default)]
pub struct Control {
    cancel: Arc<AtomicBool>,
    progress: Option<Arc<dyn Fn(Stage) + Send + Sync>>,
}

impl Control {
    /// A control nobody watches or cancels, for tests and scripts.
    pub fn new() -> Self {
        Self::default()
    }

    /// A control that reports every stage to `progress`.
    pub fn with_progress(progress: impl Fn(Stage) + Send + Sync + 'static) -> Self {
        Self { cancel: Arc::default(), progress: Some(Arc::new(progress)) }
    }

    /// Asks the run to stop at the next point where stopping is safe.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether a stop was asked for.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Returns [`SystemError::Cancelled`] if a stop was asked for; systems call this between steps.
    pub fn checkpoint(&self) -> Result<(), SystemError> {
        if self.is_cancelled() { Err(SystemError::Cancelled) } else { Ok(()) }
    }

    /// Tells whoever is watching which stage the run is in.
    pub fn report(&self, stage: Stage) {
        if let Some(progress) = &self.progress {
            progress(stage);
        }
    }
}

impl core::fmt::Debug for Control {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Control").field("cancelled", &self.is_cancelled()).finish()
    }
}

/// Options for one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunOptions {
    /// Seed for fresh secrets; `None` draws one from the operating system.
    pub seed: Option<[u8; 32]>,
    /// Whether to run the attack set after the honest proof.
    pub attacks: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self { seed: None, attacks: true }
    }
}

/// One proof system in the museum.
pub trait ProofSystem: Send + Sync {
    /// Everything countable about it.
    fn meta(&self) -> &'static SystemMeta;

    /// Whether it proves `example` as written. Most systems prove all seven.
    fn support(&self, _example: ExampleId) -> Support {
        Support::Full
    }

    /// Builds the circuit and whatever keys or parameters the system needs before proving.
    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError>;

    /// Proves, verifies and attacks `example`, timing each step.
    ///
    /// Every in-process system uses the shared harness; only systems that run in another process
    /// (companions) replace this.
    fn run(
        &self,
        example: ExampleId,
        options: &RunOptions,
        control: &Control,
    ) -> Result<RunReport, RunError> {
        harness::run(self, example, options, control)
    }
}

/// A system ready to prove one example: keys built, circuit fixed.
pub trait Prepared: Send {
    /// The circuit in the form this system consumed.
    fn shape(&self) -> CircuitShape;

    /// Size of the proving and verifying material, when the system has any worth counting.
    fn setup_bytes(&self) -> Option<u64> {
        None
    }

    /// Proves an instance. A system whose prover checks the witness may refuse a false claim with
    /// [`SystemError::Unsatisfied`]; one that does not must still produce bytes.
    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError>;

    /// Checks a proof against public inputs. Undecodable input is a [`Verdict::Malformed`], not an error.
    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError>;

    /// Proves an assignment given directly, for activities (the Toy Shielded Pool) that prove their own
    /// notes rather than a sample claim. Inputs are canonical field bytes in declaration order. Like
    /// [`Prepared::prove`], it must not pre-check the witness. Systems that do not offer it say so.
    fn prove_assignment(
        &mut self,
        _public: &[FieldBytes],
        _private: &[FieldBytes],
        _control: &Control,
    ) -> Result<Proven, SystemError> {
        Err(SystemError::NotApplicable("this system cannot prove a caller-supplied assignment yet"))
    }

    /// For interactive systems: plays the whole conversation. `public` replaces the instance's own
    /// public inputs when given, which is how the harness asks a verifier to check a different claim.
    fn interact(
        &mut self,
        _instance: &Instance,
        _public: Option<&[FieldBytes]>,
        _control: &Control,
    ) -> Result<Interaction, SystemError> {
        Err(SystemError::NotApplicable("non-interactive system"))
    }

    /// The same field element plus one, in this system's field.
    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError>;

    /// Which proof byte the flip attack alters: one inside an element the verifier actually reads.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        proof.len() / 2
    }

    /// Byte patterns under which a secret could appear in this system's proofs. The default covers
    /// canonical little- and big-endian; a system that serializes in another form adds that form.
    fn secret_encodings(&self, secret: &FieldBytes) -> Vec<Vec<u8>> {
        let mut big_endian = secret.clone();
        big_endian.reverse();
        vec![secret.clone(), big_endian]
    }
}
