//! The non-interactive proof: 109 rounds made at once, the cards drawn from a hash of everything
//! committed (Fiat–Shamir).
//!
//! A prover facing a live verifier gets one try per round, so 55 rounds leave a cheat
//! (3/5)^55 < 2^-40. A prover making a proof alone can recompute the hash as often as it likes
//! until the cards suit it, so the proof needs twice the rounds: (3/5)^109 < 2^-80.
//!
//! # Byte format
//!
//! Integers are little-endian, seeds and digests 32 bytes, field elements 8 canonical
//! little-endian bytes. `M` is the circuit's multiplications, `S` its witness values (private
//! inputs and hint outputs) and `A` its assert-zero gates.
//!
//! ```text
//! u32 rounds                                   always 109
//! rounds × (K_1 ‖ K_2 ‖ K_3 ‖ V_1 ‖ V_2 ‖ V_3)
//! rounds × opening, laid out by that round's card:
//!   dealer card        pre_1 ‖ pre_2 ‖ pre_3 ‖ c^3 × M
//!   peek card hiding h for each other friend in order: pre ‖ on
//!                      then, only when P3 is opened:  c^3 × M ‖ w^3 × S
//!                      then u_h ‖ broadcasts of h × (2M + A)
//! ```
//!
//! A friend's broadcasts are, in gate order, `d` then `e` for every multiplication and its share
//! for every assert-zero gate. The proof carries no tags, lengths or cards: the verifier
//! recomputes the cards from the commitments and then knows how to read each opening, and any
//! byte left over or missing makes the proof malformed. A proof is therefore
//! `4 + 109·192 + Σ opening` bytes, where an opening is `96 + 8M` for a dealer card,
//! `160 + 16M + 8A` for a peek card hiding P3 and `160 + 24M + 8S + 8A` for the other two.
//!
//! # Cards
//!
//! `seed = SHA-256("zk/trio/v1/challenge" ‖ u32 id length ‖ example id ‖ circuit digest ‖
//! u32 public inputs ‖ element… ‖ u32 rounds ‖ (K_1 ‖ K_2 ‖ K_3 ‖ V_1 ‖ V_2 ‖ V_3)…)`, with the
//! circuit digest of [`crate::digest`]. Digits are read from `SHA-256("zk/trio/v1/digits" ‖ seed ‖
//! u64 block)` for blocks 0, 1, …: each byte below 250 gives the digit `byte mod 5` and larger
//! bytes are skipped, one digit per round in order. Digits 0 and 1 are the dealer cards, 2, 3 and
//! 4 the peek cards hiding P1, P2 and P3.

use zk_core::{Control, Verdict};

use crate::challenge::challenge_cards;
use crate::check::check_round;
use crate::codec::{
    COMMITMENT_BYTES, Reader, read_commitments, read_opening, write_commitments, write_opening,
};
use crate::coins::Coins;
use crate::error::TrioError;
use crate::field::Fp;
use crate::program::{Claim, Statement};
use crate::round::{RoundSecrets, Tweak};

/// Rounds in a non-interactive proof: (3/5)^109 < 2^-80.
pub const PROOF_ROUNDS: u32 = 109;

/// The first byte of the first round's opening: a card seed of an opened friend whichever card
/// was drawn, which the verifier hashes into a `K` it compares with the commitment.
pub const TAMPER_OFFSET: usize = 4 + PROOF_ROUNDS as usize * COMMITMENT_BYTES;

/// Proves `claim` without checking it; a false claim still yields bytes, which the verifier rejects.
pub fn prove(
    statement: &Statement,
    claim: &Claim,
    coins: &mut Coins,
    control: &Control,
) -> Result<Vec<u8>, TrioError> {
    statement.check_public(&claim.public)?;
    let mut rounds = Vec::with_capacity(PROOF_ROUNDS as usize);
    for _ in 0..PROOF_ROUNDS {
        control.checkpoint().map_err(|_| TrioError::Cancelled)?;
        rounds.push(RoundSecrets::build(statement, claim, coins, Tweak::Honest)?);
    }
    let commitments: Vec<_> = rounds.iter().map(|round| round.commitments).collect();
    let cards = challenge_cards(statement, &claim.public, &commitments);
    let mut proof = Vec::new();
    proof.extend_from_slice(&PROOF_ROUNDS.to_le_bytes());
    for round in &commitments {
        write_commitments(&mut proof, round);
    }
    for (round, card) in rounds.iter().zip(cards) {
        write_opening(&mut proof, &round.open(card));
    }
    Ok(proof)
}

/// Checks a proof. Bytes that do not decode, or a public input list of the wrong length, are
/// [`Verdict::Malformed`]; a failed check is [`Verdict::Rejected`].
pub fn verify(
    statement: &Statement,
    public: &[Fp],
    proof: &[u8],
    control: &Control,
) -> Result<Verdict, TrioError> {
    if let Err(error) = statement.check_public(public) {
        return Ok(Verdict::Malformed(error.to_string()));
    }
    let mut reader = Reader::new(proof);
    let parsed = (|| {
        let rounds = reader.u32()?;
        if rounds != PROOF_ROUNDS {
            return Err(format!("a proof has {PROOF_ROUNDS} rounds, this one claims {rounds}"));
        }
        let commitments =
            (0..rounds).map(|_| read_commitments(&mut reader)).collect::<Result<Vec<_>, _>>()?;
        let cards = challenge_cards(statement, public, &commitments);
        let openings = cards
            .iter()
            .map(|card| read_opening(&mut reader, statement, *card))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((commitments, cards, openings))
    })();
    let (commitments, cards, openings) = match parsed.and_then(|p| reader.finish().map(|()| p)) {
        Ok(parsed) => parsed,
        Err(why) => return Ok(Verdict::Malformed(why)),
    };
    for ((round, card), opening) in commitments.iter().zip(cards).zip(&openings) {
        control.checkpoint().map_err(|_| TrioError::Cancelled)?;
        let checks = check_round(statement, public, round, card, opening);
        if !checks.iter().all(|check| check.passed) {
            return Ok(Verdict::Rejected);
        }
    }
    Ok(Verdict::Accepted)
}
