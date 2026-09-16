//! A circuit prepared for the three parties: its gates as operations on wire indices, counted.

use zk_circuit::{Assignment, Circuit, Gate, LinearCombination, Visibility, Wire, ZkField};
use zk_examples::ExampleId;

use crate::digest::circuit_digest;
use crate::error::BooError;
use crate::field::Fp;
use crate::hash::Bytes32;
use crate::party::Party;

/// `Σ coefficient·share + constant`, where only P1 adds the constant (the paper's "add α" gate).
#[derive(Clone, Debug)]
pub(crate) struct Lc {
    terms: Vec<(usize, Fp)>,
    constant: Fp,
}

impl Lc {
    fn of(lc: &LinearCombination<Fp>) -> Self {
        let terms = lc.terms().iter().map(|(wire, c)| (wire.index(), *c)).collect();
        Self { terms, constant: lc.constant_term() }
    }

    /// `party`'s share of the combination, given its share of every wire.
    pub(crate) fn share(&self, held: &[Fp], party: Party) -> Fp {
        let terms =
            self.terms.iter().fold(Fp::zero(), |acc, (wire, c)| acc.add(held[*wire].mul(*c)));
        if party == Party::P1 { terms.add(self.constant) } else { terms }
    }
}

/// One step every party takes, in gate order.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// P1 holds the value, the others zero.
    Constant { out: usize, value: Fp },
    /// P1 holds public input `index`, the others zero.
    Public { out: usize, index: usize },
    /// Each party holds its share of witness value `index`.
    Secret { out: usize, index: usize },
    /// Each party combines its own shares.
    Linear { out: usize, lc: Lc },
    /// A multiplication: each party mixes its shares with its neighbour's.
    Mul { out: usize, left: Lc, right: Lc },
    /// Each party's share of a combination that must be zero: an output share.
    Assert { lc: Lc },
}

/// A statement ZKBoo proves: one circuit, compiled for three parties, with its digest.
#[derive(Clone, Debug)]
pub struct Statement {
    id: String,
    circuit: Circuit<Fp>,
    ops: Vec<Op>,
    secret_wires: Vec<Wire>,
    multiplications: usize,
    assertions: usize,
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
    pub fn new(example: ExampleId) -> Result<Self, BooError> {
        let circuit = example.circuit::<Fp>().map_err(|e| BooError::Circuit(format!("{e:?}")))?;
        Ok(Self::from_circuit(example.id(), circuit))
    }

    /// Any circuit under a stable ID; the ID and the gates are both bound into the challenge.
    pub fn from_circuit(id: &str, circuit: Circuit<Fp>) -> Self {
        let (mut ops, mut secret_wires) = (Vec::new(), Vec::new());
        let (mut publics, mut multiplications, mut assertions) = (0, 0, 0);
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
                    ops.push(Op::Mul {
                        out: output.index(),
                        left: Lc::of(left),
                        right: Lc::of(right),
                    });
                    multiplications += 1;
                }
                Gate::AssertZero { lc, .. } => {
                    ops.push(Op::Assert { lc: Lc::of(lc) });
                    assertions += 1;
                }
            }
        }
        let digest = circuit_digest(&circuit);
        let id = id.to_string();
        Self { id, circuit, ops, secret_wires, multiplications, assertions, digest }
    }

    /// The example ID bound into the challenge.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The circuit.
    pub fn circuit(&self) -> &Circuit<Fp> {
        &self.circuit
    }

    pub(crate) fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Number of public inputs.
    pub fn public_inputs(&self) -> usize {
        self.circuit.public_inputs().len()
    }

    /// Multiplication gates `M`: one output share per party each, in every view.
    pub fn multiplications(&self) -> usize {
        self.multiplications
    }

    /// Assert-zero gates `A`: three output shares each, in every round's first message.
    pub fn assertions(&self) -> usize {
        self.assertions
    }

    /// Witness values `S` shared among the parties: private inputs and hint outputs.
    pub fn witness_values(&self) -> usize {
        self.secret_wires.len()
    }

    /// SHA-256 of the gates in the byte format of the circuit digest.
    pub fn digest(&self) -> Bytes32 {
        self.digest
    }

    /// The claim an assignment makes: its public inputs, and the witness values computed by
    /// evaluating the circuit without stopping at failed assertions.
    pub fn claim(&self, assignment: &Assignment<Fp>) -> Result<Claim, BooError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| BooError::Circuit(format!("{e:?}")))?;
        let witness = self.secret_wires.iter().map(|wire| evaluation.values.get(*wire)).collect();
        Ok(Claim { public: assignment.public.clone(), witness })
    }

    /// Refuses a public input list of the wrong length before any party runs.
    pub(crate) fn check_public(&self, public: &[Fp]) -> Result<(), BooError> {
        let expected = self.public_inputs();
        if public.len() == expected {
            Ok(())
        } else {
            Err(BooError::PublicInputCount { expected, got: public.len() })
        }
    }

    /// Refuses a claim whose witness was not computed for this circuit.
    pub(crate) fn check_claim(&self, claim: &Claim) -> Result<(), BooError> {
        self.check_public(&claim.public)?;
        let expected = self.witness_values();
        if claim.witness.len() == expected {
            Ok(())
        } else {
            Err(BooError::WitnessCount { expected, got: claim.witness.len() })
        }
    }
}
