//! The non-interactive proof: [`PROOF_ROUNDS`] rounds committed at once, the challenges drawn from
//! a hash of every first message (Fiat–Shamir), then every response.
//!
//! # One round
//!
//! Each party `i` has a 32-byte seed `k_i` from the operating system. P1's and P2's witness shares
//! come from their seeds; P3's are `x_3 = x − x_1 − x_2`. Constants and public inputs are added by
//! P1 alone, linear gates are computed share by share, and a multiplication `z = x·y` gives party
//! `i` (indices mod 3, `R_i(c)` from `k_i`)
//!
//! ```text
//! z_i = x_i·y_i + x_{i+1}·y_i + x_i·y_{i+1} + R_i(c) − R_{i+1}(c)
//! ```
//!
//! which is the paper's formula for the linear decomposition (§4.1) unchanged. Party `i`'s view is
//! `k_i`, P3's witness shares (no seed makes them) and its share of every multiplication output;
//! its output shares are its shares of every assert-zero combination. Challenge `e` opens views `e`
//! and `e + 1`. The verifier rebuilds both tapes, recomputes every multiplication output of view `e`
//! from the two views, recomputes both parties' output shares, checks both commitments and checks
//! that every assertion's three output shares sum to zero.
//!
//! # Byte format
//!
//! Integers are little-endian, seeds and digests 32 bytes, field elements 8 canonical
//! little-endian bytes. `M` is the circuit's multiplications, `S` its witness values (private
//! inputs and hint outputs) and `A` its assert-zero gates.
//!
//! ```text
//! u32 rounds                                   always 137
//! rounds × first message:
//!     c_1 ‖ c_2 ‖ c_3
//!     (y_1 ‖ y_2 ‖ y_3) × A                    one triple per assert-zero gate, in gate order
//! rounds × response, laid out by that round's challenge e:
//!     view e ‖ view e+1                        after P3 comes P1
//!     view i = k_i ‖ (P3 only: x_3 × S) ‖ z_i × M
//! ```
//!
//! `c_i = SHA-256("zk/zkboo/v1/view" ‖ k_i ‖ (P3 only: x_3 × S) ‖ z_i × M)`. The proof carries no
//! tags, lengths or challenges: the verifier recomputes the challenges from the first messages and
//! then knows how to read each response, and any byte left over or missing makes the proof
//! malformed. A proof is therefore `4 + 137·(96 + 24A) + Σ response` bytes, where a response is
//! `64 + 16M` when the challenge is P1 and `64 + 16M + 8S` when it is P2 or P3 (P3 opened).
//!
//! # Challenges
//!
//! `seed = SHA-256("zk/zkboo/v1/challenge" ‖ u32 id length ‖ example id ‖ circuit digest ‖
//! u32 public inputs ‖ element… ‖ u32 rounds ‖ first message…)`, then bit pairs of
//! `SHA-256("zk/zkboo/v1/trits" ‖ seed ‖ u64 block)` for blocks 0, 1, …, most significant pair of
//! each byte first: `(a, b)` gives party `2a + b + 1`, `(1, 1)` is skipped.

use zk_core::{Control, Verdict};

use crate::challenge::challenges;
use crate::check::check_round;
use crate::codec::{
    Reader, first_message_bytes, read_first, read_response, write_first, write_response,
};
use crate::error::BooError;
use crate::field::Fp;
use crate::party::Party;
use crate::program::{Claim, Statement};
use crate::round::{Cheat, FirstMessage, Response, Round};

/// Rounds in a proof: 137, the paper's number for soundness error 2^-80.
///
/// A false claim survives one round with probability at most 2/3: with three accepting responses
/// to one first message a witness could be extracted (3-special soundness), so at most two of the
/// three challenges can be answered. Rounds are independent, so `r` rounds leave `(2/3)^r`, and an
/// error of 2^-σ needs `r ≥ σ / (log2 3 − 1)` (§4.2, Efficiency). For σ = 80 that is
/// `80 / 0.58496… = 136.8`, so 137, and `(2/3)^137 ≈ 2^-80.1`. The paper runs 137 rounds for its
/// 2^-80 proofs and calls the 69 rounds of 2^-40 not sufficient for non-interactive proofs (§5.3):
/// a prover working alone can recompute the challenge hash as often as it likes, so after `Q` tries
/// a false claim passes with probability at most about `Q · 2^-80`.
pub const PROOF_ROUNDS: u32 = 137;

