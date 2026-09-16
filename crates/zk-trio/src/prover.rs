//! The prover's side of a live round, as one trait every prover in the exhibit plays through: the
//! honest prover, the cheats and the simulator.

use crate::cast::Card;
use crate::coins::Coins;
use crate::error::TrioError;
use crate::program::{Claim, Statement};
use crate::round::{Commitments, Opening, RoundSecrets, Tweak};

/// Anything that can sit across from the verifier for one round at a time.
pub trait RoundProver {
    /// Commits to a new round, forgetting any round not yet opened.
    fn commit(&mut self) -> Result<Commitments, TrioError>;

    /// Opens the round last committed to, for the card the verifier drew.
    fn respond(&mut self, card: Card) -> Result<Opening, TrioError>;
}

/// The prover who follows the protocol with its witness.
#[derive(Debug)]
pub struct HonestProver<'s> {
    statement: &'s Statement,
    claim: Claim,
    coins: Coins,
    pending: Option<RoundSecrets>,
}

impl<'s> HonestProver<'s> {
    /// A prover for `claim` drawing its seeds from `coins`.
    pub fn new(statement: &'s Statement, claim: Claim, coins: Coins) -> Result<Self, TrioError> {
        statement.check_public(&claim.public)?;
        Ok(Self { statement, claim, coins, pending: None })
    }
}

impl RoundProver for HonestProver<'_> {
    fn commit(&mut self) -> Result<Commitments, TrioError> {
        let secrets =
            RoundSecrets::build(self.statement, &self.claim, &mut self.coins, Tweak::Honest)?;
        let commitments = secrets.commitments;
        self.pending = Some(secrets);
        Ok(commitments)
    }

    fn respond(&mut self, card: Card) -> Result<Opening, TrioError> {
        let secrets = self.pending.take().ok_or(TrioError::NothingCommitted)?;
        Ok(secrets.open(card))
    }
}
