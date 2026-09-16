//! How the bad-card cheat picks its lie: a false claim whose failing arithmetic one rigged
//! multiplication card puts right.
//!
//! A rigged card `c = a·b + shift` makes the friends compute `x·y + shift` for that one
//! multiplication. That repairs a false claim only when every failing assertion depends on that
//! product linearly and one shift zeroes them all, so the search evaluates the claim in the clear,
//! takes the first failing assertion, follows its linear gates back to the multiplications that
//! feed it, and tries the one shift each of those would need, latest multiplication first.

use std::collections::BTreeMap;

use zk_circuit::ZkField;

use crate::field::Fp;
use crate::program::{Claim, Op, Statement};

/// The one multiplication card the cheat rigs, and by how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RiggedCard {
    /// Which multiplication, counted from 0 in gate order.
    pub multiplication: usize,
    /// What the dealer adds to `c`, so `c = a·b + shift`.
    pub shift: Fp,
}

/// A rigged card that makes every assertion of `claim` hold, if one exists.
pub(crate) fn rig(statement: &Statement, claim: &Claim) -> Option<RiggedCard> {
    let base = assertion_values(statement, claim, None);
    let failing = base.iter().position(|value| *value != Fp::zero())?;
    let mut sources = linear_sources(statement, failing);
    sources.sort_by_key(|(multiplication, _)| core::cmp::Reverse(*multiplication));
    sources.into_iter().find_map(|(multiplication, coefficient)| {
        let shift = base[failing].neg().mul(coefficient.inverse()?);
        let rigged = RiggedCard { multiplication, shift };
        let values = assertion_values(statement, claim, Some(rigged));
        values.iter().all(|value| *value == Fp::zero()).then_some(rigged)
    })
}

/// Every assertion's value when the circuit is evaluated in the clear, one product shifted.
fn assertion_values(statement: &Statement, claim: &Claim, rigged: Option<RiggedCard>) -> Vec<Fp> {
    let mut values = vec![Fp::zero(); statement.circuit().num_wires()];
    let mut assertions = Vec::with_capacity(statement.assertions());
    let mut multiplication = 0;
    for op in &statement.ops {
        match op {
            Op::Constant { out, value } => values[*out] = *value,
            Op::Public { out, index } => {
                values[*out] = claim.public.get(*index).copied().unwrap_or_else(Fp::zero);
            }
            Op::Secret { out, index } => {
                values[*out] = claim.witness.get(*index).copied().unwrap_or_else(Fp::zero);
            }
            Op::Linear { out, lc } => values[*out] = lc.terms_at(&values).add(lc.constant),
            Op::Mul { out, left, right } => {
                let x = left.terms_at(&values).add(left.constant);
                let y = right.terms_at(&values).add(right.constant);
                let shift = rigged
                    .filter(|rigged| rigged.multiplication == multiplication)
                    .map_or_else(Fp::zero, |rigged| rigged.shift);
                values[*out] = x.mul(y).add(shift);
                multiplication += 1;
            }
            Op::Assert { lc } => assertions.push(lc.terms_at(&values).add(lc.constant)),
        }
    }
    assertions
}

/// The multiplications whose outputs reach assertion `index` through linear gates only, each with
/// the coefficient it ends up with.
fn linear_sources(statement: &Statement, index: usize) -> Vec<(usize, Fp)> {
    let mut linear = BTreeMap::new();
    let mut products = BTreeMap::new();
    let mut assertion = None;
    let (mut multiplication, mut seen) = (0, 0);
    for op in &statement.ops {
        match op {
            Op::Linear { out, lc } => {
                linear.insert(*out, lc);
            }
            Op::Mul { out, .. } => {
                products.insert(*out, multiplication);
                multiplication += 1;
            }
            Op::Assert { lc } => {
                if seen == index {
                    assertion = Some(lc);
                }
                seen += 1;
            }
            Op::Constant { .. } | Op::Public { .. } | Op::Secret { .. } => {}
        }
    }
    let Some(assertion) = assertion else { return Vec::new() };
    let mut pending: BTreeMap<usize, Fp> = BTreeMap::new();
    add_terms(&mut pending, &assertion.terms, Fp::one());
    let mut sources = Vec::new();
    while let Some((wire, coefficient)) = pending.pop_last() {
        if let Some(lc) = linear.get(&wire) {
            add_terms(&mut pending, &lc.terms, coefficient);
        } else if let Some(multiplication) = products.get(&wire)
            && coefficient != Fp::zero()
        {
            sources.push((*multiplication, coefficient));
        }
    }
    sources
}

fn add_terms(pending: &mut BTreeMap<usize, Fp>, terms: &[(usize, Fp)], scale: Fp) {
    for (wire, coefficient) in terms {
        let entry = pending.entry(*wire).or_insert_with(Fp::zero);
        *entry = entry.add(coefficient.mul(scale));
    }
}
