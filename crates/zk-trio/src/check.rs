//! The verifier's side of one round: given the commitments, the card and the opening, which checks
//! pass.

use zk_circuit::ZkField;

use crate::cast::{Card, Friend};
use crate::deal::{Corrections, Tape, draw_cards};
use crate::field::Fp;
use crate::hash::{card_commitment, view_commitment, view_key};
use crate::mpc::{Listed, execute};
use crate::program::Statement;
use crate::round::{Commitments, OpenedSeeds, Opening};

/// What one check looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckKind {
    /// The opening is the kind the card asks for, with every list the length the circuit fixes.
    OpeningFitsCard,
    /// Every dealer card satisfies `c = a·b` (dealer cards only).
    CardsMultiply,
    /// A friend's recomputed `K` equals its commitment.
    CardCommitment(Friend),
    /// A friend's recomputed `V` equals its commitment (peek cards: all three friends).
    ViewCommitment(Friend),
    /// Every assert-zero gate's three shares sum to zero (peek cards only).
    AssertionsVanish,
}

/// One check's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    /// What was checked.
    pub kind: CheckKind,
    /// Whether it held.
    pub passed: bool,
    /// For [`CheckKind::CardsMultiply`] the first bad multiplication, for
    /// [`CheckKind::AssertionsVanish`] the first assertion with a nonzero sum, both counted from 0
    /// in gate order.
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

/// Runs every check one round calls for. `public` must have the circuit's length.
pub fn check_round(
    statement: &Statement,
    public: &[Fp],
    commitments: &Commitments,
    card: Card,
    opening: &Opening,
) -> Vec<Check> {
    match (card.hidden(), opening) {
        (None, Opening::Dealer { card_seeds, card_correction }) => {
            check_dealer(statement, commitments, card_seeds, card_correction)
        }
        (Some(hidden), Opening::Peek { hidden: said, .. }) if hidden == *said => {
            check_peek(statement, public, commitments, opening)
        }
        _ => vec![Check::of(CheckKind::OpeningFitsCard, false)],
    }
}

fn check_dealer(
    statement: &Statement,
    commitments: &Commitments,
    card_seeds: &[[u8; 32]; 3],
    card_correction: &[Fp],
) -> Vec<Check> {
    if card_correction.len() != statement.multiplications() {
        return vec![Check::of(CheckKind::OpeningFitsCard, false)];
    }
    let m = statement.multiplications();
    let shares = Friend::ALL.map(|friend| draw_cards(friend, &card_seeds[friend.index()], m));
    let bad = (0..m).find(|&j| {
        let a = shares[0].a[j].add(shares[1].a[j]).add(shares[2].a[j]);
        let b = shares[0].b[j].add(shares[1].b[j]).add(shares[2].b[j]);
        let c = shares[0].c[j].add(shares[1].c[j]).add(card_correction[j]);
        a.mul(b) != c
    });
    let mut checks = vec![
        Check::of(CheckKind::OpeningFitsCard, true),
        Check::located(CheckKind::CardsMultiply, bad),
    ];
    for friend in Friend::ALL {
        let correction = (friend == Friend::P3).then_some(card_correction);
        let recomputed = card_commitment(&card_seeds[friend.index()], correction);
        let passed = recomputed == commitments.cards[friend.index()];
        checks.push(Check::of(CheckKind::CardCommitment(friend), passed));
    }
    checks
}

fn check_peek(
    statement: &Statement,
    public: &[Fp],
    commitments: &Commitments,
    opening: &Opening,
) -> Vec<Check> {
    let Opening::Peek { hidden, opened, corrections, hidden_view_key, hidden_broadcasts } = opening
    else {
        return vec![Check::of(CheckKind::OpeningFitsCard, false)];
    };
    let Some(tapes) = open_tapes(statement, *hidden, opened, corrections.as_ref()) else {
        return vec![Check::of(CheckKind::OpeningFitsCard, false)];
    };
    if hidden_broadcasts.len() != statement.broadcasts_per_friend() {
        return vec![Check::of(CheckKind::OpeningFitsCard, false)];
    }
    let mut listed = Listed::new(hidden_broadcasts);
    let execution = execute(statement, public, &tapes, Some(&mut listed));
    let mut checks = vec![Check::of(CheckKind::OpeningFitsCard, true)];
    for seeds in opened {
        let friend = seeds.friend;
        let correction = corrections.as_ref().filter(|_| friend == Friend::P3);
        let k = card_commitment(&seeds.card_seed, correction.map(|c| c.cards.as_slice()));
        let passed = k == commitments.cards[friend.index()];
        checks.push(Check::of(CheckKind::CardCommitment(friend), passed));
    }
    for (seeds, sent) in opened.iter().zip(&execution.broadcasts) {
        let friend = seeds.friend;
        let correction = corrections.as_ref().filter(|_| friend == Friend::P3);
        let u = view_key(&seeds.input_seed, correction.map(|c| c.inputs.as_slice()));
        let passed = view_commitment(&u, sent) == commitments.views[friend.index()];
        checks.push(Check::of(CheckKind::ViewCommitment(friend), passed));
    }
    let hidden_view = view_commitment(hidden_view_key, hidden_broadcasts);
    let passed = hidden_view == commitments.views[hidden.index()];
    checks.push(Check::of(CheckKind::ViewCommitment(*hidden), passed));
    checks.push(Check::located(CheckKind::AssertionsVanish, execution.first_nonzero()));
    checks
}

/// The opened friends' tapes, or `None` when the opening names the wrong friends or carries the
/// wrong corrections for them.
fn open_tapes(
    statement: &Statement,
    hidden: Friend,
    opened: &[OpenedSeeds; 2],
    corrections: Option<&Corrections>,
) -> Option<Vec<Tape>> {
    let named = opened.map(|seeds| seeds.friend);
    if named != hidden.others() || corrections.is_some() != (hidden != Friend::P3) {
        return None;
    }
    opened
        .iter()
        .map(|s| Tape::open(statement, s.friend, (&s.card_seed, &s.input_seed), corrections).ok())
        .collect()
}
