//! What each party computes from apart from its neighbour: its witness shares and its randomness
//! `R_i(c)`, both drawn from its seed `k_i` except P3's witness shares, which the witness fixes.
//!
//! The paper's `Share` samples any three shares that sum to the input. Here P1's and P2's shares
//! come from their seeds and P3's are `x_3 = x − x_1 − x_2`, so two of the three shares never need
//! to be stored: any two shares are still uniformly random and independent of `x`.

use zk_circuit::ZkField;

use crate::field::Fp;
use crate::hash::{Bytes32, SeedStream};
use crate::party::Party;
use crate::program::Statement;
use crate::round::OpenedView;

/// One party's tape, expanded.
#[derive(Clone, Debug)]
pub(crate) struct Tape {
    /// Its share of every witness value.
    pub(crate) inputs: Vec<Fp>,
    /// `R_i(c)` for every multiplication `c`.
    pub(crate) randomness: Vec<Fp>,
}

/// `x_i` for P1 or P2: one element per witness value from the seed's input tape.
pub(crate) fn seeded_input_shares(seed: &Bytes32, witness_values: usize) -> Vec<Fp> {
    SeedStream::inputs(seed).take(witness_values)
}

/// `R_i(1), R_i(2), …` from any party's seed.
pub(crate) fn multiplication_randomness(seed: &Bytes32, multiplications: usize) -> Vec<Fp> {
    SeedStream::multiplications(seed).take(multiplications)
}

/// The share that completes two others to `values`: `x_3 = x − x_1 − x_2`.
pub(crate) fn completing_shares(values: &[Fp], first: &[Fp], second: &[Fp]) -> Vec<Fp> {
    values.iter().zip(first).zip(second).map(|((x, a), b)| x.sub(*a).sub(*b)).collect()
}

impl Tape {
    /// An opened party's tape as the verifier rebuilds it: P1 and P2 from the seed alone, P3's
    /// witness shares from its view. The caller has checked the view's lengths.
    pub(crate) fn open(statement: &Statement, view: &OpenedView) -> Self {
        let inputs = if view.party == Party::P3 {
            view.input_shares.clone()
        } else {
            seeded_input_shares(&view.seed, statement.witness_values())
        };
        let randomness = multiplication_randomness(&view.seed, statement.multiplications());
        Self { inputs, randomness }
    }
}
