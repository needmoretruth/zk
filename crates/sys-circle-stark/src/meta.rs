//! What the museum's catalogue says about Circle STARKs.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// The Circle STARK catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `zero_knowledge` is `No` because `p3-circle` 0.7.0's `CirclePcs` has no hiding option, so
/// this exhibit's proofs carry the witness (see the crate documentation).
pub static META: SystemMeta = SystemMeta {
    id: "circle-stark",
    name: "Circle STARK",
    shelf: Shelf::Polygon,
    year: 2024,
    authors: &["Ulrich Haböck", "David Levit", "Shahar Papini"],
    paper: Some(Paper {
        title: "Circle STARKs",
        venue: "IACR ePrint 2024/278",
        url: "https://eprint.iacr.org/2024/278",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::No,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Polylogarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Mersenne31",
    implementation: Implementation::Upstream {
        name: "p3-circle",
        version: "0.7.0",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/Plonky3/Plonky3",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &["https://l2beat.com/zk-catalog/stwo", "https://github.com/Plonky3/Plonky3"],
    deployments: &[Deployment { project: "Starknet (S-two)", since: "2025-11-03", until: None }],
};
