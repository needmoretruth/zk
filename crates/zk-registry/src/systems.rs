//! The list itself.
//!
//! To wire a system in: add its crate to this crate's `Cargo.toml`, then add `&its_crate::System`
//! below in shelf order. The tests in `lib.rs` check the order, the IDs and that a page exists.

use zk_core::ProofSystem;

/// Every system built into this binary, in shelf order. Empty until systems are wired in.
pub(crate) static SYSTEMS: &[&dyn ProofSystem] = &[];
