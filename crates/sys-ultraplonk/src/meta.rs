//! What the museum's catalogue says about UltraPlonk.

use zk_core::catalog::{
    Assumption, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};

/// The jellyfish commit `jf-plonk` is built from; `Cargo.toml` pins the same revision.
pub const JELLYFISH_REV: &str = "b1581c26f7195962d17fc5ff3768ac55db610ca1";

/// UltraPlonk's catalogue entry. Checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "ultraplonk",
    name: "UltraPlonk",
    shelf: Shelf::Aztec,
    year: 2020,
    authors: &["Ariel Gabizon", "Zachary J. Williamson"],
    paper: Some(Paper {
        title: "plookup: A simplified polynomial protocol for lookup tables",
        venue: "IACR ePrint 2020/315",
        url: "https://eprint.iacr.org/2020/315",
    }),
    trusted_setup: TrustedSetup::Universal,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BN254",
    implementation: Implementation::Upstream {
        name: "jellyfish jf-plonk",
        version: "b1581c2",
        license: "MIT",
        repository: "https://github.com/EspressoSystems/jellyfish",
    },
    status: Status::Superseded { by: "UltraHonk", since: "2025-05-20" },
    status_as_of: "2026-09",
    status_sources: &[
        "https://github.com/AztecProtocol/aztec-packages/pull/14205",
        "https://docs.zkverify.io/architecture/verification_pallets/ultraplonk",
    ],
    deployments: &[],
};
