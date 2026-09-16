//! The Fiat–Shamir challenge: the cards of every round, drawn from a hash of the statement and of
//! every commitment, so the prover cannot choose them.
//!
//! `seed = SHA-256("zk/trio/v1/challenge" ‖ u32 id length ‖ example id ‖ circuit digest ‖
//! u32 public inputs ‖ element… ‖ u32 rounds ‖ (K_1 ‖ K_2 ‖ K_3 ‖ V_1 ‖ V_2 ‖ V_3)…)`, integers
//! little-endian, elements 8 canonical little-endian bytes. The digits are read from
//! `SHA-256("zk/trio/v1/digits" ‖ seed ‖ u64 block)` for blocks 0, 1, …: each byte below 250 gives
//! the digit `byte mod 5`, larger bytes are skipped, until every round has a digit
//! ([`Card::from_digit`]).

use sha2::{Digest, Sha256};

use crate::cast::Card;
use crate::field::Fp;
use crate::hash::{Bytes32, DOMAIN_CHALLENGE, DOMAIN_DIGITS, update_elements};
use crate::program::Statement;
use crate::round::Commitments;

/// The seed every card is drawn from.
pub(crate) fn challenge_seed(
    statement: &Statement,
    public: &[Fp],
    commitments: &[Commitments],
) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_CHALLENGE);
    hasher.update((statement.id().len() as u32).to_le_bytes());
    hasher.update(statement.id().as_bytes());
    hasher.update(statement.digest());
    hasher.update((public.len() as u32).to_le_bytes());
    update_elements(&mut hasher, public);
    hasher.update((commitments.len() as u32).to_le_bytes());
    for round in commitments {
        for digest in round.cards.iter().chain(&round.views) {
            hasher.update(digest);
        }
    }
    hasher.finalize().into()
}

/// One card per round, from the challenge seed.
pub(crate) fn challenge_cards(
    statement: &Statement,
    public: &[Fp],
    commitments: &[Commitments],
) -> Vec<Card> {
    let seed = challenge_seed(statement, public, commitments);
    let mut cards = Vec::with_capacity(commitments.len());
    let mut block = 0u64;
    while cards.len() < commitments.len() {
        let bytes = Sha256::new_with_prefix(DOMAIN_DIGITS)
            .chain_update(seed)
            .chain_update(block.to_le_bytes())
            .finalize();
        block += 1;
        let missing = commitments.len() - cards.len();
        let digits = bytes.iter().filter(|byte| **byte < 250).map(|byte| byte % 5);
        cards.extend(digits.filter_map(Card::from_digit).take(missing));
    }
    cards
}
