//! The simulator: told the cards in advance, it produces rounds the verifier accepts without
//! knowing any witness — the edited tape of Ali Baba's cave.
//!
//! For a dealer card it deals honest cards and computes on an all-zero witness; a dealer card
//! never looks at the inputs. For a peek card it opens two friends with fresh seeds (and uniform
//! corrections when P3 is one of them), invents the hidden friend's `d` and `e` uniformly, and
//! sets the hidden friend's assertion shares to whatever makes each sum zero. That is exactly the
//! distribution a real round shows: the hidden friend's messages are masked by card shares the
//! verifier never sees, and its assertion shares are fixed by the other two. Since recordings
//! made this way are indistinguishable from real ones without a witness, a real recording cannot
//! carry the witness either. Only the live draw of the cards, after the commitments, convinces.

use zk_circuit::ZkField;

use crate::cast::Card;
use crate::coins::Coins;
use crate::deal::{Corrections, Tape};
use crate::error::TrioError;
use crate::field::Fp;
use crate::hash::SeedStream;
use crate::mpc::{Absent, execute};
use crate::program::{Claim, Statement};
use crate::prover::RoundProver;
use crate::round::{Commitments, Opening, RoundSecrets, Tweak};

/// One fabricated round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatedRound {
    /// The commitments shown before the card.
    pub commitments: Commitments,
    /// The card it was made for.
    pub card: Card,
    /// The opening shown after.
    pub opening: Opening,
}

/// A prover with no witness who knows which card each round will draw.
#[derive(Debug)]
pub struct Simulator<'s> {
    statement: &'s Statement,
    public: Vec<Fp>,
    plan: Vec<Card>,
    next: usize,
    coins: Coins,
    pending: Option<(Card, Opening)>,
}

impl<'s> Simulator<'s> {
    /// A simulator for `public` that will face the cards in `plan`, in order.
    pub fn new(
        statement: &'s Statement,
        public: Vec<Fp>,
        plan: Vec<Card>,
        coins: Coins,
    ) -> Result<Self, TrioError> {
        statement.check_public(&public)?;
        Ok(Self { statement, public, plan, next: 0, coins, pending: None })
    }

    fn fabricate(&mut self, card: Card) -> Result<RoundSecrets, TrioError> {
        let Some(hidden) = card.hidden() else {
            let zeros = vec![Fp::zero(); self.statement.witness_values()];
            let claim = Claim { public: self.public.clone(), witness: zeros };
            return RoundSecrets::build(self.statement, &claim, &mut self.coins, Tweak::Honest);
        };
        let coins = &mut self.coins;
        let card_seeds = [coins.seed()?, coins.seed()?, coins.seed()?];
        let input_seeds = [coins.seed()?, coins.seed()?, coins.seed()?];
        let cards = (0..self.statement.multiplications()).map(|_| coins.element());
        let cards = cards.collect::<Result<Vec<_>, _>>()?;
        let inputs = (0..self.statement.witness_values()).map(|_| coins.element());
        let corrections = Corrections { cards, inputs: inputs.collect::<Result<Vec<_>, _>>()? };
        let tapes = hidden
            .others()
            .iter()
            .map(|f| {
                let seeds = (&card_seeds[f.index()], &input_seeds[f.index()]);
                Tape::open(self.statement, *f, seeds, Some(&corrections))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|why| TrioError::Circuit(why.to_string()))?;
        let mut invented = Invented { masks: SeedStream::inputs(&coins.seed()?), sent: Vec::new() };
        let execution = execute(self.statement, &self.public, &tapes, Some(&mut invented));
        let mut broadcasts: [Vec<Fp>; 3] = Default::default();
        for (friend, sent) in hidden.others().into_iter().zip(execution.broadcasts) {
            broadcasts[friend.index()] = sent;
        }
        broadcasts[hidden.index()] = invented.sent;
        Ok(RoundSecrets::commit(card_seeds, input_seeds, corrections, broadcasts))
    }
}

/// The hidden friend's messages, made up as the two opened friends need them.
struct Invented {
    masks: SeedStream,
    sent: Vec<Fp>,
}

impl Absent for Invented {
    fn masked(&mut self) -> Fp {
        let value = self.masks.next_element();
        self.sent.push(value);
        value
    }

    fn assertion(&mut self, present: Fp) -> Fp {
        let value = present.neg();
        self.sent.push(value);
        value
    }
}

impl RoundProver for Simulator<'_> {
    fn commit(&mut self) -> Result<Commitments, TrioError> {
        let card = *self.plan.get(self.next).ok_or(TrioError::PlanExhausted)?;
        self.next += 1;
        let secrets = self.fabricate(card)?;
        let commitments = secrets.commitments;
        self.pending = Some((card, secrets.open(card)));
        Ok(commitments)
    }

    fn respond(&mut self, drawn: Card) -> Result<Opening, TrioError> {
        let (planned, opening) = self.pending.take().ok_or(TrioError::NothingCommitted)?;
        if planned != drawn {
            return Err(TrioError::UnplannedCard { planned, drawn });
        }
        Ok(opening)
    }
}

/// Fabricates one round per card in `plan`, without a witness.
pub fn simulate(
    statement: &Statement,
    public: Vec<Fp>,
    plan: &[Card],
    coins: Coins,
) -> Result<Vec<SimulatedRound>, TrioError> {
    let mut simulator = Simulator::new(statement, public, plan.to_vec(), coins)?;
    plan.iter()
        .map(|card| {
            let commitments = simulator.commit()?;
            let opening = simulator.respond(*card)?;
            Ok(SimulatedRound { commitments, card: *card, opening })
        })
        .collect()
}
