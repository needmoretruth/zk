//! What the museum says about Marlin.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// Marlin's catalogue entry; the comparison table and the README are generated from these values.
pub static META: SystemMeta = SystemMeta {
    id: "marlin",
    name: "Marlin",
    shelf: Shelf::Others,
    year: 2019,
    authors: &[
        "Alessandro Chiesa",
        "Yuncong Hu",
        "Mary Maller",
        "Pratyush Mishra",
        "Psi Vesely",
        "Nicholas Ward",
    ],
    paper: Some(Paper {
        title: "Marlin: Preprocessing zkSNARKs with Universal and Updatable SRS",
        venue: "EUROCRYPT 2020",
        url: "https://eprint.iacr.org/2019/1047",
    }),
    trusted_setup: TrustedSetup::Universal,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BLS12-381",
    implementation: Implementation::Upstream {
        name: "ark-marlin",
        version: "0.3.0",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/arkworks-rs/marlin",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://aleo.org/post/announcing-aleo-mainnet/",
        "https://github.com/ProvableHQ/snarkVM",
    ],
    deployments: &[Deployment {
        project: "Aleo (Varuna, a Marlin variant)",
        since: "2024-09-18",
        until: None,
    }],
};
