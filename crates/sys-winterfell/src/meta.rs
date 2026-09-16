//! What the museum's catalogue says about Winterfell.

use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Winterfell's catalogue entry, checked against the listed sources on 2026-09-16.
///
/// `zero_knowledge` is `No`: `winter-prover` 0.13.1 extends and commits to the trace exactly as given,
/// with no random masking to switch on, and this crate's tests find the secrets in the proof bytes.
pub static META: SystemMeta = SystemMeta {
    id: "winterfell",
    name: "Winterfell",
    shelf: Shelf::Polygon,
    year: 2021,
    authors: &["Irakliy Khaburzaniya", "Kostas Chalkias", "Harjasleen Malvai", "Kevin Lewi"],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::No,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Polylogarithmic,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Winterfell f128",
    implementation: Implementation::Upstream {
        name: "winter-prover",
        version: "0.13.1",
        license: "MIT",
        repository: "https://github.com/facebook/winterfell",
    },
    status: Status::Superseded { by: "Plonky3", since: "2026-02-14" },
    status_as_of: "2026-09",
    status_sources: &[
        "https://github.com/0xMiden/miden-vm/blob/next/CHANGELOG.md",
        "https://github.com/facebook/winterfell",
    ],
    deployments: &[],
};
