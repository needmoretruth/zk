//! What Trio declares about itself.

use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Trio's catalogue entry: designed for this museum in 2026, trusting only a hash.
pub static META: SystemMeta = SystemMeta {
    id: "trio",
    name: "Trio",
    shelf: Shelf::Homemade,
    year: 2026,
    authors: &[],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Linear,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Goldilocks",
    implementation: Implementation::Homemade,
    status: Status::Experimental,
    status_as_of: "2026-09",
    status_sources: &[],
    deployments: &[],
};
