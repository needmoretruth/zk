//! The verifier (§4.7 and §5.2): recompute both challenges, then check the responses against the
//! opened columns.
//!
//! For every repetition `h` and every opened column `j` at the point `η_j`:
//!
//! - **proximity**: `Σ_i r_i·U[i][j] + code blind[j] = v(η_j)`, where `v` has degree below `k` by
//!   construction, since the proof sends `k` coefficients;
//! - **linear**: `Σ_c q(ζ_c) = rᵀb`, and `Σ_i r_i(η_j)·U[i][j] + linear blind[j] = q(η_j)`;
//! - **quadratic**: `p(ζ_c) = 0` for every `c`, and
//!   `Σ_i r_i·(U[x_i][j]·U[y_i][j] − U[z_i][j]) + quadratic blind[j] = p(η_j)`;
//!
//! and once: the opened columns and the siblings hash to the root.

use p3_field::PrimeCharacteristicRing;
use p3_goldilocks::Goldilocks;
use zk_core::{Control, Verdict};

use crate::challenge::{Combiners, combiners, open_seed, opened_columns, tests_seed};
use crate::code::{eta, evaluate, evaluate_columns, interpolate, zetas};
use crate::error::LigeroError;
use crate::field::Fp;
use crate::merkle::{leaf, opens_to};
use crate::params::{OPENED_COLUMNS, Params};
use crate::proof::{Proof, encode_responses};
use crate::statement::Statement;

type F = Goldilocks;

/// The outcome of each of the verifier's checks, so a demonstration can say which one caught a cheat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checks {
    /// The opened columns are the committed ones.
    pub merkle: bool,
    /// Every repetition's proximity test.
    pub proximity: bool,
    /// Every repetition's linear-constraint test.
    pub linear: bool,
    /// Every repetition's quadratic-constraint test.
    pub quadratic: bool,
}

impl Checks {
    /// Whether the proof is accepted.
    pub fn all(&self) -> bool {
        self.merkle && self.proximity && self.linear && self.quadratic
    }
}

/// What the verifier sees at the opened columns.
struct Opened {
    columns: Vec<Vec<F>>,
    points: Vec<F>,
}

fn raw(values: &[Fp]) -> Vec<F> {
    values.iter().map(|value| value.0).collect()
}

/// Runs every check. Bytes that do not decode, or a public input list of the wrong length, are an
/// `Err` with the reason.
pub fn check(statement: &Statement, public: &[Fp], bytes: &[u8]) -> Result<Checks, String> {
    statement.check_public(public).map_err(|error| error.to_string())?;
    let params = statement.params();
    let proof = Proof::decode(&params, bytes)?;
    let seed = tests_seed(statement, public, &proof.root);
    let combiners = combiners(statement, &seed);
    let open = open_seed(&seed, &encode_responses(&proof.responses));
    let indices = opened_columns(&open, params.length, OPENED_COLUMNS);
    let opened = Opened {
        columns: proof.columns.iter().map(|column| raw(&column.entries)).collect(),
        points: indices.iter().map(|j| eta(params.length, *j)).collect(),
    };
    let public = raw(public);
    let rounds = || combiners.iter().zip(&proof.responses).enumerate();
    Ok(Checks {
        merkle: merkle_holds(&params, &proof, &indices, &opened),
        proximity: rounds()
            .all(|(h, (c, r))| proximity_holds(&params, h, c, &raw(&r.code), &opened)),
        linear: rounds()
            .all(|(h, (c, r))| linear_holds(statement, h, c, &raw(&r.linear), &public, &opened)),
        quadratic: rounds()
            .all(|(h, (c, r))| quadratic_holds(&params, h, c, &raw(&r.quadratic), &opened)),
    })
}

/// [`check`] as a verdict.
pub fn verify(
    statement: &Statement,
    public: &[Fp],
    proof: &[u8],
    control: &Control,
) -> Result<Verdict, LigeroError> {
    control.checkpoint().map_err(|_| LigeroError::Cancelled)?;
    Ok(match check(statement, public, proof) {
        Err(why) => Verdict::Malformed(why),
        Ok(checks) if checks.all() => Verdict::Accepted,
        Ok(_) => Verdict::Rejected,
    })
}

fn merkle_holds(params: &Params, proof: &Proof, indices: &[usize], opened: &Opened) -> bool {
    let leaves = indices
        .iter()
        .zip(&proof.columns)
        .zip(&opened.columns)
        .map(|((j, column), entries)| (*j, leaf(&column.salt, entries)))
        .collect();
    opens_to(&proof.root, params.length, leaves, &proof.siblings)
}

fn proximity_holds(
    params: &Params,
    h: usize,
    combiner: &Combiners,
    code: &[F],
    opened: &Opened,
) -> bool {
    let blind = params.blind_rows(h).code;
    opened.columns.iter().zip(&opened.points).all(|(entries, point)| {
        let combined: F = combiner.code.iter().zip(entries).map(|(r, u)| *r * *u).sum();
        combined + entries[blind] == evaluate(code, *point)
    })
}

fn linear_holds(
    statement: &Statement,
    h: usize,
    combiner: &Combiners,
    linear: &[F],
    public: &[F],
    opened: &Opened,
) -> bool {
    let (params, system) = (statement.params(), statement.system());
    let sum: F = zetas(params.message_length).iter().map(|zeta| evaluate(linear, *zeta)).sum();
    if sum != system.combine_rhs(&combiner.linear, public) {
        return false;
    }
    let multipliers = interpolate(system.multipliers(&combiner.linear));
    let blind = params.blind_rows(h).linear;
    opened.columns.iter().zip(&opened.points).all(|(entries, point)| {
        let at_point = evaluate_columns(&multipliers, *point);
        let combined: F = at_point.iter().zip(entries).map(|(m, u)| *m * *u).sum();
        combined + entries[blind] == evaluate(linear, *point)
    })
}

fn quadratic_holds(
    params: &Params,
    h: usize,
    combiner: &Combiners,
    quadratic: &[F],
    opened: &Opened,
) -> bool {
    let zetas = zetas(params.message_length);
    if !zetas.iter().all(|zeta| evaluate(quadratic, *zeta) == F::ZERO) {
        return false;
    }
    let blind = params.blind_rows(h).quadratic;
    opened.columns.iter().zip(&opened.points).all(|(entries, point)| {
        let combined: F = (combiner.quadratic.iter().enumerate())
            .map(|(i, r)| {
                let (x, y, z) = (params.x_row(i), params.y_row(i), params.z_row(i));
                *r * (entries[x] * entries[y] - entries[z])
            })
            .sum();
        combined + entries[blind] == evaluate(quadratic, *point)
    })
}