/// The byte the flip attack alters: the first byte of the first round's response, which is the seed
/// `k_e` of the view the verifier recomputes. The verifier expands its tape from that seed and
/// hashes the seed into `c_e`, so a flipped bit is caught by the commitment check whichever
/// challenge the first round drew; it is never padding, a length or a tag.
pub fn tamper_offset(statement: &Statement) -> usize {
    4 + PROOF_ROUNDS as usize * first_message_bytes(statement.assertions())
}

/// One round of a proof as the verifier reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedRound {
    /// The commitments and output shares sent first.
    pub first: FirstMessage,
    /// The challenge the hash drew.
    pub challenge: Party,
    /// The two views opened for it.
    pub response: Response,
}

/// Proves `claim` without checking it; a false claim still yields bytes, which the verifier rejects.
pub fn prove(statement: &Statement, claim: &Claim, control: &Control) -> Result<Vec<u8>, BooError> {
    prove_rounds(statement, claim, None, control)
}

/// A proof in which every round carries `cheat`, for showing how often the verifier sees it.
pub fn prove_with_cheat(
    statement: &Statement,
    claim: &Claim,
    cheat: Cheat,
    control: &Control,
) -> Result<Vec<u8>, BooError> {
    prove_rounds(statement, claim, Some(cheat), control)
}

fn prove_rounds(
    statement: &Statement,
    claim: &Claim,
    cheat: Option<Cheat>,
    control: &Control,
) -> Result<Vec<u8>, BooError> {
    statement.check_claim(claim)?;
    let mut rounds = Vec::with_capacity(PROOF_ROUNDS as usize);
    for _ in 0..PROOF_ROUNDS {
        control.checkpoint().map_err(|_| BooError::Cancelled)?;
        rounds.push(Round::commit(statement, claim, cheat)?);
    }
    let firsts: Vec<FirstMessage> =
        rounds.iter().map(|round| round.first_message().clone()).collect();
    let drawn = challenges(statement, &claim.public, &firsts);
    let mut proof = Vec::new();
    proof.extend_from_slice(&PROOF_ROUNDS.to_le_bytes());
    for first in &firsts {
        write_first(&mut proof, first);
    }
    for (round, challenge) in rounds.iter().zip(drawn) {
        write_response(&mut proof, &round.open(challenge));
    }
    Ok(proof)
}

/// Reads a proof into its rounds, recomputing the challenges. Any byte that does not decode, a
/// wrong round count, a byte left over or a public input list of the wrong length is an error
/// sentence.
pub fn parse(
    statement: &Statement,
    public: &[Fp],
    proof: &[u8],
) -> Result<Vec<OpenedRound>, String> {
    statement.check_public(public).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(proof);
    let rounds = reader.u32()?;
    if rounds != PROOF_ROUNDS {
        return Err(format!("a proof has {PROOF_ROUNDS} rounds, this one claims {rounds}"));
    }
    let firsts =
        (0..rounds).map(|_| read_first(&mut reader, statement)).collect::<Result<Vec<_>, _>>()?;
    let drawn = challenges(statement, public, &firsts);
    let responses = drawn
        .iter()
        .map(|challenge| read_response(&mut reader, statement, *challenge))
        .collect::<Result<Vec<_>, _>>()?;
    reader.finish()?;
    let rounds = firsts.into_iter().zip(drawn).zip(responses);
    Ok(rounds
        .map(|((first, challenge), response)| OpenedRound { first, challenge, response })
        .collect())
}

/// Checks a proof: [`Verdict::Malformed`] when [`parse`] fails, [`Verdict::Rejected`] when any
/// round fails a check.
pub fn verify(
    statement: &Statement,
    public: &[Fp],
    proof: &[u8],
    control: &Control,
) -> Result<Verdict, BooError> {
    let rounds = match parse(statement, public, proof) {
        Ok(rounds) => rounds,
        Err(why) => return Ok(Verdict::Malformed(why)),
    };
    for round in &rounds {
        control.checkpoint().map_err(|_| BooError::Cancelled)?;
        let checks = check_round(statement, public, &round.first, round.challenge, &round.response);
        if !checks.iter().all(|check| check.passed) {
            return Ok(Verdict::Rejected);
        }
    }
    Ok(Verdict::Accepted)
}
