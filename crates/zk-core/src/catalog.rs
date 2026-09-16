//! What a proof system declares about itself.
//!
//! Everything countable about a system lives here as an enum, not in prose: the README table and
//! the comparison screen are generated from these values and checked against them by tests, so a
//! sentence in a page can never quietly disagree with the table beside it. Prose (history,
//! strengths, weaknesses) lives in each system's page under `catalog/`.

use serde::Serialize;

/// The shelf a system stands on. A system stands on exactly one, the first that fits in this order.
///
/// The order is the owner's framing of the museum: everything Zcash used or built, then Aztec, then
/// Polygon, then everything else, then the two proofs written for this repository. Wider use is
/// recorded in [`SystemMeta::deployments`], not by standing on two shelves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shelf {
    /// Used or built by Zcash.
    Zcash,
    /// Built or used by Aztec.
    Aztec,
    /// Built or used by Polygon.
    Polygon,
    /// Everything else worth running.
    Others,
    /// Written for this repository.
    Homemade,
}

impl Shelf {
    /// Every shelf in display order.
    pub const ALL: [Shelf; 5] =
        [Self::Zcash, Self::Aztec, Self::Polygon, Self::Others, Self::Homemade];

    /// Stable key used in commands (`/list zcash`) and phrase tables.
    pub fn key(self) -> &'static str {
        match self {
            Self::Zcash => "zcash",
            Self::Aztec => "aztec",
            Self::Polygon => "polygon",
            Self::Others => "others",
            Self::Homemade => "homemade",
        }
    }
}

/// What has to be trusted before the first proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustedSetup {
    /// Nothing: public randomness is enough.
    None,
    /// One structured reference string for every circuit up to a size, which anyone can update.
    Universal,
    /// A fresh ceremony for every circuit.
    PerCircuit,
    /// A component everyone must simply trust, like the magic wall in Ali Baba's cave.
    TrustedComponent,
}

/// Whether a proof hides the witness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroKnowledge {
    /// Proofs reveal nothing beyond the statement.
    Yes,
    /// Proofs can reveal the witness; succinct is not the same as zero-knowledge.
    No,
    /// Hiding exists but has to be switched on; the museum says which setting it runs with.
    Optional,
}

/// What an attacker would have to break.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Assumption {
    /// Hardness assumptions on pairing-friendly curves; falls to a large quantum computer.
    Pairing,
    /// Discrete logarithm in a group; falls to a large quantum computer.
    DiscreteLog,
    /// Only a hash function (and the random-oracle model); believed to survive quantum computers.
    Hash,
    /// Lattice problems such as Module-SIS; believed to survive quantum computers.
    Lattice,
    /// A trusted component, not mathematics.
    TrustedComponent,
}

impl Assumption {
    /// Whether this assumption is believed to survive a large quantum computer.
    pub fn post_quantum(self) -> bool {
        matches!(self, Self::Hash | Self::Lattice)
    }
}

/// How proof size grows with the circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofSize {
    /// The same few group elements for any circuit.
    Constant,
    /// Logarithmic in the circuit size.
    Logarithmic,
    /// Polylogarithmic in the circuit size.
    Polylogarithmic,
    /// Square root of the circuit size.
    SquareRoot,
    /// Linear in the circuit size.
    Linear,
}

/// What the system offers for proving about proofs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Recursion {
    /// Nothing built in.
    None,
    /// A proof can verify another proof inside its circuit.
    Recursion,
    /// Verification work is deferred and accumulated (Halo's approach).
    Accumulation,
    /// Instances are folded together before any proof is made (Nova's approach).
    Folding,
}

/// Whether the verifier needs a live conversation or a proof object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// The prover hands over bytes that anyone can check later.
    NonInteractive,
    /// The verifier must take part; a recording convinces nobody else.
    Interactive,
}

