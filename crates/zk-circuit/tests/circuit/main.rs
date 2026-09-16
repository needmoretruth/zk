//! zk-circuit on real fields.

// A failing test should stop with the error it hit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod builder;
mod eval;
mod field;
mod fields;
mod gadgets;
mod lowering;
mod toyhash;
