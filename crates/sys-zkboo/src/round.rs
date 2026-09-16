//! One round from the prover's side (Fig. 6): sample tapes, run the three parties, commit; then
//! open two views for the challenge.

use zk_circuit::ZkField;

use crate::error::BooError;
use crate::field::Fp;
use crate::hash::{Bytes32, fresh_seed, view_commitment};
use crate::mpc::{Multiplier, execute, share_product};
use crate::party::Party;
use crate::program::{Claim, Statement};
use crate::tape::{completing_shares, multiplication_randomness, seeded_input_shares};

/// What the prover sends before the challenge: the paper's first message `a = (y_1, y_2, y_3,
/// c_1, c_2, c_3)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstMessage {
    /// `c_1, c_2, c_3`: a commitment to each party's view.
    pub commitments: [Bytes32; 3],
    /// Per assert-zero gate in gate order, the three parties' output shares `(y_1, y_2, y_3)`. All
    /// three are sent before the challenge, so the unopened party's share is fixed in advance.
    pub output_shares: Vec<[Fp; 3]>,
}

/// One party's view as the prover opens it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedView {
    /// Whose view.
    pub party: Party,
    /// `k_i`, from which the verifier expands the party's tape.
    pub seed: Bytes32,
    /// P3's witness shares, which no seed produces; empty for P1 and P2.
    pub input_shares: Vec<Fp>,
    /// The party's share of every multiplication output, in gate order.
    pub mul_outputs: Vec<Fp>,
}

impl OpenedView {
    /// The commitment this view should match.
    pub fn commitment(&self) -> Bytes32 {
        view_commitment(&self.seed, &self.input_shares, &self.mul_outputs)
    }
}

/// The prover's answer to challenge `e`: views `e` and `e + 1`, in that order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// View `e`, which the verifier recomputes, then view `e + 1`, which it takes as given.
    pub views: [OpenedView; 2],
}

/// A departure from the protocol, for demonstrations: `party` adds `shift` to its share of the
/// output of multiplication `multiplication` (counted from 0 in gate order), and every later step
/// uses the shifted value, so the only inconsistency is at that one gate of that one view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cheat {
    /// The party whose computation is broken.
    pub party: Party,
    /// Which multiplication, counted from 0 in gate order.
    pub multiplication: usize,
    /// What is added to that party's output share.
    pub shift: Fp,
}

/// A committed round: what the prover keeps until the challenge arrives.
#[derive(Clone, Debug)]
pub struct Round {
    seeds: [Bytes32; 3],
    third_shares: Vec<Fp>,
    mul_outputs: [Vec<Fp>; 3],
    first: FirstMessage,
}

/// The prover's multiplications: every party's share by the formula, then the cheat if any.
struct Honest<'a> {
    randomness: [&'a [Fp]; 3],
    cheat: Option<Cheat>,
}

impl Multiplier for Honest<'_> {
    fn multiply(&mut self, gate: usize, factors: &[(Fp, Fp)], out: &mut [Fp]) {
        for (index, slot) in out.iter_mut().enumerate() {
            let next = (index + 1) % 3;
            let (own, neighbour) = (self.randomness[index][gate], self.randomness[next][gate]);
            *slot = share_product(factors[index], factors[next], own, neighbour);
        }
        if let Some(cheat) = self.cheat.filter(|cheat| cheat.multiplication == gate) {
            let slot = &mut out[cheat.party.index()];
            *slot = slot.add(cheat.shift);
        }
    }
}

impl Round {
    /// Samples three seeds, shares the claim's witness, runs the three parties (departing as
    /// `cheat` says) and commits. The witness is not checked: a false claim commits to output
    /// shares that do not sum to zero.
    pub fn commit(
        statement: &Statement,
        claim: &Claim,
        cheat: Option<Cheat>,
    ) -> Result<Self, BooError> {
        statement.check_claim(claim)?;
        let mut seeds = [[0u8; 32]; 3];
        for seed in &mut seeds {
            *seed = fresh_seed().map_err(BooError::Randomness)?;
        }
        let (s, m) = (statement.witness_values(), statement.multiplications());
        let first = seeded_input_shares(&seeds[0], s);
        let second = seeded_input_shares(&seeds[1], s);
        let third_shares = completing_shares(&claim.witness, &first, &second);
        let randomness = seeds.map(|seed| multiplication_randomness(&seed, m));
        let mut multiplier =
            Honest { randomness: [&randomness[0], &randomness[1], &randomness[2]], cheat };
        let inputs: [&[Fp]; 3] = [&first, &second, &third_shares];
        let execution = execute(statement, &claim.public, &Party::ALL, &inputs, &mut multiplier);
        let mut outputs = execution.mul_outputs;
        let mul_outputs: [Vec<Fp>; 3] = core::array::from_fn(|i| core::mem::take(&mut outputs[i]));
        let shares = &execution.output_shares;
        let output_shares = (0..statement.assertions())
            .map(|t| [shares[0][t], shares[1][t], shares[2][t]])
            .collect();
        let commitments = Party::ALL.map(|party| {
            let own = if party == Party::P3 { third_shares.as_slice() } else { &[] };
            view_commitment(&seeds[party.index()], own, &mul_outputs[party.index()])
        });
        let first = FirstMessage { commitments, output_shares };
        Ok(Self { seeds, third_shares, mul_outputs, first })
    }

    /// The first message, to send before the challenge.
    pub fn first_message(&self) -> &FirstMessage {
        &self.first
    }

    /// Opens views `challenge` and `challenge + 1`.
    pub fn open(&self, challenge: Party) -> Response {
        let view = |party: Party| OpenedView {
            party,
            seed: self.seeds[party.index()],
            input_shares: if party == Party::P3 { self.third_shares.clone() } else { Vec::new() },
            mul_outputs: self.mul_outputs[party.index()].clone(),
        };
        Response { views: [view(challenge), view(challenge.next())] }
    }
}
