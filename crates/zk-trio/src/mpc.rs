//! The friends' computation, run for all three (the prover) or for two whose missing partner's
//! messages come from elsewhere (the verifier, the simulator).
//!
//! Every wire is held as three additive shares. Linear gates are computed share by share, P1 alone
//! adding constants and public inputs. A multiplication `z = x·y` uses the next dealer card
//! `(a, b, c)`: each friend broadcasts `d^i = x^i − a^i` and `e^i = y^i − b^i`, everyone learns
//! `d = Σd^i` and `e = Σe^i`, and each sets `z^i = c^i + d·b^i + e·a^i`, P1 adding `d·e`. The
//! shares sum to `c + d·b + e·a + d·e = x·y` exactly when `c = a·b`. An assert-zero gate has every
//! friend broadcast its share; the shares must sum to zero.
//!
//! A friend's broadcasts are, in gate order, `d` then `e` for every multiplication and its share
//! for every assert-zero gate.

use zk_circuit::ZkField;

use crate::cast::Friend;
use crate::deal::Tape;
use crate::field::Fp;
use crate::program::{Op, Statement};

/// Where the absent friend's messages come from when only two friends compute.
pub(crate) trait Absent {
    /// The absent friend's next `d` or `e`.
    fn masked(&mut self) -> Fp;
    /// The absent friend's share of the next assertion, given the present friends' shares' sum.
    fn assertion(&mut self, present: Fp) -> Fp;
}

/// The absent friend's broadcasts as a list, read in order: what the verifier was sent.
pub(crate) struct Listed<'a> {
    values: &'a [Fp],
    position: usize,
}

impl<'a> Listed<'a> {
    /// Reads `values` from the start; the caller has checked the length.
    pub(crate) fn new(values: &'a [Fp]) -> Self {
        Self { values, position: 0 }
    }

    fn next_value(&mut self) -> Fp {
        let value = self.values.get(self.position).copied().unwrap_or_else(Fp::zero);
        self.position += 1;
        value
    }
}

impl Absent for Listed<'_> {
    fn masked(&mut self) -> Fp {
        self.next_value()
    }

    fn assertion(&mut self, _present: Fp) -> Fp {
        self.next_value()
    }
}

/// What a computation produced.
#[derive(Clone, Debug)]
pub(crate) struct Execution {
    /// Each present friend's broadcasts, in the order of the tapes given.
    pub(crate) broadcasts: Vec<Vec<Fp>>,
    /// Per assert-zero gate, the sum of every friend's share, the absent one's included.
    pub(crate) sums: Vec<Fp>,
}

impl Execution {
    /// The first assert-zero gate whose shares do not sum to zero.
    pub(crate) fn first_nonzero(&self) -> Option<usize> {
        self.sums.iter().position(|sum| *sum != Fp::zero())
    }
}

/// Runs the circuit for `tapes`; `absent` supplies the missing friend's messages when fewer than
/// three friends are present. The caller has checked the public input count.
pub(crate) fn execute(
    statement: &Statement,
    public: &[Fp],
    tapes: &[Tape],
    mut absent: Option<&mut dyn Absent>,
) -> Execution {
    let wires = statement.circuit().num_wires();
    let mut shares = vec![vec![Fp::zero(); wires]; tapes.len()];
    let mut broadcasts = vec![Vec::with_capacity(statement.broadcasts_per_friend()); tapes.len()];
    let mut sums = Vec::with_capacity(statement.assertions());
    let mut card = 0;
    for op in &statement.ops {
        match op {
            Op::Constant { out, value } => set_first(&mut shares, tapes, *out, *value),
            Op::Public { out, index } => {
                let value = public.get(*index).copied().unwrap_or_else(Fp::zero);
                set_first(&mut shares, tapes, *out, value);
            }
            Op::Secret { out, index } => {
                for (held, tape) in shares.iter_mut().zip(tapes) {
                    held[*out] = tape.inputs.get(*index).copied().unwrap_or_else(Fp::zero);
                }
            }
            Op::Linear { out, lc } => {
                for (held, tape) in shares.iter_mut().zip(tapes) {
                    held[*out] = with_constant(lc.terms_at(held), lc.constant, tape.friend);
                }
            }
            Op::Mul { out, left, right } => {
                let lcs = (left, right);
                multiply(&mut shares, &mut broadcasts, tapes, &mut absent, (*out, card), lcs);
                card += 1;
            }
            Op::Assert { lc } => {
                let mut present = Fp::zero();
                for ((held, tape), sent) in shares.iter().zip(tapes).zip(&mut broadcasts) {
                    let share = with_constant(lc.terms_at(held), lc.constant, tape.friend);
                    sent.push(share);
                    present = present.add(share);
                }
                let missing = absent.as_mut().map_or_else(Fp::zero, |a| a.assertion(present));
                sums.push(present.add(missing));
            }
        }
    }
    Execution { broadcasts, sums }
}

fn with_constant(terms: Fp, constant: Fp, friend: Friend) -> Fp {
    if friend == Friend::P1 { terms.add(constant) } else { terms }
}

fn set_first(shares: &mut [Vec<Fp>], tapes: &[Tape], out: usize, value: Fp) {
    for (held, tape) in shares.iter_mut().zip(tapes) {
        held[out] = if tape.friend == Friend::P1 { value } else { Fp::zero() };
    }
}

fn multiply(
    shares: &mut [Vec<Fp>],
    broadcasts: &mut [Vec<Fp>],
    tapes: &[Tape],
    absent: &mut Option<&mut dyn Absent>,
    (out, card): (usize, usize),
    (left, right): (&crate::program::Lc, &crate::program::Lc),
) {
    let (mut d, mut e) = (Fp::zero(), Fp::zero());
    for ((held, tape), sent) in shares.iter().zip(tapes).zip(broadcasts.iter_mut()) {
        let x = with_constant(left.terms_at(held), left.constant, tape.friend);
        let y = with_constant(right.terms_at(held), right.constant, tape.friend);
        let (di, ei) = (x.sub(tape.cards.a[card]), y.sub(tape.cards.b[card]));
        sent.push(di);
        sent.push(ei);
        d = d.add(di);
        e = e.add(ei);
    }
    if let Some(absent) = absent.as_mut() {
        d = d.add(absent.masked());
        e = e.add(absent.masked());
    }
    for (held, tape) in shares.iter_mut().zip(tapes) {
        let (a, b, c) = (tape.cards.a[card], tape.cards.b[card], tape.cards.c[card]);
        let z = c.add(d.mul(b)).add(e.mul(a));
        held[out] = if tape.friend == Friend::P1 { z.add(d.mul(e)) } else { z };
    }
}
