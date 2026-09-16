//! What the museum's catalogue says about Halo 2.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// Halo 2's catalogue entry, checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "halo2",
    name: "Halo 2",
    shelf: Shelf::Zcash,
    year: 2020,
    authors: &["Sean Bowe", "Jack Grigg", "Daira-Emma Hopwood"],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::Logarithmic,
    recursion: Recursion::Accumulation,
    mode: Mode::NonInteractive,
    field: "Pasta (Pallas/Vesta)",
    implementation: Implementation::Upstream {
        name: "halo2_proofs",
        version: "0.3.5",
        license: "MIT OR Apache-2.0",
        repository: "https://github.com/zcash/halo2",
    },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &["https://zips.z.cash/zip-0224", "https://zips.z.cash/zip-0258"],
    deployments: &[
        Deployment { project: "Zcash Orchard", since: "2022-05-31", until: None },
        Deployment { project: "Zcash Ironwood", since: "2026-07-28", until: None },
    ],
};
