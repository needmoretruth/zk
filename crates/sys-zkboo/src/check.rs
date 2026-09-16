//! The verifier's side of one round (Fig. 6, Verify): given the first message, the challenge and
//! the response, which checks pass.

use zk_circuit::ZkField;

use crate::field::Fp;
use crate::mpc::{Multiplier, execute, share_product};
use crate::party::Party;
use crate::program::Statement;
use crate::round::{FirstMessage, OpenedView, Response};
use crate::tape::Tape;

/// What one check looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckKind {
    /// The response opens views `e` and `e + 1` in that order, every list has the length the
    /// circuit fixes, and the public inputs have the circuit's count.
    ResponseFitsChallenge,
    /// Every assertion's three output shares sum to zero (the paper's `Rec(y_1, y_2, y_3) = y`).
    OutputsSumToZero,
    /// Every multiplication output in view `e`, recomputed from views `e` and `e + 1`, equals the
    /// opened one (the paper's check 3).
    MultiplicationsRecomputed,
    /// An opened party's output shares, computed from its view, equal the ones it sent first.
    OutputSharesMatch(Party),
    /// An opened view hashes to its commitment.
    ViewCommitment(Party),
}

/// One check's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    /// What was checked.
    pub kind: CheckKind,
    /// Whether it held.
    pub passed: bool,
    /// For [`CheckKind::MultiplicationsRecomputed`] the first bad multiplication, for the output
    /// checks the first bad assertion, both counted from 0 in gate order.
    pub first_failure: Option<usize>,
}

impl Check {
    fn of(kind: CheckKind, passed: bool) -> Self {
        Self { kind, passed, first_failure: None }
    }

    fn located(kind: CheckKind, first_failure: Option<usize>) -> Self {
        Self { kind, passed: first_failure.is_none(), first_failure }
    }
}

/// The verifier's multiplications: view `e`'s share recomputed and compared, then both parties
/// carry on with the opened values, as the paper checks each entry against the views as given.
struct Replay<'a> {
    randomness: [&'a [Fp]; 2],
    opened: [&'a [Fp]; 2],
    first_mismatch: Option<usize>,
}

impl Multiplier for Replay<'_> {
    fn multiply(&mut self, gate: usize, factors: &[(Fp, Fp)], out: &mut [Fp]) {
        let (own, next) = (self.randomness[0][gate], self.randomness[1][gate]);
        let recomputed = share_product(factors[0], factors[1], own, next);
        if recomputed != self.opened[0][gate] && self.first_mismatch.is_none() {
            self.first_mismatch = Some(gate);
        }
        out[0] = self.opened[0][gate];
        out[1] = self.opened[1][gate];
    }
}

/// Whether `response` opens the views `challenge` asks for, with the circuit's lengths.
pub(crate) fn response_fits(statement: &Statement, challenge: Party, response: &Response) -> bool {
    let fits = |view: &OpenedView, party: Party| {
        let inputs = if party == Party::P3 { statement.witness_values() } else { 0 };
        view.party == party
            && view.input_shares.len() == inputs
            && view.mul_outputs.len() == statement.multiplications()
    };
    let [first, second] = &response.views;
    fits(first, challenge) && fits(second, challenge.next())
}

/// Runs every check one round calls for.
pub fn check_round(
    statement: &Statement,
    public: &[Fp],
    first: &FirstMessage,
    challenge: Party,
    response: &Response,
) -> Vec<Check> {
    let fits = response_fits(statement, challenge, response)
        && first.output_shares.len() == statement.assertions()
        && public.len() == statement.public_inputs();
    if !fits {
        return vec![Check::of(CheckKind::ResponseFitsChallenge, false)];
    }
    let mut checks = vec![Check::of(CheckKind::ResponseFitsChallenge, true)];
    let bad_sum = first.output_shares.iter().position(|[a, b, c]| a.add(*b).add(*c) != Fp::zero());
    checks.push(Check::located(CheckKind::OutputsSumToZero, bad_sum));
    let views = &response.views;
    let tapes = [Tape::open(statement, &views[0]), Tape::open(statement, &views[1])];
    let mut replay = Replay {
        randomness: [&tapes[0].randomness, &tapes[1].randomness],
        opened: [&views[0].mul_outputs, &views[1].mul_outputs],
        first_mismatch: None,
    };
    let parties = [challenge, challenge.next()];
    let inputs: [&[Fp]; 2] = [&tapes[0].inputs, &tapes[1].inputs];
    let execution = execute(statement, public, &parties, &inputs, &mut replay);
    checks.push(Check::located(CheckKind::MultiplicationsRecomputed, replay.first_mismatch));
    for (view, computed) in views.iter().zip(&execution.output_shares) {
        let sent = first.output_shares.iter().map(|shares| shares[view.party.index()]);
        let bad = computed.iter().zip(sent).position(|(computed, sent)| *computed != sent);
        checks.push(Check::located(CheckKind::OutputSharesMatch(view.party), bad));
    }
    for view in views {
        let passed = view.commitment() == first.commitments[view.party.index()];
        checks.push(Check::of(CheckKind::ViewCommitment(view.party), passed));
    }
    checks
}
