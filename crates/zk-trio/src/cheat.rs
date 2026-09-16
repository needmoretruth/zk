//! Provers who lie, to show how often a lie survives: never more than three cards in five.

use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, InstanceKind};

use crate::cast::{Card, Friend};
use crate::coins::Coins;
use crate::error::TrioError;
use crate::field::Fp;
use crate::program::{Claim, Statement};
use crate::prover::RoundProver;
use crate::rig::{RiggedCard, rig};
use crate::round::{Commitments, Opening, RoundSecrets, Tweak};
use crate::session::{SessionSummary, play};

/// A way to make a false claim look true.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cheat {
    /// The dealer rigs one multiplication card (`c ≠ a·b`) so a false claim's arithmetic comes
    /// out right. Only the two dealer cards look at the cards: it survives three cards in five.
    BadCard,
    /// This friend broadcasts shares that make every failing assertion sum to zero, which is not
    /// what its own computation gives. Caught whenever it is opened: it survives the two dealer
    /// cards and the one peek card that hides it, three cards in five.
    BadComputation(Friend),
    /// The forgery that broke Trio's first draft: compute honestly with a false witness, and
    /// after seeing a peek card rewrite the hidden friend's assertion shares so the sums vanish.
    /// The draft could not check those shares against a commitment; this design recomputes the
    /// hidden friend's `V` from them, so the forgery now survives only the two dealer cards.
    RewriteHiddenShares,
}

/// Where a cheat's false claim came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClaimSource {
    /// The example's own false claim (a wrong PIN, a 17-year-old, …).
    ExampleFalseClaim,
    /// The honest witness offered for a statement whose public input at this index is one larger.
    BumpedPublicInput(usize),
}

/// The lie a cheating prover tells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FalseClaim {
    /// Where it came from.
    pub source: ClaimSource,
    /// The public inputs the verifier is asked to accept.
    pub public: Vec<Fp>,
    /// For [`Cheat::BadCard`], the card rigged to make it pass.
    pub rigged: Option<RiggedCard>,
}

/// A prover telling one [`Cheat`], round after round.
#[derive(Debug)]
pub struct CheatingProver<'s> {
    statement: &'s Statement,
    cheat: Cheat,
    claim: Claim,
    false_claim: FalseClaim,
    coins: Coins,
    pending: Option<RoundSecrets>,
}

impl<'s> CheatingProver<'s> {
    /// A cheat on `example`'s statement; the false claim's salts come from `coins`.
    pub fn new(
        statement: &'s Statement,
        example: ExampleId,
        cheat: Cheat,
        mut coins: Coins,
    ) -> Result<Self, TrioError> {
        let seed = coins.seed()?;
        let dishonest = statement.claim(&example.instance::<Fp>(InstanceKind::Dishonest, &seed))?;
        let (claim, false_claim) = if cheat == Cheat::BadCard {
            riggable_claim(statement, example, &seed, dishonest)?
        } else {
            let public = dishonest.public.clone();
            (dishonest, FalseClaim { source: ClaimSource::ExampleFalseClaim, public, rigged: None })
        };
        Ok(Self { statement, cheat, claim, false_claim, coins, pending: None })
    }

    /// The lie being told; its public inputs are what the verifier must be given.
    pub fn false_claim(&self) -> &FalseClaim {
        &self.false_claim
    }
}

/// The example's false claim if one card can rig it, else the honest witness against each public
/// input bumped by one in turn.
fn riggable_claim(
    statement: &Statement,
    example: ExampleId,
    seed: &[u8; 32],
    dishonest: Claim,
) -> Result<(Claim, FalseClaim), TrioError> {
    if let Some(rigged) = rig(statement, &dishonest) {
        let public = dishonest.public.clone();
        let source = ClaimSource::ExampleFalseClaim;
        return Ok((dishonest, FalseClaim { source, public, rigged: Some(rigged) }));
    }
    let honest = example.instance::<Fp>(InstanceKind::Honest, seed);
    for index in 0..honest.public.len() {
        let mut bumped = honest.clone();
        bumped.public[index] = bumped.public[index].add(Fp::one());
        let claim = statement.claim(&bumped)?;
        if let Some(rigged) = rig(statement, &claim) {
            let source = ClaimSource::BumpedPublicInput(index);
            let public = claim.public.clone();
            return Ok((claim, FalseClaim { source, public, rigged: Some(rigged) }));
        }
    }
    Err(TrioError::NoRiggableClaim)
}

impl RoundProver for CheatingProver<'_> {
    fn commit(&mut self) -> Result<Commitments, TrioError> {
        let tweak = match (self.cheat, self.false_claim.rigged) {
            (Cheat::BadCard, Some(RiggedCard { multiplication, shift })) => {
                Tweak::RigCard { multiplication, shift }
            }
            (Cheat::BadComputation(friend), _) => Tweak::LieAboutAssertions(friend),
            _ => Tweak::Honest,
        };
        let secrets = RoundSecrets::build(self.statement, &self.claim, &mut self.coins, tweak)?;
        let commitments = secrets.commitments;
        self.pending = Some(secrets);
        Ok(commitments)
    }

    fn respond(&mut self, card: Card) -> Result<Opening, TrioError> {
        let secrets = self.pending.take().ok_or(TrioError::NothingCommitted)?;
        let mut opening = secrets.open(card);
        if self.cheat == Cheat::RewriteHiddenShares
            && let Opening::Peek { hidden_broadcasts, .. } = &mut opening
        {
            for (slot, sum) in self.statement.assert_slots.iter().zip(&secrets.sums) {
                if let Some(share) = hidden_broadcasts.get_mut(*slot) {
                    *share = share.sub(*sum);
                }
            }
        }
        Ok(opening)
    }
}

/// A cheat's conversation: the lie it told and how the verifier took it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheatRun {
    /// The false claim.
    pub claim: FalseClaim,
    /// The conversation; `caught_at` is the round that exposed the cheat, `None` if it escaped.
    pub summary: SessionSummary,
}

/// Plays `rounds` rounds of `cheat` on `example`, stopping at the round that catches it.
pub fn play_cheat(
    example: ExampleId,
    cheat: Cheat,
    rounds: u32,
    prover_coins: Coins,
    cards: &mut Coins,
    control: &Control,
) -> Result<CheatRun, TrioError> {
    let statement = Statement::new(example)?;
    let mut prover = CheatingProver::new(&statement, example, cheat, prover_coins)?;
    let claim = prover.false_claim.clone();
    let summary = play(&statement, claim.public.clone(), &mut prover, rounds, cards, control)?;
    Ok(CheatRun { claim, summary })
}

/// The chance a cheat survives `rounds` rounds: (3/5)^rounds.
pub fn escape_probability(rounds: u32) -> f64 {
    0.6f64.powi(i32::try_from(rounds).unwrap_or(i32::MAX))
}

/// (3/5)^rounds as an exact fraction `(3^rounds, 5^rounds)`, while it fits in 128 bits (up to
/// 55 rounds, exactly the interactive default).
pub fn escape_odds(rounds: u32) -> Option<(u128, u128)> {
    Some((3u128.checked_pow(rounds)?, 5u128.checked_pow(rounds)?))
}
