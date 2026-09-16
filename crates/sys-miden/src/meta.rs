//! What the museum's catalogue says about Miden VM.

use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// The Miden VM catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `zero_knowledge` is `No` because the 0.32.1 proof configuration commits to the execution trace
/// without salt or random rows, and the prover draws no randomness (see the crate documentation).
pub static META: SystemMeta = SystemMeta {
    id: "miden",
    name: "Miden VM",
    shelf: Shelf::Polygon,
    year: 2021,
    authors: &["Bobbin Threadbare"],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::No,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Polylogarithmic,
    recursion: Recursion::Recursion,
    mode: Mode::NonInteractive,
    field: "Goldilocks",
    implementation: Implementation::Upstream {
        name: "miden-vm",
        version: "0.32.1",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/0xMiden/miden-vm",
    },
    status: Status::Experimental,
    status_as_of: "2026-09",
    status_sources: &["https://miden.xyz/", "https://github.com/0xMiden/miden-vm"],
    deployments: &[],
};
