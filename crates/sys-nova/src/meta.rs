//! What the museum says about Nova.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Nova's catalogue entry; the comparison table and the README are generated from these values.
///
/// Checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "nova",
    name: "Nova",
    shelf: Shelf::Others,
    year: 2021,
    authors: &["Abhiram Kothapalli", "Srinath Setty", "Ioanna Tzialla"],
    paper: Some(Paper {
        title: "Nova: Recursive Zero-Knowledge Arguments from Folding Schemes",
        venue: "CRYPTO 2022",
        url: "https://eprint.iacr.org/2021/370",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::Logarithmic,
    recursion: Recursion::Folding,
    mode: Mode::NonInteractive,
    field: "Pallas scalar",
    implementation: Implementation::Upstream {
        name: "nova-snark",
        version: "0.76.0",
        license: "MIT",
        repository: "https://github.com/microsoft/Nova",
    },
    status: Status::Experimental,
    status_as_of: "2026-09",
    status_sources: &["https://github.com/microsoft/Nova", "https://crates.io/crates/nova-snark"],
    deployments: &[],
};
