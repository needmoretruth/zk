//! The three parties' computation (the paper's protocol Π*_φ for the linear decomposition, §4.1),
//! run for all three (the prover) or with the multiplications supplied by the caller (the
//! verifier's replay of two views, the search for a hidden view).
//!
//! Every wire is held as additive shares. P1 alone adds constants and public inputs; linear gates
//! are computed share by share. A multiplication `z = x·y` gives party `i`
//!
//! ```text
//! z_i = x_i·y_i + x_{i+1}·y_i + x_i·y_{i+1} + R_i(c) − R_{i+1}(c)      (indices mod 3)
//! ```
//!
//! exactly as the paper writes it: summed over the three parties, each of the nine products
//! `x_j·y_k` appears once and the random terms cancel, so `Σ z_i = x·y`. An assert-zero gate's
//! share of its combination is that party's output share.

use zk_circuit::ZkField;

use crate::field::Fp;
use crate::party::Party;
use crate::program::{Op, Statement};

/// Where a multiplication's output shares come from.
pub(crate) trait Multiplier {
    /// Fills `out` with the present parties' shares of multiplication `gate`'s output, given each
    /// present party's shares `(x, y)` of the two factors, both in the order the parties were given.
    fn multiply(&mut self, gate: usize, factors: &[(Fp, Fp)], out: &mut [Fp]);
}

/// What a computation produced, per present party in the order given.
#[derive(Clone, Debug)]
pub(crate) struct Execution {
    /// Each party's share of every multiplication output: the computed part of its view.
    pub(crate) mul_outputs: Vec<Vec<Fp>>,
    /// Each party's share of every assert-zero combination: its output shares.
    pub(crate) output_shares: Vec<Vec<Fp>>,
}

/// Party `i`'s share of a product, from its own factor shares, its neighbour's, and both tapes'
/// randomness for this gate.
pub(crate) fn share_product(own: (Fp, Fp), next: (Fp, Fp), own_random: Fp, next_random: Fp) -> Fp {
    let (x, y) = own;
    let (x_next, y_next) = next;
    x.mul(y).add(x_next.mul(y)).add(x.mul(y_next)).add(own_random).sub(next_random)
}

/// Runs the circuit for `parties`, each holding `inputs[k]` as its witness shares. The caller has
/// checked the public input count and that every input list has the circuit's length.
pub(crate) fn execute(
    statement: &Statement,
    public: &[Fp],
    parties: &[Party],
    inputs: &[&[Fp]],
    multiplier: &mut dyn Multiplier,
) -> Execution {
    let count = parties.len();
    let mut shares = vec![vec![Fp::zero(); statement.circuit().num_wires()]; count];
    let mut mul_outputs = vec![Vec::with_capacity(statement.multiplications()); count];
    let mut output_shares = vec![Vec::with_capacity(statement.assertions()); count];
    let mut products = vec![Fp::zero(); count];
    let mut gate = 0;
    for op in statement.ops() {
        match op {
            Op::Constant { out, value } => set_first(&mut shares, parties, *out, *value),
            Op::Public { out, index } => {
                let value = public.get(*index).copied().unwrap_or_else(Fp::zero);
                set_first(&mut shares, parties, *out, value);
            }
            Op::Secret { out, index } => {
                for (held, input) in shares.iter_mut().zip(inputs) {
                    held[*out] = input.get(*index).copied().unwrap_or_else(Fp::zero);
                }
            }
            Op::Linear { out, lc } => {
                for (held, party) in shares.iter_mut().zip(parties) {
                    held[*out] = lc.share(held, *party);
                }
            }
            Op::Mul { out, left, right } => {
                let shared = shares.iter().zip(parties);
                let factors: Vec<_> =
                    shared.map(|(held, p)| (left.share(held, *p), right.share(held, *p))).collect();
                multiplier.multiply(gate, &factors, &mut products);
                for (index, product) in products.iter().enumerate() {
                    shares[index][*out] = *product;
                    mul_outputs[index].push(*product);
                }
                gate += 1;
            }
            Op::Assert { lc } => {
                for ((held, party), outputs) in shares.iter().zip(parties).zip(&mut output_shares) {
                    outputs.push(lc.share(held, *party));
                }
            }
        }
    }
    Execution { mul_outputs, output_shares }
}

fn set_first(shares: &mut [Vec<Fp>], parties: &[Party], out: usize, value: Fp) {
    for (held, party) in shares.iter_mut().zip(parties) {
        held[out] = if *party == Party::P1 { value } else { Fp::zero() };
    }
}
