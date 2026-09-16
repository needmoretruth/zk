//! What one run found, in a form a screen can draw and a companion process can send as JSON.

use serde::{Deserialize, Serialize};

use crate::catalog::Mode;
use crate::system::{CircuitShape, Verdict};

/// The three attacks every system receives, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttackKind {
    /// Flip the lowest bit of one proof byte.
    FlipProofByte,
    /// Add one to the first public input and verify the unchanged proof against it.
    BumpPublicInput,
    /// Try to prove the example's false claim.
    DishonestWitness,
}

impl AttackKind {
    /// All three, in running order.
    pub const ALL: [AttackKind; 3] =
        [Self::FlipProofByte, Self::BumpPublicInput, Self::DishonestWitness];
}

/// How an attack ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "detail")]
pub enum AttackOutcome {
    /// The verifier said no.
    Rejected,
    /// The proof or input no longer decoded; also a rejection.
    Malformed(String),
    /// The prover checked the witness and would not prove the false claim.
    ProverRefused(String),
    /// The verifier was convinced. For the flip attack this can be malleability rather than a bug;
    /// for the other two it means the system is broken.
    Accepted,
    /// The attack does not apply (there is no proof object in a live conversation).
    NotApplicable(String),
}

impl AttackOutcome {
    /// Whether the system held.
    pub fn held(&self) -> bool {
        !matches!(self, Self::Accepted)
    }
}

/// One attack's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackReport {
    /// Which attack.
    pub kind: AttackKind,
    /// How it ended.
    pub outcome: AttackOutcome,
    /// The proof byte that was flipped, for [`AttackKind::FlipProofByte`].
    pub offset: Option<u64>,
}

/// Whether a secret turned up, byte for byte, inside the proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "inputs")]
pub enum SecretScan {
    /// No secret's encoding appears. A smoke detector, not a proof of zero knowledge.
    NotFound,
    /// These private inputs appear verbatim.
    Found(Vec<String>),
    /// Every secret was below 2^16, small enough to turn up in any proof by chance, so a search
    /// could prove nothing either way.
    Inconclusive,
}

/// Wall-clock time of each step, measured on this machine during this run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timings {
    /// Circuit building plus key or parameter generation.
    pub setup_micros: u64,
    /// Proving the honest claim (or the whole conversation, for interactive systems).
    pub prove_micros: u64,
    /// Verifying the honest proof (zero for interactive systems, where it is part of the conversation).
    pub verify_micros: u64,
}

/// Everything one run found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReport {
    /// System ID.
    pub system: String,
    /// Example ID.
    pub example: String,
    /// Field or curve.
    pub field: String,
    /// Proof object or live conversation.
    pub mode: RunMode,
    /// The circuit in the form the system consumed.
    pub shape: CircuitShape,
    /// Step times.
    pub timings: Timings,
    /// Proving and verifying material size, when the system has any.
    pub setup_bytes: Option<u64>,
    /// Proof (or transcript) size in bytes.
    pub proof_bytes: u64,
    /// The first bytes of the proof, for showing what a proof looks like.
    pub proof_head: Vec<u8>,
    /// Rounds played, for interactive systems.
    pub rounds: Option<u32>,
    /// The verifier's decision on the honest claim.
    pub verdict: Verdict,
    /// The attack set, empty when attacks were not asked for.
    pub attacks: Vec<AttackReport>,
    /// Whether a secret appears verbatim in the proof.
    pub secret_scan: SecretScan,
}

/// [`Mode`] in the report's own serializable form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunMode {
    /// See [`Mode::NonInteractive`].
    NonInteractive,
    /// See [`Mode::Interactive`].
    Interactive,
}

impl From<Mode> for RunMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::NonInteractive => Self::NonInteractive,
            Mode::Interactive => Self::Interactive,
        }
    }
}

impl RunReport {
    /// Whether the honest claim verified and every applicable attack failed.
    pub fn sound(&self) -> bool {
        self.verdict.accepted() && self.attacks.iter().all(|attack| attack.outcome.held())
    }
}

/// How many leading proof bytes a report keeps.
pub const PROOF_HEAD_BYTES: usize = 48;
