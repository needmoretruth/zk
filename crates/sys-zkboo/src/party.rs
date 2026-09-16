//! The three parties the prover plays, and the order that makes them neighbours.

/// One of the three parties of the decomposition.
///
/// Party `i` mixes its shares with those of [`Party::next`] at every multiplication, so the parties
/// sit in a circle P1 → P2 → P3 → P1. P1 alone adds constants and public inputs; P3 alone holds
/// input shares that no seed produces, which is why its view carries them.
///
/// A challenge is a party too: challenge `e` opens views `e` and `e.next()` and recomputes `e`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Party {
    /// The first party.
    P1,
    /// The second party.
    P2,
    /// The third party, whose input shares are fixed by the witness.
    P3,
}

impl Party {
    /// All three, in order.
    pub const ALL: [Party; 3] = [Self::P1, Self::P2, Self::P3];

    /// Position 0, 1 or 2, for arrays indexed by party.
    pub fn index(self) -> usize {
        match self {
            Self::P1 => 0,
            Self::P2 => 1,
            Self::P3 => 2,
        }
    }

    /// 1, 2 or 3, as the paper and a screen number the parties.
    pub fn number(self) -> u8 {
        match self {
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
        }
    }

    /// The right-hand neighbour, `i + 1` taken mod 3.
    pub fn next(self) -> Party {
        match self {
            Self::P1 => Self::P2,
            Self::P2 => Self::P3,
            Self::P3 => Self::P1,
        }
    }

    /// The party a challenge leaves unopened: `e + 2` taken mod 3.
    pub fn hidden_by(challenge: Party) -> Party {
        challenge.next().next()
    }
}