/// Where the running code comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Implementation {
    /// The Rust implementation by the system's authors or the de facto standard one, used as is.
    Upstream {
        /// Crate or repository name.
        name: &'static str,
        /// Version or git revision the lockfile pins.
        version: &'static str,
        /// SPDX license expression.
        license: &'static str,
        /// Where its source lives.
        repository: &'static str,
    },
    /// Written for this repository on top of audited primitives, because no permissively licensed
    /// Rust implementation exists. Not audited; shown with a warning wherever it runs.
    Teaching {
        /// The primitive crates it is built on.
        built_on: &'static [&'static str],
    },
    /// Built in a separate workspace (a nightly compiler or a conflicting dependency) and run as
    /// its own executable, `nmtzk-<id>`.
    Companion {
        /// Directory under `companions/`.
        workspace: &'static str,
        /// What the upstream code is, as for [`Implementation::Upstream`].
        upstream: &'static str,
    },
    /// One of the two proofs designed for this repository.
    Homemade,
}

/// Where a system stands today. Always read together with [`SystemMeta::status_as_of`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Status {
    /// In production use.
    Active,
    /// Replaced in its main deployment.
    Superseded {
        /// What replaced it.
        by: &'static str,
        /// When, as `YYYY-MM` or `YYYY-MM-DD`.
        since: &'static str,
    },
    /// Its maintainers stopped it; the page says why.
    Discontinued {
        /// When, as `YYYY-MM` or `YYYY-MM-DD`.
        since: &'static str,
    },
    /// Built and published but not yet relied on in production.
    Experimental,
    /// Of historical and teaching importance; never deployed at scale.
    Historical,
}

/// A paper, so a reader can go to the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Paper {
    /// Title as published.
    pub title: &'static str,
    /// Venue and year, such as `"EUROCRYPT 2016"`.
    pub venue: &'static str,
    /// A stable address: ePrint, DOI or arXiv.
    pub url: &'static str,
}

/// A place the system has been used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Deployment {
    /// The project, such as `"Zcash Sapling"`.
    pub project: &'static str,
    /// When it started, as `YYYY` or `YYYY-MM-DD`.
    pub since: &'static str,
    /// When it ended, if it did.
    pub until: Option<&'static str>,
}

/// Everything countable about one proof system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SystemMeta {
    /// Permanent ID used in commands and file names (`groth16`). Never changes.
    pub id: &'static str,
    /// Display name (`Groth16`).
    pub name: &'static str,
    /// The one shelf it stands on.
    pub shelf: Shelf,
    /// Year it was first made public.
    pub year: u16,
    /// Authors of the defining paper or design.
    pub authors: &'static [&'static str],
    /// The defining paper, if there is one.
    pub paper: Option<Paper>,
    /// What has to be trusted before the first proof.
    pub trusted_setup: TrustedSetup,
    /// Whether proofs hide the witness, as this repository runs it.
    pub zero_knowledge: ZeroKnowledge,
    /// Everything an attacker would have to break (all of them hold for security).
    pub assumptions: &'static [Assumption],
    /// How proof size grows.
    pub proof_size: ProofSize,
    /// What it offers for proofs about proofs.
    pub recursion: Recursion,
    /// Proof object or live conversation.
    pub mode: Mode,
    /// The field or curve this repository runs it over.
    pub field: &'static str,
    /// Where the running code comes from.
    pub implementation: Implementation,
    /// Where it stands.
    pub status: Status,
    /// The month the status was last checked, `YYYY-MM`.
    pub status_as_of: &'static str,
    /// Primary sources for the status.
    pub status_sources: &'static [&'static str],
    /// Where it has been used.
    pub deployments: &'static [Deployment],
}

impl SystemMeta {
    /// Whether every assumption is believed to survive a large quantum computer.
    pub fn post_quantum(&self) -> bool {
        !self.assumptions.is_empty() && self.assumptions.iter().all(|a| a.post_quantum())
    }

    /// Whether this is code written for this repository rather than its authors' implementation.
    pub fn is_teaching(&self) -> bool {
        matches!(self.implementation, Implementation::Teaching { .. })
    }
}
