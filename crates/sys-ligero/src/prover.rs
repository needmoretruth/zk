//! The prover (§4.7 and §5.2): encode, commit, answer the combinations, open the columns.
//!
//! It never checks the witness. A false claim is encoded and answered exactly like a true one, and
//! the verifier's sums at `ζ` come out wrong.

use p3_goldilocks::Goldilocks;
use p3_matrix::dense::RowMajorMatrix;
use zk_core::Control;

use crate::challenge::{Combiners, combiners, open_seed, opened_columns, tests_seed};
use crate::code::{
    Blinds, committed_polynomials, extend, interpolate_one, on_coset, on_subgroup, row,
    witness_polynomials,
};
use crate::coins::Coins;
use crate::error::LigeroError;
use crate::field::Fp;
use crate::hash::Bytes32;
use crate::merkle::{Tree, leaf};
use crate::params::{OPENED_COLUMNS, Params, REPETITIONS};
use crate::proof::{OpenedColumn, Proof, Response, encode_responses};
use crate::statement::{Claim, Statement};

type F = Goldilocks;

/// What the prover keeps after committing.
struct Committed {
    /// The tested rows' polynomials, `k` high.
    witness: RowMajorMatrix<F>,
    /// One per repetition.
    blinds: Vec<Blinds>,
    /// Every committed row's evaluations on `η`: row `j` of the matrix is column `j` of the paper.
    columns: RowMajorMatrix<F>,
    salts: Vec<Bytes32>,
    tree: Tree,
}

fn checkpoint(control: &Control) -> Result<(), LigeroError> {
    control.checkpoint().map_err(|_| LigeroError::Cancelled)
}

/// Proves `claim` without checking it.
pub fn prove(
    statement: &Statement,
    claim: &Claim,
    control: &Control,
) -> Result<Vec<u8>, LigeroError> {
    statement.check_public(&claim.public)?;
    let params = statement.params();
    let messages = statement.messages(&claim.witness)?;
    let mut coins = Coins::new();
    let committed = commit(&params, &messages, &mut coins)?;
    checkpoint(control)?;
    let root = committed.tree.root();
    let seed = tests_seed(statement, &claim.public, &root);
    let responses = respond(statement, &committed, &combiners(statement, &seed));
    checkpoint(control)?;
    let opened = opened_columns(
        &open_seed(&seed, &encode_responses(&responses)),
        params.length,
        OPENED_COLUMNS,
    );
    let columns = opened
        .iter()
        .map(|j| OpenedColumn {
            salt: committed.salts[*j],
            entries: row(&committed.columns, *j).iter().map(|entry| Fp(*entry)).collect(),
        })
        .collect();
    let siblings = committed.tree.siblings(&opened);
    Ok(Proof { root, responses, columns, siblings }.encode(&params))
}

/// Encodes the rows, adds the blinding rows and commits to every column.
fn commit(params: &Params, messages: &[F], coins: &mut Coins) -> Result<Committed, LigeroError> {
    let witness = witness_polynomials(messages, params, coins)?;
    let blinds =
        (0..REPETITIONS).map(|_| Blinds::sample(params, coins)).collect::<Result<Vec<_>, _>>()?;
    let columns = on_coset(committed_polynomials(&witness, &blinds, params), params.length);
    let salts = (0..params.length).map(|_| coins.salt()).collect::<Result<Vec<_>, _>>()?;
    let leaves = salts.iter().enumerate().map(|(j, salt)| leaf(salt, row(&columns, j))).collect();
    Ok(Committed { witness, blinds, columns, salts, tree: Tree::new(leaves) })
}

/// Every repetition's three polynomials.
fn respond(statement: &Statement, committed: &Committed, combiners: &[Combiners]) -> Vec<Response> {
    let params = statement.params();
    let doubled = 2 * params.dimension;
    // Products of two rows have degree below 2k − 1, so the subgroup of order 2k holds them exactly.
    let on_doubled = on_subgroup(committed.witness.clone(), doubled);
    combiners
        .iter()
        .zip(&committed.blinds)
        .map(|(combiner, blind)| {
            let code = combine_rows(&committed.witness, &combiner.code);
            let linear = linear_polynomial(statement, &on_doubled, &combiner.linear);
            let quadratic = quadratic_polynomial(&params, &on_doubled, &combiner.quadratic);
            Response {
                code: plus(&code, &blind.code),
                linear: plus(&linear, &blind.linear),
                quadratic: plus(&quadratic, &blind.quadratic),
            }
        })
        .collect()
}

/// `a + b`, coefficient by coefficient, as the proof's elements.
fn plus(a: &[F], b: &[F]) -> Vec<Fp> {
    a.iter().zip(b).map(|(x, y)| Fp(*x + *y)).collect()
}

/// `Σ r_i·p_i` in coefficients.
fn combine_rows(polynomials: &RowMajorMatrix<F>, combiners: &[F]) -> Vec<F> {
    polynomials
        .row_slices()
        .map(|coefficients| coefficients.iter().zip(combiners).map(|(c, r)| *c * *r).sum())
        .collect()
}

/// `Σ_i r_i(X)·p_i(X)`, where `r_i` takes row `i`'s multipliers of `rᵀA` at `ζ`.
fn linear_polynomial(
    statement: &Statement,
    on_doubled: &RowMajorMatrix<F>,
    combiners: &[F],
) -> Vec<F> {
    let params = statement.params();
    let height = 2 * params.dimension;
    let lifted = extend(statement.system().multipliers(combiners), height);
    let sums: Vec<F> = (0..height)
        .map(|x| row(&lifted, x).iter().zip(row(on_doubled, x)).map(|(r, p)| *r * *p).sum())
        .collect();
    let mut coefficients = interpolate_one(sums);
    coefficients.truncate(params.dimension + params.message_length - 1);
    coefficients
}

/// `Σ_i r_i·(p_{x,i}·p_{y,i} − p_{z,i})`.
fn quadratic_polynomial(
    params: &Params,
    on_doubled: &RowMajorMatrix<F>,
    combiners: &[F],
) -> Vec<F> {
    let height = 2 * params.dimension;
    let sums: Vec<F> = (0..height)
        .map(|x| {
            let values = row(on_doubled, x);
            combiners
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let (px, py, pz) =
                        (values[params.x_row(i)], values[params.y_row(i)], values[params.z_row(i)]);
                    *r * (px * py - pz)
                })
                .sum()
        })
        .collect();
    let mut coefficients = interpolate_one(sums);
    coefficients.truncate(height - 1);
    coefficients
}
