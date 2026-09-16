//! What the museum's catalogue says about Schnorr proofs and sigma protocols.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};

/// The catalogue entry, checked against the listed sources on 2026-09-16.
pub static META: SystemMeta = SystemMeta {
    id: "schnorr",
    name: "Schnorr and sigma protocols",
    shelf: Shelf::Zcash,
    year: 1989,
    authors: &["Claus-Peter Schnorr"],
    paper: Some(Paper {
        title: "Efficient Identification and Signatures for Smart Cards",
        venue: "CRYPTO 1989",
        url: "https://doi.org/10.1007/0-387-34805-0_22",
    }),
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::DiscreteLog],
    proof_size: ProofSize::Linear,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "Pallas scalar",
    implementation: Implementation::Teaching { built_on: &["sigma-proofs", "pasta_curves"] },
    status: Status::Active,
    status_as_of: "2026-09",
    status_sources: &[
        "https://zips.z.cash/protocol/protocol.pdf",
        "https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki",
    ],
    deployments: &[
        Deployment { project: "Zcash Sapling (RedJubjub)", since: "2018-10-28", until: None },
        Deployment { project: "Bitcoin Taproot (BIP340)", since: "2021-11", until: None },
        Deployment { project: "Zcash Orchard (RedPallas)", since: "2022-05-31", until: None },
    ],
};
