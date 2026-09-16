//! The contract between the museum and every proof system in it.
//!
//! A system declares what it is ([`catalog::SystemMeta`]) and how to prove one example
//! ([`ProofSystem`], [`Prepared`]). The shared [`harness`] then runs every system the same way:
//! setup, an honest proof, verification, and the same three attacks — flip a proof byte, change a
//! public input, prove a false claim — and it searches the proof for the secrets it was meant to
//! hide. The result is a [`RunReport`] a screen can draw or a companion process can send as JSON.

pub mod catalog;
mod error;
pub mod harness;
mod report;
mod scan;
mod system;

pub use error::{RunError, SystemError};
pub use report::{
    AttackKind, AttackOutcome, AttackReport, PROOF_HEAD_BYTES, RunMode, RunReport, SecretScan,
    Timings,
};
pub use scan::scan_secrets;
pub use system::{
    CircuitShape, Control, FieldBytes, Instance, Interaction, Prepared, ProofSystem, Proven,
    RunOptions, ShapeForm, Stage, Support, Verdict,
};
pub use zk_examples::{ExampleId, InstanceKind};
