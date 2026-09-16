//! The museum's circuit rebuilt gate by gate with jellyfish's own constraint API.
//!
//! jellyfish's `PlonkCircuit` holds both the gates and the witness, so the same walk builds the
//! circuit for preprocessing (over any assignment; only the shape matters there) and for proving.
//! Every gate of the museum's IR becomes jellyfish calls:
//! - an input becomes `create_public_variable` or `create_variable`, in declaration order, so
//!   jellyfish's public inputs come out in the museum's order;
//! - a constant wire is folded into the constant term of whatever reads it;
//! - a linear gate or assertion becomes one TurboPlonk `quad_poly_gate`, which reads four wires
//!   (`q_1·a + q_2·b + q_3·c + q_4·d + q_C = q_O·e`); longer sums are first folded four terms at a
//!   time with jellyfish's `lc`, and an assertion sets `q_O = 0`;
//! - a product becomes one `quad_poly_gate` using its two multiplication selectors, so
//!   `(α₁x₁ + α₂x₂ + β)(γy + δ)` fits in a single gate; wider factors are first folded with `lc`;
//! - a hint's outputs become plain variables holding the evaluated values;
//! - a range check found by [`RangePlan`] becomes `enforce_in_range`, UltraPlonk's table lookup.

use ark_ff::{One, Zero};
use jf_relation::constants::GATE_WIDTH;
use jf_relation::{Circuit as _, CircuitError, PlonkCircuit, Variable};
use zk_circuit::{Circuit, Gate, LinearCombination, Visibility, WireValues};

use crate::field::Fr;
use crate::ranges::{RANGE_BIT_LEN, RangePlan};

type Scalar = ark_bn254::Fr;

/// A jellyfish circuit ready for arithmetization, with the size it had before padding.
pub(crate) struct Built {
    /// The finalized circuit: padded to its domain, public-input gates moved to the front.
    pub(crate) circuit: PlonkCircuit<Scalar>,
    /// Gates jellyfish holds before `finalize_for_arithmetization` pads them to the domain size.
    pub(crate) gates: usize,
}

/// Rebuilds `circuit` with wire values `values` and finalizes it for jellyfish's prover.
pub(crate) fn build(
    circuit: &Circuit<Fr>,
    plan: &RangePlan,
    values: &WireValues<Fr>,
) -> Result<Built, CircuitError> {
    let mut walk = Walk {
        cs: PlonkCircuit::new_ultra_plonk(RANGE_BIT_LEN),
        values,
        variables: vec![None; circuit.num_wires()],
        constants: vec![None; circuit.num_wires()],
    };
    for (index, gate) in circuit.gates().iter().enumerate() {
        if !plan.is_replaced(index) {
            walk.gate(gate, plan.width_at(index))?;
        }
    }
    let gates = walk.cs.num_gates();
    walk.cs.finalize_for_arithmetization()?;
    Ok(Built { circuit: walk.cs, gates })
}

/// `Σ coefficient·variable + constant`, a museum combination in jellyfish's variables.
struct Sum {
    terms: Vec<(Variable, Scalar)>,
    constant: Scalar,
}

struct Walk<'a> {
    cs: PlonkCircuit<Scalar>,
    values: &'a WireValues<Fr>,
    /// The jellyfish variable of every museum wire that has one.
    variables: Vec<Option<Variable>>,
    /// The value of every constant wire.
    constants: Vec<Option<Scalar>>,
}

