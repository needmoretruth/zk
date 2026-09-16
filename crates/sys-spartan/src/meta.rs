//! What the museum says about Spartan.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Spartan's catalogue entry; the comparison table and the README are generated from these values.
pub static META: SystemMeta = SystemMeta {
    id: "spartan",
    name: "Spartan",
    shelf: Shelf::Others,
    year: 2019,
    authors: &["Srinath Setty"],
    paper: Some(Paper {
        title: "Spartan: Efficient and general-purpose zkSNARKs without trusted setup",
        venue: "CRYPTO 2020",
        url: "https://eprint.iacr.org/2019/550",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::SquareRoot,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "ristretto255 scalar",
    implementation: Implementation::Upstream {
        name: "spartan",
        version: "0.9.0",
        license: "MIT",
        repository: "https://github.com/microsoft/Spartan",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://github.com/microsoft/Spartan",
        "https://world.org/blog/engineering/provekit-privacy-for-the-real-world",
    ],
    deployments: &[],
};
