//! Lowering one circuit into the three shapes proof systems consume.
//!
//! Each lowering comes with a checker that replays what a verifier relies on: given the public
//! inputs and the prover's lowered assignment, does every constraint hold. Tests use the checkers
//! to show a lowering accepts exactly the witnesses the circuit accepts; backends can use them to
//! tell a lowering bug from a prover bug.

mod constants;
pub mod plonkish;
mod plonkish_build;
pub mod r1cs;
pub mod wide_air;

use core::fmt;

/// The first reason a lowered assignment fails its checker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Violation {
    /// The verifier's public input list has the wrong length.
    PublicInputCount {
        /// Public inputs the circuit declares.
        expected: usize,
        /// Values supplied.
        got: usize,
    },
    /// The assignment (R1CS vector, PLONKish table or AIR row) has the wrong length.
    AssignmentLength {
        /// Length the lowering requires.
        expected: usize,
        /// Length supplied.
        got: usize,
    },
    /// R1CS variable 0 is not the constant one.
    ConstantOne,
    /// The assignment disagrees with public input `index` (R1CS slot or AIR boundary).
    PublicInput {
        /// Position in declaration order.
        index: usize,
    },
    /// Constraint or row `index` does not evaluate to zero.
    Constraint {
        /// Position in the lowering's constraint list.
        index: usize,
    },
    /// Copy class `index` holds cells with different values.
    CopyClass {
        /// Position in [`plonkish::Plonkish::copy_classes`].
        index: usize,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublicInputCount { expected, got } => {
                write!(f, "expected {expected} public inputs, got {got}")
            }
            Self::AssignmentLength { expected, got } => {
                write!(f, "expected an assignment of length {expected}, got {got}")
            }
            Self::ConstantOne => write!(f, "variable 0 is not one"),
            Self::PublicInput { index } => write!(f, "public input {index} does not match"),
            Self::Constraint { index } => write!(f, "constraint {index} does not hold"),
            Self::CopyClass { index } => write!(f, "copy class {index} holds different values"),
        }
    }
}

impl std::error::Error for Violation {}

fn check_public_count(expected: usize, got: usize) -> Result<(), Violation> {
    if expected == got { Ok(()) } else { Err(Violation::PublicInputCount { expected, got }) }
}

fn check_length(expected: usize, got: usize) -> Result<(), Violation> {
    if expected == got { Ok(()) } else { Err(Violation::AssignmentLength { expected, got }) }
}
