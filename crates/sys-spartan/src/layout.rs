//! The museum's R1CS rearranged into the matrices and assignments Spartan's API takes.
//!
//! The museum orders the assignment `z = [1, public inputs, witness]`; Spartan orders it
//! `z = [witness, 1, public inputs, zeros]`, with the witness part padded to a power of two `N`
//! at least one longer than the inputs, so every column moves. Public inputs go into Spartan's input
//! slot rather than the witness, because only that slot enters the verifier's final check.

use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::{ELEMENT_BYTES, RistrettoScalar};

/// One matrix entry as `Instance::new` takes it: row, column, canonical little-endian value.
pub(crate) type Entry = (usize, usize, [u8; ELEMENT_BYTES]);

/// The three matrices in Spartan's column order, with the sizes before and after padding.
pub(crate) struct Layout {
    /// Constraints the museum's circuit has.
    pub(crate) num_cons: usize,
    /// Public inputs, which Spartan calls inputs.
    pub(crate) num_inputs: usize,
    /// Rows Spartan proves: `num_cons` rounded up to a power of two, at least 2; extra rows are zero.
    pub(crate) padded_cons: usize,
    /// Witness slots Spartan commits to: the witness length rounded up to a power of two, at least
    /// `num_inputs + 1`; extra slots hold zero.
    pub(crate) padded_vars: usize,
    /// Left factors.
    pub(crate) a: Vec<Entry>,
    /// Right factors.
    pub(crate) b: Vec<Entry>,
    /// Products.
    pub(crate) c: Vec<Entry>,
}

impl Layout {
    /// Re-indexes every row of `r1cs` and applies the padding `Instance::new` would apply itself, so
    /// the sizes reported are the sizes proved.
    pub(crate) fn of(r1cs: &R1cs<RistrettoScalar>) -> Self {
        let num_cons = r1cs.num_constraints();
        let num_inputs = r1cs.num_public_inputs();
        let num_witness = r1cs.num_variables() - 1 - num_inputs;
        let mut layout = Self {
            num_cons,
            num_inputs,
            padded_cons: num_cons.max(2).next_power_of_two(),
            padded_vars: num_witness.max(num_inputs + 1).next_power_of_two(),
            a: Vec::new(),
            b: Vec::new(),
            c: Vec::new(),
        };
        for (row, constraint) in r1cs.constraints().iter().enumerate() {
            let a = layout.entries(row, &constraint.a);
            let b = layout.entries(row, &constraint.b);
            let c = layout.entries(row, &constraint.c);
            layout.a.extend(a);
            layout.b.extend(b);
            layout.c.extend(c);
        }
        layout
    }

    /// The most non-zero entries in any one matrix, which sizes `SNARKGens`.
    pub(crate) fn max_nonzero(&self) -> usize {
        self.a.len().max(self.b.len()).max(self.c.len())
    }

    /// Splits the museum's `z` into Spartan's padded witness and its inputs, as canonical bytes.
    pub(crate) fn split(
        &self,
        z: &[RistrettoScalar],
    ) -> (Vec<[u8; ELEMENT_BYTES]>, Vec<[u8; ELEMENT_BYTES]>) {
        let bytes = |value: &RistrettoScalar| value.0.to_bytes();
        let inputs = z.iter().skip(1).take(self.num_inputs).map(bytes).collect();
        let mut vars: Vec<_> = z.iter().skip(1 + self.num_inputs).map(bytes).collect();
        vars.resize(self.padded_vars, [0; ELEMENT_BYTES]);
        (vars, inputs)
    }

    fn entries(&self, row: usize, lc: &SparseLc<RistrettoScalar>) -> Vec<Entry> {
        lc.iter()
            .map(|(variable, value)| (row, self.column(*variable), value.0.to_bytes()))
            .collect()
    }

    /// Where the museum's variable `index` sits in Spartan's `z`.
    fn column(&self, index: usize) -> usize {
        match index {
            0 => self.padded_vars,
            input if input <= self.num_inputs => self.padded_vars + input,
            witness => witness - 1 - self.num_inputs,
        }
    }
}
