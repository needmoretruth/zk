//! What Ligero declares about itself.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// Ligero's catalogue entry, checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "ligero",
    name: "Ligero",
    shelf: Shelf::Others,
    year: 2017,
    authors: &["Scott Ames", "Carmit Hazay", "Yuval Ishai", "Muthuramakrishnan Venkitasubramaniam"],
    paper: Some(Paper {
        title: "Ligero: Lightweight Sublinear Arguments Without a Trusted Setup",
        venue: "CCS 2017",
        url: "https://eprint.iacr.org/2022/1608",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::SquareRoot,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Goldilocks",
    implementation: Implementation::Teaching { built_on: &["p3-goldilocks", "p3-dft", "sha2"] },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://ageverification.dev/av-doc-technical-specification/docs/annexes/annex-B/annex-B-zkp/",
        "https://github.com/google/longfellow-zk",
    ],
    deployments: &[Deployment {
        project: "Google Wallet (Longfellow ZK)",
        since: "2025-04",
        until: None,
    }],
};
