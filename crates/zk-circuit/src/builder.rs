//! Appending gates to a circuit.

use std::collections::HashSet;

use crate::circuit::{Circuit, Gate, Hint, Input, Visibility};
use crate::error::CircuitError;
use crate::field::{ZkField, check_field};
use crate::lc::{LinearCombination, Wire};

/// Records gates in order and hands out wires.
///
/// Gadgets and examples take `&mut CircuitBuilder` so a statement is written as ordinary Rust
/// calls; [`Self::finish`] then checks the result once instead of every call returning a `Result`.
#[derive(Debug)]
pub struct CircuitBuilder<F> {
    gates: Vec<Gate<F>>,
    num_wires: usize,
    public_inputs: Vec<Input>,
    private_inputs: Vec<Input>,
}

impl<F: ZkField> CircuitBuilder<F> {
    /// Starts a circuit, refusing fields whose modulus does not exceed 2^30.
    pub fn new() -> Result<Self, CircuitError> {
        check_field::<F>()?;
        Ok(Self {
            gates: Vec::new(),
            num_wires: 0,
            public_inputs: Vec::new(),
            private_inputs: Vec::new(),
        })
    }

    fn fresh_wire(&mut self) -> Wire {
        let wire = Wire::new(self.num_wires);
        self.num_wires += 1;
        wire
    }

    /// A wire fixed to `value`.
    pub fn constant(&mut self, value: F) -> Wire {
        let output = self.fresh_wire();
        self.gates.push(Gate::Constant { output, value });
        output
    }

    /// Declares the next public input; public inputs are ordered by these calls.
    pub fn public_input(&mut self, name: impl Into<String>) -> Wire {
        self.input(name.into(), Visibility::Public)
    }

    /// Declares the next private input; private inputs are ordered by these calls.
    pub fn private_input(&mut self, name: impl Into<String>) -> Wire {
        self.input(name.into(), Visibility::Private)
    }

    fn input(&mut self, name: String, visibility: Visibility) -> Wire {
        let output = self.fresh_wire();
        let entry = Input { name: name.clone(), wire: output };
        match visibility {
            Visibility::Public => self.public_inputs.push(entry),
            Visibility::Private => self.private_inputs.push(entry),
        }
        self.gates.push(Gate::Input { output, name, visibility });
        output
    }

    /// A wire equal to `lc`, so a sum used several times is lowered once.
    pub fn linear(&mut self, lc: impl Into<LinearCombination<F>>) -> Wire {
        let lc = lc.into();
        let output = self.fresh_wire();
        self.gates.push(Gate::Linear { output, lc });
        output
    }

    /// A wire equal to `left · right`.
    pub fn mul(
        &mut self,
        left: impl Into<LinearCombination<F>>,
        right: impl Into<LinearCombination<F>>,
    ) -> Wire {
        let (left, right) = (left.into(), right.into());
        let output = self.fresh_wire();
        self.gates.push(Gate::Mul { output, left, right });
        output
    }

    /// Requires `lc = 0`; `label` names the statement that is false when it does not hold.
    pub fn assert_zero(&mut self, lc: impl Into<LinearCombination<F>>, label: impl Into<String>) {
        self.gates.push(Gate::AssertZero { lc: lc.into(), label: label.into() });
    }

    /// `count` unconstrained wires meant to hold the low bits of `input`, least significant first.
    pub fn hint_bits(&mut self, input: impl Into<LinearCombination<F>>, count: u32) -> Vec<Wire> {
        let outputs: Vec<Wire> = (0..count).map(|_| self.fresh_wire()).collect();
        self.gates.push(Gate::Hint {
            outputs: outputs.clone(),
            kind: Hint::Bits(count),
            input: input.into(),
        });
        outputs
    }

    /// One unconstrained wire meant to hold the inverse of `input`.
    pub fn hint_inverse(&mut self, input: impl Into<LinearCombination<F>>) -> Wire {
        let output = self.fresh_wire();
        self.gates.push(Gate::Hint {
            outputs: vec![output],
            kind: Hint::Inverse,
            input: input.into(),
        });
        output
    }

    /// Validates wire order and input names and returns the finished description.
    pub fn finish(self) -> Result<Circuit<F>, CircuitError> {
        let mut names = HashSet::new();
        for input in self.public_inputs.iter().chain(&self.private_inputs) {
            if !names.insert(input.name.as_str()) {
                return Err(CircuitError::DuplicateInputName { name: input.name.clone() });
            }
        }
        let mut defined = 0;
        for (index, gate) in self.gates.iter().enumerate() {
            for lc in gate.operands() {
                if let Some((wire, _)) = lc.terms().iter().find(|(wire, _)| wire.index() >= defined)
                {
                    return Err(CircuitError::UndefinedWire { gate: index, wire: wire.index() });
                }
            }
            for output in gate.outputs() {
                if output.index() != defined {
                    return Err(CircuitError::UndefinedWire { gate: index, wire: output.index() });
                }
                defined += 1;
            }
        }
        Ok(Circuit {
            gates: self.gates,
            num_wires: self.num_wires,
            public_inputs: self.public_inputs,
            private_inputs: self.private_inputs,
        })
    }
}
