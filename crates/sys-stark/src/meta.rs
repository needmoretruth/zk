//! What the museum's catalogue says about STARK.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// STARK's catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `zero_knowledge` is `No`: lambdaworks' prover commits to the trace exactly as given, with no
/// random masking to switch on, and this crate's tests find the secrets in the proof bytes. The
/// `version` is the short form of the commit `Cargo.toml` pins.
pub static META: SystemMeta = SystemMeta {
    id: "stark",
    name: "STARK",
    shelf: Shelf::Others,
    year: 2018,
    authors: &["Eli Ben-Sasson", "Iddo Bentov", "Yinon Horesh", "Michael Riabzev"],
    paper: Some(Paper {
        title: "Scalable, transparent, and post-quantum secure computational integrity",
        venue: "IACR ePrint 2018/046",
        url: "https://eprint.iacr.org/2018/046",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::No,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Polylogarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Stark252",
    implementation: Implementation::Upstream {
        name: "lambdaworks stark",
        version: "3a87850",
        license: "Apache-2.0",
        repository: "https://github.com/lambdaclass/lambdaworks",
    },
    status: Status::Superseded { by: "S-two (Circle STARK)", since: "2025-11-03" },
    status_as_of: "2026-09",
    status_sources: &[
        "https://www.starknet.io/blog/s-two-is-live-on-starknet-mainnet-the-fastest-prover-for-a-more-private-future/",
        "https://github.com/starkware-libs/stone-prover",
    ],
    deployments: &[
        Deployment { project: "StarkEx (Stone)", since: "2020-06", until: None },
        Deployment { project: "Starknet (Stone)", since: "2021-11", until: Some("2025-11-03") },
    ],
};