impl Walk<'_> {
    /// Emits one museum gate; `range` is the width of a replaced range check whose hint this is.
    fn gate(&mut self, gate: &Gate<Fr>, range: Option<u32>) -> Result<(), CircuitError> {
        match gate {
            Gate::Constant { output, value } => self.constants[output.index()] = Some(value.0),
            Gate::Input { output, visibility, .. } => {
                let value = self.values.get(*output).0;
                let variable = match visibility {
                    Visibility::Public => self.cs.create_public_variable(value)?,
                    Visibility::Private => self.cs.create_variable(value)?,
                };
                self.variables[output.index()] = Some(variable);
            }
            Gate::Linear { output, lc } => {
                let sum = self.fold(lc)?;
                let variable = self.define(*output)?;
                self.constrain_sum(sum, Some(variable))?;
            }
            Gate::Mul { output, left, right } => {
                let (left, right) = (self.fold(left)?, self.fold(right)?);
                let variable = self.define(*output)?;
                self.constrain_product(left, right, variable)?;
            }
            Gate::AssertZero { lc, .. } => {
                let sum = self.fold(lc)?;
                self.constrain_sum(sum, None)?;
            }
            Gate::Hint { outputs, input, .. } => match range {
                Some(width) => {
                    let sum = self.fold(input)?;
                    let value = self.materialize(sum)?;
                    self.cs.enforce_in_range(value, width as usize)?;
                }
                None => {
                    for output in outputs {
                        self.define(*output)?;
                    }
                }
            },
        }
        Ok(())
    }

    /// A fresh variable holding the evaluated value of museum wire `wire`.
    fn define(&mut self, wire: zk_circuit::Wire) -> Result<Variable, CircuitError> {
        let variable = self.cs.create_variable(self.values.get(wire).0)?;
        self.variables[wire.index()] = Some(variable);
        Ok(variable)
    }

    /// The combination over jellyfish variables, constant wires moved into the constant term.
    fn fold(&self, lc: &LinearCombination<Fr>) -> Result<Sum, CircuitError> {
        let lc = lc.normalized();
        let mut sum =
            Sum { terms: Vec::with_capacity(lc.terms().len()), constant: lc.constant_term().0 };
        for (wire, coefficient) in lc.terms() {
            match (self.constants[wire.index()], self.variables[wire.index()]) {
                (Some(value), _) => sum.constant += value * coefficient.0,
                (None, Some(variable)) => sum.terms.push((variable, coefficient.0)),
                (None, None) => {
                    let wire = wire.index();
                    return Err(CircuitError::ParameterError(format!(
                        "museum wire {wire} is read before a jellyfish variable holds it"
                    )));
                }
            }
        }
        Ok(sum)
    }

    /// Constrains `Σ terms + constant = output`, or `= 0` without an output.
    ///
    /// Sums wider than one gate are folded four terms at a time into jellyfish `lc` variables.
    fn constrain_sum(
        &mut self,
        mut sum: Sum,
        output: Option<Variable>,
    ) -> Result<(), CircuitError> {
        let zero = self.cs.zero();
        while sum.terms.len() > GATE_WIDTH {
            let rest = sum.terms.split_off(GATE_WIDTH);
            let (wires, coefficients) = self.row(&sum.terms);
            let partial = self.cs.lc(&wires, &coefficients)?;
            sum.terms = core::iter::once((partial, Scalar::one())).chain(rest).collect();
        }
        let ([a, b, c, d], q_lc) = self.row(&sum.terms);
        let (e, q_o) = output.map_or((zero, Scalar::zero()), |variable| (variable, Scalar::one()));
        self.cs.quad_poly_gate(&[a, b, c, d, e], &q_lc, &[Scalar::zero(); 2], q_o, sum.constant)
    }

    /// Up to four terms as a gate's input wires and linear selectors, padded with the zero variable.
    fn row(&self, terms: &[(Variable, Scalar)]) -> ([Variable; GATE_WIDTH], [Scalar; GATE_WIDTH]) {
        let mut wires = [self.cs.zero(); GATE_WIDTH];
        let mut coefficients = [Scalar::zero(); GATE_WIDTH];
        for (slot, (variable, coefficient)) in terms.iter().enumerate().take(GATE_WIDTH) {
            wires[slot] = *variable;
            coefficients[slot] = *coefficient;
        }
        (wires, coefficients)
    }

    /// Constrains `left · right = output` in one gate once each factor is narrow enough.
    ///
    /// With `left = α₁x₁ + α₂x₂ + β` and `right = γy + δ` the product is
    /// `α₁γ·x₁y + α₂γ·x₂y + α₁δ·x₁ + α₂δ·x₂ + βγ·y + βδ`: wires `[x₁, y, x₂, y]`, multiplication
    /// selectors `[α₁γ, α₂γ]`, linear selectors `[α₁δ, βγ, α₂δ, 0]`, constant `βδ`.
    fn constrain_product(
        &mut self,
        left: Sum,
        right: Sum,
        output: Variable,
    ) -> Result<(), CircuitError> {
        if left.terms.is_empty() {
            return self.constrain_sum(scaled(right, left.constant), Some(output));
        }
        if right.terms.is_empty() {
            return self.constrain_sum(scaled(left, right.constant), Some(output));
        }
        let (wide, narrow) =
            if left.terms.len() >= right.terms.len() { (left, right) } else { (right, left) };
        let narrow = self.narrowed(narrow, 1)?;
        let wide = self.narrowed(wide, 2)?;
        let (y, gamma) = narrow.terms[0];
        let (beta, delta) = (wide.constant, narrow.constant);
        let zero = self.cs.zero();
        let mut wires = [zero, y, zero, zero, output];
        let mut q_lc = [Scalar::zero(), beta * gamma, Scalar::zero(), Scalar::zero()];
        let mut q_mul = [Scalar::zero(); 2];
        for (slot, (x, alpha)) in wide.terms.iter().enumerate() {
            wires[2 * slot] = *x;
            wires[2 * slot + 1] = y;
            q_mul[slot] = *alpha * gamma;
            q_lc[2 * slot] = *alpha * delta;
        }
        self.cs.quad_poly_gate(&wires, &q_lc, &q_mul, Scalar::one(), beta * delta)
    }

    /// The same sum with at most `limit` terms: wider sums are replaced by one variable holding
    /// their wire part, the constant kept aside.
    fn narrowed(&mut self, sum: Sum, limit: usize) -> Result<Sum, CircuitError> {
        if sum.terms.len() <= limit {
            return Ok(sum);
        }
        let constant = sum.constant;
        let variable = self.materialize(Sum { terms: sum.terms, constant: Scalar::zero() })?;
        Ok(Sum { terms: vec![(variable, Scalar::one())], constant })
    }

    /// A variable equal to `sum`: the variable itself when the sum is one, otherwise a new one.
    fn materialize(&mut self, sum: Sum) -> Result<Variable, CircuitError> {
        if let [(variable, coefficient)] = sum.terms.as_slice()
            && coefficient.is_one()
            && sum.constant.is_zero()
        {
            return Ok(*variable);
        }
        let mut value = sum.constant;
        for (variable, coefficient) in &sum.terms {
            value += self.cs.witness(*variable)? * coefficient;
        }
        let variable = self.cs.create_variable(value)?;
        self.constrain_sum(sum, Some(variable))?;
        Ok(variable)
    }
}

