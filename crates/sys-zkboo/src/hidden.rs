//! Why two opened views say nothing about the witness: for any witness at all, exactly one view of
//! the hidden party makes the opened pair part of a correct three-party run on it.
//!
//! Given challenge `e`, the pair `(e, e + 1)` and a candidate witness, the hidden party `h = e + 2`
//! takes the witness shares `x_h = x − x_e − x_{e+1}`, and at every multiplication the randomness
//! that reproduces the opened share of `e + 1`:
//!
//! ```text
//! z_{e+1} = x_{e+1}·y_{e+1} + x_h·y_{e+1} + x_{e+1}·y_h + R_{e+1}(c) − R_h(c)
//!   ⇒ R_h(c) = x_{e+1}·y_{e+1} + x_h·y_{e+1} + x_{e+1}·y_h + R_{e+1}(c) − z_{e+1}
//! ```
//!
//! View `e` depends only on the pair, so it is unchanged. The run is a correct execution of the
//! decomposition, so the hidden party's output shares complete the opened ones to the candidate's
//! assertion values: they match the published ones exactly when the candidate satisfies the
//! circuit. A real hidden tape is uniformly random, so every valid witness is equally likely given
//! what was opened (the paper's 2-privacy, Appendix A).

use zk_circuit::ZkField;

use crate::check::response_fits;
use crate::error::BooError;
use crate::field::Fp;
use crate::mpc::{Multiplier, execute, share_product};
use crate::party::Party;
use crate::program::{Claim, Statement};
use crate::round::Response;
use crate::tape::{Tape, completing_shares};

/// The hidden party's view that fits an opened pair to a candidate witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenView {
    /// The unopened party, `e + 2`.
    pub party: Party,
    /// Its share of every witness value.
    pub input_shares: Vec<Fp>,
    /// `R_h(c)` for every multiplication: the tape it would need.
    pub randomness: Vec<Fp>,
    /// Its share of every multiplication output.
    pub mul_outputs: Vec<Fp>,
    /// Its share of every assert-zero combination.
    pub output_shares: Vec<Fp>,
}

/// Recomputes party `e`, takes party `e + 1` as opened, and solves the hidden party's randomness.
struct Explain<'a> {
    randomness: [&'a [Fp]; 2],
    opened_next: &'a [Fp],
    hidden_randomness: Vec<Fp>,
}

impl Multiplier for Explain<'_> {
    fn multiply(&mut self, gate: usize, factors: &[(Fp, Fp)], out: &mut [Fp]) {
        let (own, next, hidden) = (factors[0], factors[1], factors[2]);
        let (r_own, r_next) = (self.randomness[0][gate], self.randomness[1][gate]);
        out[0] = share_product(own, next, r_own, r_next);
        out[1] = self.opened_next[gate];
        let r_hidden = share_product(next, hidden, r_next, Fp::zero()).sub(out[1]);
        self.hidden_randomness.push(r_hidden);
        out[2] = share_product(hidden, own, r_hidden, r_own);
    }
}

/// The one hidden view that makes `response`, opened for `challenge`, part of a correct run on
/// `claim`. Compare its output shares with the first message's to see whether `claim` fits.
pub fn hidden_view_for(
    statement: &Statement,
    claim: &Claim,
    challenge: Party,
    response: &Response,
) -> Result<HiddenView, BooError> {
    statement.check_claim(claim)?;
    if !response_fits(statement, challenge, response) {
        return Err(BooError::ResponseDoesNotFit);
    }
    let [own, next] = &response.views;
    let tapes = [Tape::open(statement, own), Tape::open(statement, next)];
    let input_shares = completing_shares(&claim.witness, &tapes[0].inputs, &tapes[1].inputs);
    let mut explain = Explain {
        randomness: [&tapes[0].randomness, &tapes[1].randomness],
        opened_next: &next.mul_outputs,
        hidden_randomness: Vec::with_capacity(statement.multiplications()),
    };
    let party = Party::hidden_by(challenge);
    let parties = [challenge, challenge.next(), party];
    let inputs: [&[Fp]; 3] = [&tapes[0].inputs, &tapes[1].inputs, &input_shares];
    let mut execution = execute(statement, &claim.public, &parties, &inputs, &mut explain);
    Ok(HiddenView {
        party,
        input_shares,
        randomness: explain.hidden_randomness,
        mul_outputs: core::mem::take(&mut execution.mul_outputs[2]),
        output_shares: core::mem::take(&mut execution.output_shares[2]),
    })
}
