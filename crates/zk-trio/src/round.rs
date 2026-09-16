//! One round from the prover's side: deal, share, compute, commit; then open for the drawn card.

use zk_circuit::ZkField;

use crate::cast::{Card, Friend};
use crate::coins::Coins;
use crate::deal::{
    CardShares, Corrections, Tape, card_correction, draw_cards, draw_inputs, input_correction,
};
use crate::error::TrioError;
use crate::field::Fp;
use crate::hash::{Bytes32, card_commitment, view_commitment, view_key};
use crate::mpc::execute;
use crate::program::{Claim, Statement};

/// The six commitments of one round, sent before the card is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commitments {
    /// `K_1, K_2, K_3`: each friend's card seed (and P3's card correction).
    pub cards: [Bytes32; 3],
    /// `V_1, V_2, V_3`: each friend's view key `u_i` and every message it broadcast.
    pub views: [Bytes32; 3],
}

/// The seeds of a friend a peek card opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenedSeeds {
    /// Which friend.
    pub friend: Friend,
    /// Its `pre` seed, from which its card shares are drawn.
    pub card_seed: Bytes32,
    /// Its `on` seed, from which its input shares are drawn (for P3, salt only).
    pub input_seed: Bytes32,
}

/// What the prover reveals once the card is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opening {
    /// For a dealer card: every card seed and the card correction; nothing about the inputs.
    Dealer {
        /// `pre_1, pre_2, pre_3`.
        card_seeds: [Bytes32; 3],
        /// `c^3` for every multiplication.
        card_correction: Vec<Fp>,
    },
    /// For a peek card: the two other friends in full, and the hidden friend's messages.
    Peek {
        /// The hidden friend.
        hidden: Friend,
        /// The two opened friends, in order.
        opened: [OpenedSeeds; 2],
        /// Both corrections, present exactly when P3 is opened.
        corrections: Option<Corrections>,
        /// The hidden friend's `u`, so its `V` can be recomputed without its seed.
        hidden_view_key: Bytes32,
        /// Every message the hidden friend broadcast.
        hidden_broadcasts: Vec<Fp>,
    },
}

/// How a round departs from the honest protocol; only the cheating provers use anything but
/// `Honest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tweak {
    /// Follow the protocol.
    Honest,
    /// Add `shift` to P3's card correction for one multiplication, so `c ≠ a·b` there.
    RigCard { multiplication: usize, shift: Fp },
    /// Have one friend broadcast assertion shares that make every assertion sum to zero.
    LieAboutAssertions(Friend),
}

/// Everything the prover remembers about a committed round until the card is drawn.
#[derive(Clone, Debug)]
pub(crate) struct RoundSecrets {
    pub(crate) card_seeds: [Bytes32; 3],
    pub(crate) input_seeds: [Bytes32; 3],
    pub(crate) corrections: Corrections,
    pub(crate) view_keys: [Bytes32; 3],
    pub(crate) broadcasts: [Vec<Fp>; 3],
    /// Per assertion, the sum the three friends' honest shares would have had.
    pub(crate) sums: Vec<Fp>,
    pub(crate) commitments: Commitments,
}

impl RoundSecrets {
    /// Deals, shares the claim's witness, computes and commits, departing as `tweak` says.
    pub(crate) fn build(
        statement: &Statement,
        claim: &Claim,
        coins: &mut Coins,
        tweak: Tweak,
    ) -> Result<Self, TrioError> {
        let card_seeds = [coins.seed()?, coins.seed()?, coins.seed()?];
        let input_seeds = [coins.seed()?, coins.seed()?, coins.seed()?];
        let (m, s) = (statement.multiplications(), statement.witness_values());
        let cards: [CardShares; 3] = Friend::ALL.map(|f| draw_cards(f, &card_seeds[f.index()], m));
        let inputs: [Vec<Fp>; 3] = Friend::ALL.map(|f| draw_inputs(f, &input_seeds[f.index()], s));
        let mut corrections = Corrections {
            cards: card_correction(&cards),
            inputs: input_correction(&claim.witness, &inputs[0], &inputs[1]),
        };
        if let Tweak::RigCard { multiplication, shift } = tweak
            && let Some(c) = corrections.cards.get_mut(multiplication)
        {
            *c = c.add(shift);
        }
        let tapes: Vec<Tape> = Friend::ALL
            .into_iter()
            .zip(cards)
            .zip(inputs)
            .map(|((friend, cards), inputs)| Tape::assemble(friend, cards, inputs, &corrections))
            .collect();
        let execution = execute(statement, &claim.public, &tapes, None);
        let mut broadcasts: [Vec<Fp>; 3] = [0, 1, 2].map(|i| execution.broadcasts[i].clone());
        if let Tweak::LieAboutAssertions(liar) = tweak {
            for (slot, sum) in statement.assert_slots.iter().zip(&execution.sums) {
                let share = &mut broadcasts[liar.index()][*slot];
                *share = share.sub(*sum);
            }
        }
        let secrets = Self::commit(card_seeds, input_seeds, corrections, broadcasts);
        Ok(Self { sums: execution.sums, ..secrets })
    }

    /// Computes the commitments over everything decided so far.
    pub(crate) fn commit(
        card_seeds: [Bytes32; 3],
        input_seeds: [Bytes32; 3],
        corrections: Corrections,
        broadcasts: [Vec<Fp>; 3],
    ) -> Self {
        let third = |friend: Friend, correction| (friend == Friend::P3).then_some(correction);
        let view_keys = Friend::ALL.map(|friend| {
            view_key(&input_seeds[friend.index()], third(friend, corrections.inputs.as_slice()))
        });
        let commitments = Commitments {
            cards: Friend::ALL.map(|friend| {
                card_commitment(&card_seeds[friend.index()], third(friend, &corrections.cards))
            }),
            views: Friend::ALL
                .map(|f| view_commitment(&view_keys[f.index()], &broadcasts[f.index()])),
        };
        Self {
            card_seeds,
            input_seeds,
            corrections,
            view_keys,
            broadcasts,
            sums: Vec::new(),
            commitments,
        }
    }

    /// The honest opening for `card`.
    pub(crate) fn open(&self, card: Card) -> Opening {
        let Some(hidden) = card.hidden() else {
            return Opening::Dealer {
                card_seeds: self.card_seeds,
                card_correction: self.corrections.cards.clone(),
            };
        };
        let opened = hidden.others().map(|friend| OpenedSeeds {
            friend,
            card_seed: self.card_seeds[friend.index()],
            input_seed: self.input_seeds[friend.index()],
        });
        Opening::Peek {
            hidden,
            opened,
            corrections: (hidden != Friend::P3).then(|| self.corrections.clone()),
            hidden_view_key: self.view_keys[hidden.index()],
            hidden_broadcasts: self.broadcasts[hidden.index()].clone(),
        }
    }
}
