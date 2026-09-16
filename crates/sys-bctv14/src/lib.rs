//! BCTV14 (Pinocchio variant) — the preprocessing zk-SNARK Zcash Sprout ran from 2016 to 2018.
//!
//! Zcash launched with BCTV14 proofs: libsnark's `r1cs_ppzksnark`, a variant of Pinocchio/PGHR13,
//! over the BN254 pairing-friendly curve. libsnark is C++ and no permissively licensed pure-Rust
//! implementation exists, so this crate is a **teaching implementation** written from the papers on
//! top of arkworks primitives (ark-bn254, ark-ec, ark-ff, ark-poly). It is **not audited** and must
//! not guard anything real; it is here so the museum can run the real scheme and attack it.
//!
//! What it follows:
//! - R1CS → QAP over a multiplicative evaluation domain in the Lagrange basis, with the
//!   `num_inputs + 1` input-consistency constraints libsnark appends ([`qap`]).
//! - Per-circuit trusted setup: the toxic waste `t, alpha_A, alpha_B, alpha_C, rho_A, rho_B, beta,
//!   gamma` is sampled and dropped; the proving key's A/B/C/H/K queries and the verification key
//!   (`alpha_A, alpha_B, alpha_C, gamma, beta*gamma, rho_C*Z(t)` and the IC vector) match libsnark
//!   ([`keys`]).
//! - The proof `pi = (pi_A, pi'_A, pi_B, pi'_B, pi_C, pi'_C, pi_K, pi_H)`, `pi_B` in G2 and the rest
//!   in G1, with zero-knowledge blinding `d1, d2, d3` ([`prover`], [`proof`]).
//! - The three knowledge-commitment checks, the same-coefficient check and the QAP divisibility
//!   check, each a named function in [`verifier`] whose doc comment states its pairing equation.
//!
//! # Serialization and proof size
//!
//! [`proof::Proof::to_bytes`] writes the eight elements in the fixed order `A, A', B, B', C, C',
//! K, H` using arkworks compressed encoding: 32 bytes per G1 element and 64 per G2, so a proof is
//! `7 * 32 + 64 = 288` bytes. Zcash's Sprout proof encoding is **296 bytes** for the same eight
//! elements: libff/libsnark prepend a separate one-byte tag to every compressed point (33 bytes per
//! G1, 65 per G2), which is exactly 8 bytes more across the eight elements. The parity bit that tag
//! carries is folded into the high bits of the field encoding in arkworks, so no extra byte is
//! needed here.
//!
//! # The counterfeiting flaw
//!
//! [`cve_2019_7167`] reproduces the soundness break of Gabizon (ePrint 2019/119, CVE-2019-7167):
//! a key generator that also publishes the redundant `pi'_A` input elements the paper's construction
//! exposed, and a forger that uses them to turn a proof of a true statement into a proof of a false
//! one that the ordinary verifier accepts.

use zk_core::catalog::{
    Assumption, Deployment, Implementation, Mode, Paper, ProofSize, Recursion, Shelf, Status,
    SystemMeta, TrustedSetup, ZeroKnowledge,
};
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub mod adapter;
pub mod cve_2019_7167;
pub mod field;
pub mod keys;
pub mod proof;
pub mod prover;
pub mod qap;
pub mod verifier;

pub use field::Bn254Fr;

/// The circuit field under the name every exhibit exports it by, so an activity can compute values
/// for [`zk_core::Prepared::prove_assignment`].
pub type Field = Bn254Fr;
pub use proof::Proof;

/// The BCTV14 proof system exhibit.
pub struct Bctv14;

/// Everything the museum records about this exhibit, checked against the listed sources on 2026-09-16.
///
/// Marked a teaching implementation (built on the arkworks primitives) and superseded by Groth16
/// when Zcash's Sapling upgrade activated on 2018-10-28.
pub static META: SystemMeta = SystemMeta {
    id: "bctv14",
    name: "BCTV14 (Pinocchio)",
    shelf: Shelf::Zcash,
    year: 2013,
    authors: &["Eli Ben-Sasson", "Alessandro Chiesa", "Eran Tromer", "Madars Virza"],
    paper: Some(Paper {
        title: "Succinct Non-Interactive Zero Knowledge for a von Neumann Architecture",
        venue: "USENIX Security 2014",
        url: "https://eprint.iacr.org/2013/879",
    }),
    trusted_setup: TrustedSetup::PerCircuit,
    zero_knowledge: ZeroKnowledge::Yes,
    assumptions: &[Assumption::Pairing],
    proof_size: ProofSize::Constant,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "BN254",
    implementation: Implementation::Teaching {
        built_on: &["ark-bn254", "ark-ec", "ark-ff", "ark-poly"],
    },
    status: Status::Superseded { by: "Groth16", since: "2018-10-28" },
    status_as_of: "2026-09",
    status_sources: &["https://zips.z.cash/protocol/protocol.pdf"],
    deployments: &[Deployment {
        project: "Zcash Sprout",
        since: "2016-10-28",
        until: Some("2018-10-28"),
    }],
};

impl ProofSystem for Bctv14 {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        Ok(Box::new(adapter::prepare(example, control)?))
    }
}
