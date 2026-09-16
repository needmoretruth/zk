//! Errors from building and evaluating circuits.

use core::fmt;

use crate::circuit::Visibility;

/// Why a circuit could not be built.
///
/// Building fails instead of panicking so a program listing every system and every example can
/// report one unusable combination and carry on with the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CircuitError {
    /// The field's modulus does not exceed 2^30, so example integers could wrap around.
    FieldTooSmall {
        /// The field's human name.
        field: &'static str,
        /// The bit length the field declares.
        modulus_bits: u32,
    },
    /// A range check was asked for more bits than the field can hold without wrapping.
    RangeTooWide {
        /// Requested width.
        bits: u32,
        /// Bit length of the field's modulus.
        modulus_bits: u32,
    },
    /// Two inputs share a name, so a backend could not tell them apart.
    DuplicateInputName {
        /// The repeated name.
        name: String,
    },
    /// A gate reads a wire that is not defined before it, such as a wire from another builder.
    UndefinedWire {
        /// Index of the offending gate.
        gate: usize,
        /// Index of the wire it reads.
        wire: usize,
    },
    /// A Merkle path has a different number of direction bits and siblings.
    PathLengthMismatch {
        /// Number of direction bits.
        bits: usize,
        /// Number of siblings.
        siblings: usize,
    },
}

impl fmt::Display for CircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldTooSmall { field, modulus_bits } => {
                write!(f, "field {field} has a {modulus_bits}-bit modulus; more than 2^30 needed")
            }
            Self::RangeTooWide { bits, modulus_bits } => {
                write!(f, "a {bits}-bit range check wraps in a {modulus_bits}-bit field")
            }
            Self::DuplicateInputName { name } => write!(f, "input name {name:?} is used twice"),
            Self::UndefinedWire { gate, wire } => {
                write!(f, "gate {gate} reads wire {wire} before it is defined")
            }
            Self::PathLengthMismatch { bits, siblings } => {
                write!(f, "Merkle path has {bits} direction bits but {siblings} siblings")
            }
        }
    }
}

impl std::error::Error for CircuitError {}

/// Why a circuit could not be evaluated on an assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// The assignment has the wrong number of public or private values.
    InputCount {
        /// Which list has the wrong length.
        visibility: Visibility,
        /// Inputs the circuit declares.
        expected: usize,
        /// Values the assignment supplies.
        got: usize,
    },
    /// An assert-zero gate does not hold; the label says which statement is false.
    AssertionFailed {
        /// Label of the first violated assertion, in gate order.
        label: String,
    },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputCount { visibility, expected, got } => {
                write!(f, "expected {expected} {visibility:?} inputs, got {got}")
            }
            Self::AssertionFailed { label } => write!(f, "assertion failed: {label}"),
        }
    }
}

impl std::error::Error for EvalError {}
