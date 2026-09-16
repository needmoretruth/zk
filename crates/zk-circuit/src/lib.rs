//! Statements written once, proved by every system.
//!
//! The museum compares about thirty proof systems on the same seven statements. A comparison is
//! only fair if every system proves the very same circuit, so each statement is written once over
//! any [`ZkField`] with a [`CircuitBuilder`], and then lowered into the shape a proof system
//! consumes: [`lower::r1cs::R1cs`], [`lower::plonkish::Plonkish`] or [`lower::wide_air::WideAir`].
//! The circuit layer also computes the witness ([`Circuit::evaluate`]), so adapters only prove.
//!
//! Fields must be prime with a modulus above 2^30; [`CircuitBuilder::new`] refuses smaller ones.

mod builder;
mod circuit;
mod error;
mod eval;
mod field;
pub mod gadgets;
mod lc;
pub mod lower;
pub mod toyhash;

pub use builder::CircuitBuilder;
pub use circuit::{Circuit, Gate, Hint, Input, Visibility};
pub use error::{CircuitError, EvalError};
pub use eval::{Assignment, UncheckedEvaluation, WireValues};
pub use field::{MIN_MODULUS_BITS, ZkField, check_field, to_u64};
pub use lc::{LinearCombination, Wire};
