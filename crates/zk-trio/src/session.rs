//! A live conversation, one round at a time, so a screen can show every card, every opened piece
//! and every check as it happens.

use zk_core::{Control, Stage, Verdict};

use crate::cast::{Card, Friend};
use crate::check::{Check, check_round};
use crate::coins::Coins;
use crate::error::TrioError;
use crate::field::Fp;
use crate::program::Statement;
use crate::prover::RoundProver;
use crate::round::{Commitments, Opening};

/// Rounds in a live conversation: (3/5)^55 < 2^-40.
pub const INTERACTIVE_ROUNDS: u32 = 55;

/// One thing the prover revealed in a round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Opened {
    /// A friend's `pre` seed.
    CardSeed(Friend),
    /// A friend's `on` seed.
    InputSeed(Friend),
    /// The dealer's correction P3 holds.
    CardCorrection,
    /// The input correction P3 holds.
    InputCorrection,
    /// The hidden friend's `u`.
    ViewKey(Friend),
    /// Every message the hidden friend broadcast.
    Broadcasts(Friend),
}

/// Everything a camera would have seen in one round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundEvent {
    /// Round number, from 1.
    pub round: u32,
    /// Rounds planned.
    pub of: u32,
    /// What the prover committed to before the card was drawn.
    pub commitments: Commitments,
    /// The card drawn.
    pub card: Card,
    /// The friend the card hid, if it was a peek card.
    pub hidden: Option<Friend>,
    /// What the prover revealed, in the order it was sent.
    pub opened: Vec<Opened>,
    /// Every check, in the order the verifier ran them.
    pub checks: Vec<Check>,
    /// Whether every check held.
    pub passed: bool,
}

/// What a whole conversation ended with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSummary {
    /// Accepted after every round passed, rejected at the first failed round.
    pub verdict: Verdict,
    /// The round that failed, if one did.
    pub caught_at: Option<u32>,
    /// Every round played, in order.
    pub events: Vec<RoundEvent>,
}

/// The verifier's side of a live conversation.
#[derive(Debug)]
pub struct Session<'s> {
    statement: &'s Statement,
    public: Vec<Fp>,
    rounds: u32,
    played: u32,
    caught_at: Option<u32>,
}

impl<'s> Session<'s> {
    /// A conversation of `rounds` rounds about `public`.
    pub fn new(statement: &'s Statement, public: Vec<Fp>, rounds: u32) -> Result<Self, TrioError> {
        statement.check_public(&public)?;
        Ok(Self { statement, public, rounds, played: 0, caught_at: None })
    }

    /// Whether the verifier has decided: a round failed, or every round passed.
    pub fn is_over(&self) -> bool {
        self.caught_at.is_some() || self.played >= self.rounds
    }

    /// The decision so far: `None` while rounds remain and none failed.
    pub fn verdict(&self) -> Option<Verdict> {
        match (self.caught_at, self.played >= self.rounds) {
            (Some(_), _) => Some(Verdict::Rejected),
            (None, true) => Some(Verdict::Accepted),
            (None, false) => None,
        }
    }

    /// Rounds played so far.
    pub fn played(&self) -> u32 {
        self.played
    }

    /// Rounds planned.
    pub fn rounds(&self) -> u32 {
        self.rounds
    }

    /// The round that failed, if one did.
    pub fn caught_at(&self) -> Option<u32> {
        self.caught_at
    }

    /// Plays the next round with a card drawn from `cards` after the prover commits.
    pub fn play_round(
        &mut self,
        prover: &mut dyn RoundProver,
        cards: &mut Coins,
    ) -> Result<RoundEvent, TrioError> {
        self.refuse_when_over()?;
        let commitments = prover.commit()?;
        let card = cards.card()?;
        self.finish_round(prover, commitments, card)
    }

    /// Plays the next round with a card fixed in advance, as the simulator's editor does; the
    /// prover still commits before it is shown the card.
    pub fn play_round_with(
        &mut self,
        prover: &mut dyn RoundProver,
        card: Card,
    ) -> Result<RoundEvent, TrioError> {
        self.refuse_when_over()?;
        let commitments = prover.commit()?;
        self.finish_round(prover, commitments, card)
    }

    fn refuse_when_over(&self) -> Result<(), TrioError> {
        if self.is_over() { Err(TrioError::SessionOver) } else { Ok(()) }
    }

    fn finish_round(
        &mut self,
        prover: &mut dyn RoundProver,
        commitments: Commitments,
        card: Card,
    ) -> Result<RoundEvent, TrioError> {
        let opening = prover.respond(card)?;
        let checks = check_round(self.statement, &self.public, &commitments, card, &opening);
        let passed = checks.iter().all(|check| check.passed);
        self.played += 1;
        if !passed && self.caught_at.is_none() {
            self.caught_at = Some(self.played);
        }
        Ok(RoundEvent {
            round: self.played,
            of: self.rounds,
            commitments,
            card,
            hidden: card.hidden(),
            opened: opened_items(&opening),
            checks,
            passed,
        })
    }
}

/// Plays a whole conversation, stopping at the first failed round.
pub fn play(
    statement: &Statement,
    public: Vec<Fp>,
    prover: &mut dyn RoundProver,
    rounds: u32,
    cards: &mut Coins,
    control: &Control,
) -> Result<SessionSummary, TrioError> {
    let mut session = Session::new(statement, public, rounds)?;
    let mut events = Vec::new();
    while !session.is_over() {
        control.checkpoint().map_err(|_| TrioError::Cancelled)?;
        control.report(Stage::Round { round: session.played + 1, of: rounds });
        events.push(session.play_round(prover, cards)?);
    }
    let verdict = session.verdict().unwrap_or(Verdict::Rejected);
    Ok(SessionSummary { verdict, caught_at: session.caught_at, events })
}

/// What an opening reveals, item by item.
pub fn opened_items(opening: &Opening) -> Vec<Opened> {
    match opening {
        Opening::Dealer { .. } => {
            let mut items: Vec<Opened> = Friend::ALL.map(Opened::CardSeed).to_vec();
            items.push(Opened::CardCorrection);
            items
        }
        Opening::Peek { hidden, opened, corrections, .. } => {
            let mut items = Vec::new();
            for seeds in opened {
                items.push(Opened::CardSeed(seeds.friend));
                items.push(Opened::InputSeed(seeds.friend));
            }
            if corrections.is_some() {
                items.extend([Opened::CardCorrection, Opened::InputCorrection]);
            }
            items.extend([Opened::ViewKey(*hidden), Opened::Broadcasts(*hidden)]);
            items
        }
    }
}
