//! What ZKBoo declares about itself.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// ZKBoo's catalogue entry: the first practical MPC-in-the-head proof, trusting only a hash.
pub static META: SystemMeta = SystemMeta {
    id: "zkboo",
    name: "ZKBoo",
    shelf: Shelf::Others,
    year: 2016,
    authors: &["Irene Giacomelli", "Jesper Madsen", "Claudio Orlandi"],
    paper: Some(Paper {
        title: "ZKBoo: Faster Zero-Knowledge for Boolean Circuits",
        venue: "USENIX Security 2016",
        url: "https://eprint.iacr.org/2016/163",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Linear,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Goldilocks",
    implementation: Implementation::Teaching { built_on: &["p3-goldilocks", "sha2"] },
    status: Status::Historical,
    status_as_of: "2026-09",
    status_sources: &[
        "https://eprint.iacr.org/2016/163",
        "https://github.com/open-quantum-safe/liboqs/releases/tag/0.8.0",
    ],
    deployments: &[],
};
