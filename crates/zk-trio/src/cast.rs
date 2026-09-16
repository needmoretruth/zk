//! The cast of a round: three friends, and the five cards the verifier draws from.

/// One of the three friends who compute the circuit on shares of the witness.
///
/// P1 alone adds public inputs and constants; P3 alone holds the two corrections (the dealer's
/// last card shares and the last input shares), which is why opening P3 opens both corrections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Friend {
    /// The first friend.
    P1,
    /// The second friend.
    P2,
    /// The third friend, who holds the corrections.
    P3,
}

impl Friend {
    /// All three, in order.
    pub const ALL: [Friend; 3] = [Self::P1, Self::P2, Self::P3];

    /// Position 0, 1 or 2, for arrays indexed by friend.
    pub fn index(self) -> usize {
        match self {
            Self::P1 => 0,
            Self::P2 => 1,
            Self::P3 => 2,
        }
    }

    /// 1, 2 or 3, as a screen names the friend.
    pub fn number(self) -> u8 {
        match self {
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
        }
    }

    /// The two other friends, in order: the ones a peek card hiding this friend opens.
    pub fn others(self) -> [Friend; 2] {
        match self {
            Self::P1 => [Self::P2, Self::P3],
            Self::P2 => [Self::P1, Self::P3],
            Self::P3 => [Self::P1, Self::P2],
        }
    }
}

/// A card the verifier draws, one per round, after the prover has committed.
///
/// Two dealer cards and three peek cards: a rigged multiplication card survives exactly the three
/// peek cards, and a friend who computed wrongly survives the two dealer cards and the one peek
/// card that hides it. Either way a cheat survives three cards of five.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Card {
    /// The first dealer card: open every card seed and check every multiplication card.
    DealerA,
    /// The second dealer card, identical in effect; two copies make the dealer check 2/5 likely.
    DealerB,
    /// Hide this friend, open the other two and re-run their computation.
    Peek(Friend),
}

impl Card {
    /// All five, in digit order.
    pub const ALL: [Card; 5] = [
        Self::DealerA,
        Self::DealerB,
        Self::Peek(Friend::P1),
        Self::Peek(Friend::P2),
        Self::Peek(Friend::P3),
    ];

    /// The base-5 digit that draws this card: 0 and 1 are the dealer cards, 2 + i hides friend i+1.
    pub fn digit(self) -> u8 {
        match self {
            Self::DealerA => 0,
            Self::DealerB => 1,
            Self::Peek(friend) => 2 + friend.index() as u8,
        }
    }

    /// The card a base-5 digit draws, `None` for 5 and above.
    pub fn from_digit(digit: u8) -> Option<Card> {
        Self::ALL.get(usize::from(digit)).copied()
    }

    /// The friend this card hides, `None` for a dealer card.
    pub fn hidden(self) -> Option<Friend> {
        match self {
            Self::Peek(friend) => Some(friend),
            Self::DealerA | Self::DealerB => None,
        }
    }

    /// Whether this is one of the two dealer cards.
    pub fn is_dealer(self) -> bool {
        self.hidden().is_none()
    }
}
