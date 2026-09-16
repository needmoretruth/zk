//! What the museum's catalogue says about PLONK.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// The lambdaworks commit this exhibit is built from; `Cargo.toml` pins the same revision.
pub const LAMBDAWORKS_REV: &str = "3a87850dc405e12ab6c4bbaeba68d52815ab1158";

/// PLONK's catalogue entry, checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "plonk",
    name: "PLONK",
    shelf: Shelf::Aztec,
    year: 2019,
    authors: &["Ariel Gabizon", "Zachary J. Williamson", "Oana Ciobotaru"],
    paper: Some(Paper {
        title: "PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge",
        venue: "IACR ePrint 2019/953",
        url: "https://eprint.iacr.org/2019/953",
    }),
    trusted_setup: TrustedSetup::Universal,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BLS12-381",
    implementation: Implementation::Upstream {
        name: "lambdaworks plonk",
        version: LAMBDAWORKS_REV,
        license: "Apache-2.0",
        repository: "https://github.com/lambdaclass/lambdaworks",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://eprint.iacr.org/2019/953",
        "https://github.com/AztecProtocol/aztec-packages/pull/14205",
    ],
    deployments: &[],
};
