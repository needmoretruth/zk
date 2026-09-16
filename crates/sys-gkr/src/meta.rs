//! What the museum says about GKR.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// GKR's catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `proof_size` is logarithmic in the circuit's width; the proof also grows linearly with its depth,
/// one sum-check per layer.
pub static META: SystemMeta = SystemMeta {
    id: "gkr",
    name: "GKR",
    shelf: Shelf::Others,
    year: 2008,
    authors: &["Shafi Goldwasser", "Yael Tauman Kalai", "Guy N. Rothblum"],
    paper: Some(Paper {
        title: "Delegating Computation: Interactive Proofs for Muggles",
        venue: "STOC 2008",
        url: "https://doi.org/10.1145/2699436",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::Logarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BN254",
    implementation: Implementation::Upstream {
        name: "remainder",
        version: "5687fe7",
        license: "MIT OR Apache-2.0 WITH LLVM-exception",
        repository: "https://github.com/worldcoin/Remainder_CE",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://github.com/starkware-libs/stwo",
        "https://github.com/worldcoin/Remainder_CE",
    ],
    deployments: &[],
};
