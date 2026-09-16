//! What the museum says about GM17.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// GM17's catalogue entry, checked against the listed sources on 2026-09-16; the comparison table
/// and the README are generated from these values.
pub static META: SystemMeta = SystemMeta {
    id: "gm17",
    name: "GM17",
    shelf: Shelf::Others,
    year: 2017,
    authors: &["Jens Groth", "Mary Maller"],
    paper: Some(Paper {
        title: "Snarky Signatures: Minimal Signatures of Knowledge from Simulation-Extractable SNARKs",
        venue: "CRYPTO 2017",
        url: "https://eprint.iacr.org/2017/540",
    }),
    trusted_setup: TrustedSetup::PerCircuit,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BLS12-381",
    implementation: Implementation::Upstream {
        name: "ark-gm17",
        version: "0.3.0",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/arkworks-rs/gm17",
    },
    status: Status::Historical,
    status_as_of: "2026-09",
    status_sources: &["https://eprint.iacr.org/2017/540", "https://github.com/Zokrates/ZoKrates"],
    deployments: &[],
};
