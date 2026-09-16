//! What the museum says about Groth16.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// Groth16's catalogue entry; the comparison table and the README are generated from these values.
pub static META: SystemMeta = SystemMeta {
    id: "groth16",
    name: "Groth16",
    shelf: Shelf::Zcash,
    year: 2016,
    authors: &["Jens Groth"],
    paper: Some(Paper {
        title: "On the Size of Pairing-based Non-interactive Arguments",
        venue: "EUROCRYPT 2016",
        url: "https://eprint.iacr.org/2016/260",
    }),
    trusted_setup: TrustedSetup::PerCircuit,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BLS12-381",
    implementation: Implementation::Upstream {
        name: "bellman",
        version: "0.14.0",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/zkcrypto/bellman",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &["https://zips.z.cash/protocol/protocol.pdf"],
    deployments: &[Deployment { project: "Zcash Sapling", since: "2018-10-28", until: None }],
};
