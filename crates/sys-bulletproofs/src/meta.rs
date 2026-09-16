//! What the museum's catalogue says about Bulletproofs.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// The zkcrypto/bulletproofs commit this exhibit is built from; `Cargo.toml` pins the same revision.
pub const BULLETPROOFS_REV: &str = "04bce4e66013ff857ed462fd4206210544101461";

/// Bulletproofs' catalogue entry, checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "bulletproofs",
    name: "Bulletproofs",
    shelf: Shelf::Others,
    year: 2017,
    authors: &[
        "Benedikt Bünz",
        "Jonathan Bootle",
        "Dan Boneh",
        "Andrew Poelstra",
        "Pieter Wuille",
        "Greg Maxwell",
    ],
    paper: Some(Paper {
        title: "Bulletproofs: Short Proofs for Confidential Transactions and More",
        venue: "IEEE S&P 2018",
        url: "https://eprint.iacr.org/2017/1066",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::Logarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "ristretto255 scalar",
    implementation: Implementation::Upstream {
        name: "bulletproofs",
        version: "04bce4e",
        license: "MIT",
        repository: "https://github.com/zkcrypto/bulletproofs",
    },
    status: Status::Superseded { by: "Bulletproofs+", since: "2022-08-13" },
    status_as_of: "2026-09",
    status_sources: &[
        "https://www.getmonero.org/2018/10/11/monero-0.13.0-released.html",
        "https://web.getmonero.org/2022/04/20/network-upgrade-july-2022.html",
    ],
    deployments: &[Deployment {
        project: "Monero range proofs",
        since: "2018-10-18",
        until: Some("2022-08-13"),
    }],
};
