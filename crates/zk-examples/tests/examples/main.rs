//! The seven examples on real fields.

// A failing test should stop with the error it hit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "../../../zk-circuit/tests/circuit/fields.rs"]
mod fields;
mod instances;
mod pool;
mod statements;
