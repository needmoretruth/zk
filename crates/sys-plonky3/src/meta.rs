//! What the museum's catalogue says about Plonky3.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Plonky3's catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `zero_knowledge` is `Optional` because `p3-fri` 0.7.0 ships a hiding commitment that has to be
/// chosen in the configuration, and this exhibit chooses it (see the crate documentation).
pub static META: SystemMeta = SystemMeta {
    id: "plonky3",
    name: "Plonky3 (uni-STARK)",
    shelf: Shelf::Polygon,
    year: 2024,
    authors: &["Polygon Zero"],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Optional,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Polylogarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BabyBear",
    implementation: Implementation::Upstream {
        name: "p3-uni-stark",
        version: "0.7.0",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/Plonky3/Plonky3",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://polygon.technology/blog/polygon-plonky3-the-next-generation-of-zk-proving-systems-is-production-ready",
        "https://github.com/Plonky3/Plonky3",
        "https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md",
    ],
    deployments: &[Deployment { project: "Miden VM", since: "2026-02-14", until: None }],
};
