//! Rank-1 constraint systems: `⟨A_k, z⟩ · ⟨B_k, z⟩ = ⟨C_k, z⟩` for every row `k`.
//!
//! The shape of Groth16, BCTV14, GM17, Marlin, Spartan, Nova and the Bulletproofs arithmetic
//! circuits: one row per multiplication, additions free inside the rows.

use std::collections::BTreeMap;

use crate::circuit::{Circuit, Gate};
use crate::eval::WireValues;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};
use crate::lower::constants::ConstantWires;
use crate::lower::{Violation, check_length, check_public_count};

/// A sparse linear combination of variables: `(variable index, coefficient)`, sorted by index,
/// without repeats or zero coefficients.
pub type SparseLc<F> = Vec<(usize, F)>;

/// One row `⟨a, z⟩ · ⟨b, z⟩ = ⟨c, z⟩`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct R1csConstraint<F> {
    /// Left factor.
    pub a: SparseLc<F>,
    /// Right factor.
    pub b: SparseLc<F>,
    /// Product.
    pub c: SparseLc<F>,
}

/// A circuit lowered to R1CS.
///
/// Variable layout of the assignment vector `z`:
/// - `z[0] = 1`;
/// - `z[1..=n]` are the `n` public inputs in declaration order;
/// - then the private inputs in declaration order;
/// - then one variable per multiplication output and per hint output, in gate order.
///
/// Linear and constant gates get no variable: they are inlined into the rows that read them.
/// Each multiplication becomes `left · right = output`; each assert-zero becomes `lc · 1 = 0`
/// (with `c` empty). Hint outputs are variables that only the gadgets' rows constrain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct R1cs<F> {
    num_public: usize,
    variable_wires: Vec<Wire>,
    constraints: Vec<R1csConstraint<F>>,
}

impl<F: ZkField> R1cs<F> {
    /// Lowers `circuit`.
    pub fn from_circuit(circuit: &Circuit<F>) -> Self {
        let constants = ConstantWires::of(circuit);
        let mut resolved: Vec<SparseLc<F>> = vec![Vec::new(); circuit.num_wires()];
        let mut variable_wires = Vec::new();
        let mut add_variable = |wire: Wire, resolved: &mut Vec<SparseLc<F>>| {
            variable_wires.push(wire);
            resolved[wire.index()] = vec![(variable_wires.len(), F::one())];
        };
        for input in circuit.public_inputs().iter().chain(circuit.private_inputs()) {
            add_variable(input.wire, &mut resolved);
        }
        let mut constraints = Vec::new();
        for gate in circuit.gates() {
            match gate {
                Gate::Constant { .. } | Gate::Input { .. } => {}
                Gate::Linear { output, lc } => {
                    resolved[output.index()] = inline(&constants.fold(lc), &resolved);
                }
                Gate::Mul { output, left, right } => {
                    let a = inline(&constants.fold(left), &resolved);
                    let b = inline(&constants.fold(right), &resolved);
                    add_variable(*output, &mut resolved);
                    let c = resolved[output.index()].clone();
                    constraints.push(R1csConstraint { a, b, c });
                }
                Gate::AssertZero { lc, .. } => {
                    let a = inline(&constants.fold(lc), &resolved);
                    constraints.push(R1csConstraint { a, b: vec![(0, F::one())], c: Vec::new() });
                }
                Gate::Hint { outputs, .. } => {
                    for output in outputs {
                        add_variable(*output, &mut resolved);
                    }
                }
            }
        }
        Self { num_public: circuit.public_inputs().len(), variable_wires, constraints }
    }

    /// The rows.
    pub fn constraints(&self) -> &[R1csConstraint<F>] {
        &self.constraints
    }

    /// Number of rows.
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// Length of `z`, including the constant one.
    pub fn num_variables(&self) -> usize {
        self.variable_wires.len() + 1
    }

    /// Number of public inputs, which occupy `z[1..=n]`.
    pub fn num_public_inputs(&self) -> usize {
        self.num_public
    }

    /// Number of non-zero coefficients across `A`, `B` and `C`, the usual measure of prover work.
    pub fn num_nonzero_entries(&self) -> usize {
        self.constraints.iter().map(|row| row.a.len() + row.b.len() + row.c.len()).sum()
    }

    /// The full assignment vector `z` for evaluated wires.
    pub fn assignment(&self, wires: &WireValues<F>) -> Vec<F> {
        core::iter::once(F::one())
            .chain(self.variable_wires.iter().map(|wire| wires.get(*wire)))
            .collect()
    }

    /// Checks `z[0] = 1`, `z[1..=n] = public`, and every row.
    pub fn check(&self, public: &[F], z: &[F]) -> Result<(), Violation> {
        check_public_count(self.num_public, public.len())?;
        check_length(self.num_variables(), z.len())?;
        if z[0] != F::one() {
            return Err(Violation::ConstantOne);
        }
        if let Some(index) = (0..self.num_public).find(|i| z[i + 1] != public[*i]) {
            return Err(Violation::PublicInput { index });
        }
        let dot = |lc: &SparseLc<F>| {
            lc.iter().fold(F::zero(), |acc, (variable, coefficient)| {
                acc.add(z[*variable].mul(*coefficient))
            })
        };
        match self.constraints.iter().position(|row| dot(&row.a).mul(dot(&row.b)) != dot(&row.c)) {
            Some(index) => Err(Violation::Constraint { index }),
            None => Ok(()),
        }
    }
}

/// Rewrites a folded combination over wires as a sparse combination over variables.
fn inline<F: ZkField>(lc: &LinearCombination<F>, resolved: &[SparseLc<F>]) -> SparseLc<F> {
    let mut sum: BTreeMap<usize, F> = BTreeMap::new();
    if lc.constant_term() != F::zero() {
        sum.insert(0, lc.constant_term());
    }
    for (wire, coefficient) in lc.terms() {
        for (variable, inner) in &resolved[wire.index()] {
            let entry = sum.entry(*variable).or_insert_with(F::zero);
            *entry = entry.add(inner.mul(*coefficient));
        }
    }
    sum.into_iter().filter(|(_, coefficient)| *coefficient != F::zero()).collect()
}
