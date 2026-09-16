//! A circuit prepared for the three friends: its gates as operations on wire indices, counted.

use zk_circuit::{Assignment, Circuit, Gate, LinearCombination, Visibility, Wire, ZkField};
use zk_examples::ExampleId;

use crate::digest::circuit_digest;
use crate::error::TrioError;
use crate::field::Fp;
use crate::hash::Bytes32;

/// `Σ coefficient·share + constant`, where only P1 adds the constant.
#[derive(Clone, Debug)]
pub(crate) struct Lc {
    pub(crate) terms: Vec<(usize, Fp)>,
    pub(crate) constant: Fp,
}

impl Lc {
    fn of(lc: &LinearCombination<Fp>) -> Self {
        let terms = lc.terms().iter().map(|(wire, c)| (wire.index(), *c)).collect();
        Self { terms, constant: lc.constant_term() }
    }

    /// The combination of `values` without the constant.
    pub(crate) fn terms_at(&self, values: &[Fp]) -> Fp {
        self.terms.iter().fold(Fp::zero(), |acc, (wire, c)| acc.add(values[*wire].mul(*c)))
    }
}

/// One step every friend takes, in gate order.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// P1 holds the value, the others zero.
    Constant { out: usize, value: Fp },
    /// P1 holds public input `index`, the others zero.
    Public { out: usize, index: usize },
    /// Each friend holds its share of witness value `index`.
    Secret { out: usize, index: usize },
    /// Each friend combines its own shares.
    Linear { out: usize, lc: Lc },
    /// A multiplication with the next dealer card; broadcasts `d` and `e`.
    Mul { out: usize, left: Lc, right: Lc },
    /// Each friend broadcasts its share of a value that must be zero.
    Assert { lc: Lc },
}

/// A statement Trio proves: one circuit, compiled for three friends, with its digest.
#[derive(Clone, Debug)]
pub struct Statement {
    id: String,
    circuit: Circuit<Fp>,
    pub(crate) ops: Vec<Op>,
    secret_wires: Vec<Wire>,
    multiplications: usize,
    assertions: usize,
    pub(crate) assert_slots: Vec<usize>,
    digest: Bytes32,
}

/// A claim to prove: public inputs and the prover's witness values (private inputs and hint
/// outputs, in gate order). Nothing checks that the witness is valid; that is the verifier's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Public inputs in declaration order.
    pub public: Vec<Fp>,
    pub(crate) witness: Vec<Fp>,
}

impl Statement {
    /// The statement of one of the museum's examples, over Goldilocks.
    pub fn new(example: ExampleId) -> Result<Self, TrioError> {
        let circuit = example.circuit::<Fp>().map_err(|e| TrioError::Circuit(format!("{e:?}")))?;
        Ok(Self::from_circuit(example.id(), circuit))
    }

    /// Any circuit under a stable ID; the ID and the gates are both bound into the challenge.
    pub fn from_circuit(id: &str, circuit: Circuit<Fp>) -> Self {
        let (mut ops, mut secret_wires, mut assert_slots) = (Vec::new(), Vec::new(), Vec::new());
        let (mut publics, mut multiplications) = (0, 0);
        for gate in circuit.gates() {
            match gate {
                Gate::Constant { output, value } => {
                    ops.push(Op::Constant { out: output.index(), value: *value });
                }
                Gate::Input { output, visibility: Visibility::Public, .. } => {
                    ops.push(Op::Public { out: output.index(), index: publics });
                    publics += 1;
                }
                Gate::Input { output, visibility: Visibility::Private, .. } => {
                    ops.push(Op::Secret { out: output.index(), index: secret_wires.len() });
                    secret_wires.push(*output);
                }
                Gate::Hint { outputs, .. } => {
                    for output in outputs {
                        ops.push(Op::Secret { out: output.index(), index: secret_wires.len() });
                        secret_wires.push(*output);
                    }
                }
                Gate::Linear { output, lc } => {
                    ops.push(Op::Linear { out: output.index(), lc: Lc::of(lc) });
                }
                Gate::Mul { output, left, right } => {
                    let (left, right) = (Lc::of(left), Lc::of(right));
                    ops.push(Op::Mul { out: output.index(), left, right });
                    multiplications += 1;
                }
                Gate::AssertZero { lc, .. } => {
                    assert_slots.push(2 * multiplications + assert_slots.len());
                    ops.push(Op::Assert { lc: Lc::of(lc) });
                }
            }
        }
        let digest = circuit_digest(&circuit);
        let assertions = assert_slots.len();
        let id = id.to_string();
        Self { id, circuit, ops, secret_wires, multiplications, assertions, assert_slots, digest }
    }

    /// The example ID bound into the challenge.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The circuit.
    pub fn circuit(&self) -> &Circuit<Fp> {
        &self.circuit
    }

    /// Number of public inputs.
    pub fn public_inputs(&self) -> usize {
        self.circuit.public_inputs().len()
    }

    /// Multiplication gates: one dealer card each, and two broadcasts per friend each.
    pub fn multiplications(&self) -> usize {
        self.multiplications
    }

    /// Assert-zero gates: one broadcast per friend each.
    pub fn assertions(&self) -> usize {
        self.assertions
    }

    /// Witness values shared among the friends: private inputs and hint outputs.
    pub fn witness_values(&self) -> usize {
        self.secret_wires.len()
    }

    /// Elements each friend broadcasts: `2·multiplications + assertions`.
    pub fn broadcasts_per_friend(&self) -> usize {
        2 * self.multiplications + self.assertions
    }

    /// SHA-256 of the gates in the byte format of [`crate::digest`].
    pub fn digest(&self) -> Bytes32 {
        self.digest
    }

    /// The claim an assignment makes: its public inputs, and the witness values computed by
    /// evaluating the circuit without stopping at failed assertions.
    pub fn claim(&self, assignment: &Assignment<Fp>) -> Result<Claim, TrioError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| TrioError::Circuit(format!("{e:?}")))?;
        let witness = self.secret_wires.iter().map(|wire| evaluation.values.get(*wire)).collect();
        Ok(Claim { public: assignment.public.clone(), witness })
    }

    /// Refuses a public input list of the wrong length before any friend runs.
    pub(crate) fn check_public(&self, public: &[Fp]) -> Result<(), TrioError> {
        let expected = self.public_inputs();
        if public.len() == expected {
            Ok(())
        } else {
            Err(TrioError::PublicInputCount { expected, got: public.len() })
        }
    }
}
