//! A circuit compiled into commitments and checks, identically by prover and verifier.

use ff::Field as _;
use zk_circuit::{Circuit, Gate, LinearCombination, Visibility, Wire};

use crate::affine::Affine;
use crate::field::PallasScalar;

/// One thing the proof has to establish about committed values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Check {
    /// `output = left · right` for two values that both involve commitments; `output` is the index of
    /// the commitment the prover publishes for the product.
    Product {
        /// The factor `a`, used as a base point.
        left: Affine,
        /// The factor `b`, whose opening is proved.
        right: Affine,
        /// Commitment index of `c`.
        output: usize,
    },
    /// A value involving commitments is zero: its commitment is a multiple of `H` alone.
    Zero(Affine),
    /// A value the verifier computes itself (public inputs and constants only) is zero.
    KnownZero(PallasScalar),
}

/// The commitments a proof publishes and the checks it proves about them.
///
/// Built from the circuit and the public inputs only, so the verifier rebuilds exactly what the
/// prover used. A product with a public factor is linear here, and a factor of zero drops every
/// commitment from a value, which is why the public inputs take part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The wire behind each published commitment, in gate order: private inputs, hint outputs and
    /// products of two committed values.
    pub(crate) committed: Vec<Wire>,
    /// Checks in gate order.
    pub(crate) checks: Vec<Check>,
}

impl Layout {
    /// Walks the gates once. Missing public inputs read as zero; callers check the count first.
    pub(crate) fn new(circuit: &Circuit<PallasScalar>, public: &[PallasScalar]) -> Self {
        let mut walk = Walk {
            wires: vec![Affine::default(); circuit.num_wires()],
            layout: Layout { committed: Vec::new(), checks: Vec::new() },
        };
        let mut public = public.iter();
        for gate in circuit.gates() {
            match gate {
                Gate::Constant { output, value } => walk.set(*output, Affine::known(*value)),
                Gate::Input { output, visibility: Visibility::Public, .. } => {
                    let value = public.next().copied().unwrap_or(PallasScalar::ZERO);
                    walk.set(*output, Affine::known(value));
                }
                Gate::Input { output, visibility: Visibility::Private, .. } => walk.commit(*output),
                Gate::Linear { output, lc } => {
                    let value = walk.combine(lc);
                    walk.set(*output, value);
                }
                Gate::Mul { output, left, right } => walk.product(*output, left, right),
                Gate::AssertZero { lc, .. } => {
                    let value = walk.combine(lc);
                    let check = match value.known_value() {
                        Some(known) => Check::KnownZero(known),
                        None => Check::Zero(value),
                    };
                    walk.layout.checks.push(check);
                }
                Gate::Hint { outputs, .. } => {
                    outputs.iter().for_each(|output| walk.commit(*output))
                }
            }
        }
        walk.layout
    }

    /// Products of two committed values.
    pub(crate) fn products(&self) -> usize {
        self.checks.iter().filter(|check| matches!(check, Check::Product { .. })).count()
    }

    /// Zero checks on committed values.
    pub(crate) fn zeros(&self) -> usize {
        self.checks.iter().filter(|check| matches!(check, Check::Zero(_))).count()
    }

    /// Linear equations handed to `sigma-proofs`: two per product, one per zero check.
    pub(crate) fn equations(&self) -> usize {
        2 * self.products() + self.zeros()
    }

    /// Secret scalars the sigma protocol proves knowledge of: `b`, `r_b`, `s` per product, `t` per zero.
    pub(crate) fn witness_scalars(&self) -> usize {
        3 * self.products() + self.zeros()
    }
}

struct Walk {
    wires: Vec<Affine>,
    layout: Layout,
}

impl Walk {
    fn set(&mut self, wire: Wire, value: Affine) {
        self.wires[wire.index()] = value;
    }

    fn commit(&mut self, wire: Wire) {
        let index = self.layout.committed.len();
        self.layout.committed.push(wire);
        self.set(wire, Affine::committed(index));
    }

    fn combine(&self, lc: &LinearCombination<PallasScalar>) -> Affine {
        let parts = lc.terms().iter().map(|(wire, weight)| (&self.wires[wire.index()], *weight));
        Affine::combine(parts, lc.constant_term())
    }

    /// A product is linear when either factor is known; otherwise `c` gets its own commitment.
    fn product(
        &mut self,
        output: Wire,
        left: &LinearCombination<PallasScalar>,
        right: &LinearCombination<PallasScalar>,
    ) {
        let (left, right) = (self.combine(left), self.combine(right));
        if let Some(factor) = left.known_value() {
            self.set(output, right.scaled(factor));
        } else if let Some(factor) = right.known_value() {
            self.set(output, left.scaled(factor));
        } else {
            self.commit(output);
            let output = self.layout.committed.len() - 1;
            self.layout.checks.push(Check::Product { left, right, output });
        }
    }
}
