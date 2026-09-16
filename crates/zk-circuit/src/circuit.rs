//! The circuit IR: a list of gates and nothing else.

use crate::lc::{LinearCombination, Wire};

/// Whether the verifier sees an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Known to the verifier; part of the statement.
    Public,
    /// Known only to the prover; part of the witness.
    Private,
}

/// A non-deterministic helper computation.
///
/// Some values are cheap to check but expensive to compute with gates (the bits of a number, an
/// inverse). The prover computes them outside the circuit; their output wires are unconstrained
/// until a gadget adds the gates that pin them down, so a hint alone proves nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hint {
    /// The low `n` bits of the input's canonical representative, least significant first.
    Bits(u32),
    /// The inverse of the input, or zero when the input is zero.
    Inverse,
}

/// One step of a circuit.
///
/// Five kinds of arithmetic step plus hints are enough for every statement in the museum, and each
/// maps directly onto R1CS rows, PLONKish rows and AIR constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gate<F> {
    /// A wire fixed to a value; lowerings fold it into constant terms.
    Constant {
        /// The defined wire.
        output: Wire,
        /// Its value.
        value: F,
    },
    /// A value supplied by the prover, public or private, under a stable name.
    Input {
        /// The defined wire.
        output: Wire,
        /// Stable name backends and screens use to refer to the input.
        name: String,
        /// Whether the verifier sees it.
        visibility: Visibility,
    },
    /// `output = lc`.
    Linear {
        /// The defined wire.
        output: Wire,
        /// The combination it equals.
        lc: LinearCombination<F>,
    },
    /// `output = left · right`, the only source of degree.
    Mul {
        /// The defined wire.
        output: Wire,
        /// Left factor.
        left: LinearCombination<F>,
        /// Right factor.
        right: LinearCombination<F>,
    },
    /// `lc = 0`, the only gate that can make a witness invalid.
    AssertZero {
        /// The combination that must vanish.
        lc: LinearCombination<F>,
        /// Stable sentence saying what the assertion means, e.g. `"row 2 sums to 10"`.
        label: String,
    },
    /// Outputs computed outside the circuit from `input`; unconstrained by themselves.
    Hint {
        /// The defined wires, in the order the hint produces them.
        outputs: Vec<Wire>,
        /// What the hint computes.
        kind: Hint,
        /// The value it computes from.
        input: LinearCombination<F>,
    },
}

impl<F> Gate<F> {
    /// Wires this gate defines, in order; empty for assert-zero.
    pub fn outputs(&self) -> &[Wire] {
        match self {
            Self::Constant { output, .. }
            | Self::Input { output, .. }
            | Self::Linear { output, .. }
            | Self::Mul { output, .. } => core::slice::from_ref(output),
            Self::AssertZero { .. } => &[],
            Self::Hint { outputs, .. } => outputs,
        }
    }

    /// Linear combinations this gate reads, for validation.
    pub(crate) fn operands(&self) -> Vec<&LinearCombination<F>> {
        match self {
            Self::Constant { .. } | Self::Input { .. } => Vec::new(),
            Self::Linear { lc, .. } | Self::AssertZero { lc, .. } => vec![lc],
            Self::Mul { left, right, .. } => vec![left, right],
            Self::Hint { input, .. } => vec![input],
        }
    }
}

/// A named input, as listed by [`Circuit::public_inputs`] and [`Circuit::private_inputs`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// Stable name.
    pub name: String,
    /// The wire holding its value.
    pub wire: Wire,
}

/// A statement as a pure description: gates and input order, no values.
///
/// Built only through [`crate::CircuitBuilder`], which guarantees that every gate reads wires
/// defined before it and that input names are unique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Circuit<F> {
    pub(crate) gates: Vec<Gate<F>>,
    pub(crate) num_wires: usize,
    pub(crate) public_inputs: Vec<Input>,
    pub(crate) private_inputs: Vec<Input>,
}

impl<F> Circuit<F> {
    /// Gates in evaluation order.
    pub fn gates(&self) -> &[Gate<F>] {
        &self.gates
    }

    /// Number of wires, which is also the length of [`crate::WireValues`].
    pub fn num_wires(&self) -> usize {
        self.num_wires
    }

    /// Public inputs in declaration order, the order a verifier receives them in.
    pub fn public_inputs(&self) -> &[Input] {
        &self.public_inputs
    }

    /// Private inputs in declaration order.
    pub fn private_inputs(&self) -> &[Input] {
        &self.private_inputs
    }

    /// Labels of every assert-zero gate, in gate order.
    pub fn assertion_labels(&self) -> Vec<&str> {
        self.gates
            .iter()
            .filter_map(|gate| match gate {
                Gate::AssertZero { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect()
    }
}
