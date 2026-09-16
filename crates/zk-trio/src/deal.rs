//! What each friend holds before computing: its share of every dealer card and of every witness
//! value, all drawn from its two seeds except P3's, which are the two corrections.

use zk_circuit::ZkField;

use crate::cast::Friend;
use crate::field::Fp;
use crate::hash::{Bytes32, SeedStream};
use crate::program::Statement;

/// The dealer's and the witness's last shares, which only P3 holds and which no seed can produce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corrections {
    /// `c^3` per multiplication: `a·b − c^1 − c^2`.
    pub cards: Vec<Fp>,
    /// `w^3` per witness value: `w − w^1 − w^2`.
    pub inputs: Vec<Fp>,
}

/// Card shares drawn from one card seed.
#[derive(Clone, Debug)]
pub(crate) struct CardShares {
    pub(crate) a: Vec<Fp>,
    pub(crate) b: Vec<Fp>,
    /// Empty for P3, whose `c` is the card correction.
    pub(crate) c: Vec<Fp>,
}

/// One friend's tape: everything it computes from, apart from other friends' broadcasts.
#[derive(Clone, Debug)]
pub(crate) struct Tape {
    pub(crate) friend: Friend,
    pub(crate) cards: CardShares,
    pub(crate) inputs: Vec<Fp>,
}

/// The card tape of a seed: for each multiplication in order `a`, `b`, and `c` unless P3.
pub(crate) fn draw_cards(friend: Friend, seed: &Bytes32, multiplications: usize) -> CardShares {
    let mut stream = SeedStream::cards(seed);
    let mut shares = CardShares {
        a: Vec::with_capacity(multiplications),
        b: Vec::with_capacity(multiplications),
        c: Vec::with_capacity(multiplications),
    };
    for _ in 0..multiplications {
        shares.a.push(stream.next_element());
        shares.b.push(stream.next_element());
        if friend != Friend::P3 {
            shares.c.push(stream.next_element());
        }
    }
    shares
}

/// The input tape of a seed: one share per witness value, none for P3 (its seed is only salt).
pub(crate) fn draw_inputs(friend: Friend, seed: &Bytes32, witness_values: usize) -> Vec<Fp> {
    if friend == Friend::P3 { Vec::new() } else { SeedStream::inputs(seed).take(witness_values) }
}

/// The dealer's correction: `c^3 = (a^1 + a^2 + a^3)·(b^1 + b^2 + b^3) − c^1 − c^2`.
pub(crate) fn card_correction(shares: &[CardShares; 3]) -> Vec<Fp> {
    let [first, second, third] = shares;
    (0..third.a.len())
        .map(|j| {
            let a = first.a[j].add(second.a[j]).add(third.a[j]);
            let b = first.b[j].add(second.b[j]).add(third.b[j]);
            a.mul(b).sub(first.c[j]).sub(second.c[j])
        })
        .collect()
}

/// The input correction: `w^3 = w − w^1 − w^2`.
pub(crate) fn input_correction(witness: &[Fp], first: &[Fp], second: &[Fp]) -> Vec<Fp> {
    witness.iter().zip(first).zip(second).map(|((w, w1), w2)| w.sub(*w1).sub(*w2)).collect()
}

impl Tape {
    /// A tape from shares already drawn; P3 takes its `c` and inputs from the corrections.
    pub(crate) fn assemble(
        friend: Friend,
        mut cards: CardShares,
        inputs: Vec<Fp>,
        corrections: &Corrections,
    ) -> Self {
        if friend == Friend::P3 {
            cards.c = corrections.cards.clone();
            return Self { friend, cards, inputs: corrections.inputs.clone() };
        }
        Self { friend, cards, inputs }
    }

    /// Re-derives an opened friend's tape from its seeds, as the verifier does. P3 needs both
    /// corrections, each of the length the circuit fixes.
    pub(crate) fn open(
        statement: &Statement,
        friend: Friend,
        seeds: (&Bytes32, &Bytes32),
        corrections: Option<&Corrections>,
    ) -> Result<Self, &'static str> {
        let cards = draw_cards(friend, seeds.0, statement.multiplications());
        if friend != Friend::P3 {
            let inputs = draw_inputs(friend, seeds.1, statement.witness_values());
            return Ok(Self { friend, cards, inputs });
        }
        let corrections = corrections.ok_or("P3 was opened without its corrections")?;
        if corrections.cards.len() != statement.multiplications()
            || corrections.inputs.len() != statement.witness_values()
        {
            return Err("a correction has the wrong length for this circuit");
        }
        Ok(Self::assemble(friend, cards, Vec::new(), corrections))
    }
}