/// `factor · sum`, for a product with a constant factor.
fn scaled(mut sum: Sum, factor: Scalar) -> Sum {
    for (_, coefficient) in &mut sum.terms {
        *coefficient *= factor;
    }
    sum.constant *= factor;
    sum
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use jf_relation::Circuit as _;
    use zk_circuit::gadgets::range_check;
    use zk_circuit::{Assignment, Circuit, CircuitBuilder, LinearCombination, ZkField};
    use zk_core::{ExampleId, InstanceKind};

    use super::build;
    use crate::field::Fr;
    use crate::ranges::RangePlan;

    /// Whether jellyfish's own satisfiability check accepts the translation of `assignment`.
    fn satisfied(circuit: &Circuit<Fr>, plan: &RangePlan, assignment: &Assignment<Fr>) -> bool {
        let values = circuit.evaluate_unchecked(assignment).unwrap().values;
        let built = build(circuit, plan, &values).unwrap();
        let public = built.circuit.public_input().unwrap();
        built.circuit.check_circuit_satisfiability(&public).is_ok()
    }

    /// The translation accepts a sample witness exactly when the museum's circuit does.
    #[test]
    fn every_sample_claim_is_satisfied_in_jellyfish_exactly_when_it_is_in_the_museum() {
        for example in ExampleId::ALL {
            let circuit = example.circuit::<Fr>().unwrap();
            let plan = RangePlan::find(&circuit);
            for kind in [InstanceKind::Honest, InstanceKind::Dishonest] {
                let assignment = example.instance::<Fr>(kind, &[7; 32]);
                let museum = circuit.evaluate_unchecked(&assignment).unwrap().violated.is_empty();
                assert_eq!(satisfied(&circuit, &plan, &assignment), museum, "{example:?} {kind:?}");
            }
        }
    }

    /// A bit that another gate reads is part of the statement, so the check stays bit gates.
    #[test]
    fn a_range_check_whose_bits_are_read_elsewhere_is_kept_as_bits_and_still_holds() {
        let mut builder = CircuitBuilder::<Fr>::new().unwrap();
        let value = builder.private_input("value");
        let bits = range_check(&mut builder, value, 4, "value fits in 4 bits").unwrap();
        let parity = builder.public_input("parity");
        builder.assert_zero(LinearCombination::from(bits[0]) - parity, "parity is bit 0");
        let circuit = builder.finish().unwrap();
        let plan = RangePlan::find(&circuit);
        assert_eq!(plan.checks(), 0);
        let claim = |value: u64, parity: u64| Assignment {
            public: vec![Fr::from_u64(parity)],
            private: vec![Fr::from_u64(value)],
        };
        assert!(satisfied(&circuit, &plan, &claim(15, 1)));
        assert!(!satisfied(&circuit, &plan, &claim(16, 0)));
    }
}
