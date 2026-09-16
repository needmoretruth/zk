//! How large the matrix is and how many columns are opened, derived from the paper's bounds.
//!
//! Notation as in the paper's Table 1: `ℓ` witness entries per row, `k` the code dimension (a row
//! is a polynomial of degree below `k`), `n` the code length (the columns), `d = n − k + 1` the
//! distance, `e` the errors the analysis tolerates, `t` the opened columns, `σ` the repetitions.
//!
//! # Soundness: about 2^-80 made non-interactive
//!
//! Appendix C of the extended version bounds a cheating prover's chance, for any `e < d/2`, by
//!
//! ```text
//! (1 − e/n)^t + ((k + ℓ)/n)^t + (2k/n)^t + (n + 3)/|F|^σ
//! ```
//!
//! Made non-interactive (§5.2), the prover can recompute each of the verifier's two messages by
//! rehashing: the random combinations (the last term is a bad draw of them) and the opened
//! columns (the first three terms). The paper sets the interactive error to `2^-κ` and the hash
//! output to `2κ` bits; a prover making `T` hash queries then succeeds with probability at most
//! about `T·2^-κ`. Here `κ = 80` and SHA-256 gives 256 ≥ 160 bits.
//!
//! - **Rate 1/4**, `n = 4k`: the power of two nearest the paper's optimum `n = 3k` (§5.3), since the
//!   encoding uses the power-of-two NTT (the paper notes the same constraint).
//! - **`e = (n − k)/2 = 3k/2`**, the largest integer below `d/2 = (3k + 1)/2`. Then `1 − e/n = 5/8`,
//!   `2k/n = 1/2`, and `(k + ℓ)/n < 1/2` because `ℓ < k`.
//! - **`t = 118`**: `(5/8)^118 + 2·(1/2)^118 = 2^-80.012`; with `t = 117` the first term alone is
//!   `2^-79.3`.
//! - **`σ = 2`** over Goldilocks: `(n + 3)/p² < 2^-95` for every `n ≤ 2^32`, the largest subgroup
//!   the field has.
//! - Together `2^-80.012 + 2^-95 < 2^-80`. The tests of this module recompute both numbers.
//!
//! # Zero knowledge
//!
//! Lemma 4.15 needs `k > ℓ + t`: a row polynomial then has at least `t + 1` free evaluations beyond
//! its `ℓ` message values, so any `t` opened columns are uniformly random. `k` is the smallest power
//! of two with `k ≥ ℓ + t + 1`.
//!
//! # The square root
//!
//! `ℓ` is chosen per circuit to minimise the paper's communication count (§5.3):
//! `σ·(k + (k + ℓ − 1) + (2k − 1))` response elements, `t` opened columns of one element per row,
//! and `t·⌈log₂ n⌉` hashes. Longer rows mean fewer rows in every opened column but longer
//! responses; the balance is at `ℓ ≈ √(t·entries/σ)`, so the proof grows with the square root of the
//! circuit.

use p3_field::PrimeField64;
use p3_goldilocks::Goldilocks;

use crate::error::LigeroError;

/// Security target of the non-interactive proof, in bits.
pub const SECURITY_BITS: u32 = 80;

/// `n = k·2^RATE_BITS`: rate 1/4.
pub const RATE_BITS: u32 = 2;

/// `t`, the columns the verifier opens.
pub const OPENED_COLUMNS: usize = 118;

/// `σ`, how many times each test is repeated with fresh random combinations.
pub const REPETITIONS: usize = 2;

/// Goldilocks has multiplicative subgroups of order up to 2^32.
const MAX_LOG_LENGTH: u32 = 32;

/// The shape of one circuit's proof: the code and how the extended witness fills the rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// `ℓ`: witness entries per row, carried at the points `ζ`.
    pub message_length: usize,
    /// `k`: every row polynomial has degree below `k`.
    pub dimension: usize,
    /// `n`: evaluations per row; the columns the Merkle tree commits to.
    pub length: usize,
    /// Rows holding the block `w`.
    pub witness_rows: usize,
    /// Rows holding each of the blocks `x`, `y` and `z`.
    pub triple_rows: usize,
}

/// Where one repetition's three blinding rows sit in the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BlindRows {
    /// Masks the proximity test's combination (a random codeword).
    pub code: usize,
    /// Masks the linear test (message summing to zero).
    pub linear: usize,
    /// Masks the quadratic test (vanishing on every `ζ`).
    pub quadratic: usize,
}

impl Params {
    /// The parameters for `witness` entries in `w` and `triples` quadratic constraints, with the `ℓ`
    /// that minimises the paper's communication count (the smallest `ℓ` on a tie).
    pub fn for_counts(witness: usize, triples: usize) -> Result<Self, LigeroError> {
        (0..MAX_LOG_LENGTH)
            .filter_map(|bits| Self::with_message_length(witness, triples, 1 << bits))
            .min_by_key(Self::communication_bits)
            .ok_or(LigeroError::TooLarge { entries: witness.saturating_add(triples * 3) })
    }

    fn with_message_length(witness: usize, triples: usize, message_length: usize) -> Option<Self> {
        let dimension = (message_length + OPENED_COLUMNS + 1).checked_next_power_of_two()?;
        let length = dimension.checked_mul(1 << RATE_BITS)?;
        if u64::try_from(length).ok()? > 1u64 << MAX_LOG_LENGTH {
            return None;
        }
        Some(Self {
            message_length,
            dimension,
            length,
            // At least one row, so the matrix of tested rows is never empty.
            witness_rows: witness.div_ceil(message_length).max(1),
            triple_rows: triples.div_ceil(message_length),
        })
    }

    /// Rows the proximity and linear tests read: `w`, `x`, `y`, `z`.
    pub fn tested_rows(&self) -> usize {
        self.witness_rows + 3 * self.triple_rows
    }

    /// Every committed row: the tested rows and three blinding rows per repetition.
    pub fn rows(&self) -> usize {
        self.tested_rows() + 3 * REPETITIONS
    }

    /// Row `i` of the block `x`; `y` and `z` follow at the same offsets, so the rows line up.
    pub(crate) fn x_row(&self, i: usize) -> usize {
        self.witness_rows + i
    }

    /// Row `i` of the block `y`.
    pub(crate) fn y_row(&self, i: usize) -> usize {
        self.witness_rows + self.triple_rows + i
    }

    /// Row `i` of the block `z`.
    pub(crate) fn z_row(&self, i: usize) -> usize {
        self.witness_rows + 2 * self.triple_rows + i
    }

    /// Repetition `h`'s blinding rows.
    pub(crate) fn blind_rows(&self, h: usize) -> BlindRows {
        let first = self.tested_rows() + 3 * h;
        BlindRows { code: first, linear: first + 1, quadratic: first + 2 }
    }

    /// The paper's communication count in bits (§5.3), which chooses `ℓ`.
    pub fn communication_bits(&self) -> u64 {
        let [k, l, t, sigma, rows] =
            [self.dimension, self.message_length, OPENED_COLUMNS, REPETITIONS, self.rows()]
                .map(|value| value as u64);
        let elements = sigma * (k + (k + l - 1) + (2 * k - 1)) + t * rows;
        64 * elements + 256 * t * u64::from(self.length.trailing_zeros())
    }

    /// Appendix C's bound on surviving the column choice with `opened` columns, `e = (n − k)/2`.
    pub fn column_error(&self, opened: usize) -> f64 {
        let [n, k, l] = [self.length, self.dimension, self.message_length].map(|v| v as f64);
        let e = ((self.length - self.dimension) / 2) as f64;
        let t = i32::try_from(opened).unwrap_or(i32::MAX);
        (1.0 - e / n).powi(t) + ((k + l) / n).powi(t) + (2.0 * k / n).powi(t)
    }

    /// Appendix C's bound on a bad draw of the random combinations, `(n + 3)/|F|^σ`.
    pub fn combination_error(&self) -> f64 {
        let field = Goldilocks::ORDER_U64 as f64;
        (self.length as f64 + 3.0) / field.powi(REPETITIONS as i32)
    }

    /// The whole bound, which this crate keeps at or below `2^-SECURITY_BITS`.
    pub fn soundness_error(&self) -> f64 {
        self.column_error(OPENED_COLUMNS) + self.combination_error()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const TARGET: f64 = 1.0 / (1u128 << SECURITY_BITS) as f64;

    fn spread() -> impl Iterator<Item = Params> {
        [(1, 1), (40, 12), (600, 400), (5_000, 4_000), (100_000, 80_000), (1 << 22, 1 << 21)]
            .into_iter()
            .map(|(witness, triples)| Params::for_counts(witness, triples).unwrap())
    }

    #[test]
    fn every_choice_meets_the_target_and_one_column_fewer_would_not() {
        for params in spread() {
            assert!(params.soundness_error() <= TARGET, "{params:?}");
            assert!(params.column_error(OPENED_COLUMNS - 1) > TARGET, "{params:?}");
            assert!(params.combination_error() < TARGET / f64::from(1u32 << 15), "{params:?}");
        }
        // The figure the module documentation quotes.
        let largest = Params {
            message_length: 1,
            dimension: 1 << 30,
            length: 1 << 32,
            witness_rows: 1,
            triple_rows: 1,
        };
        assert!(largest.combination_error() < 2f64.powi(-95));
    }

    #[test]
    fn every_choice_is_zero_knowledge_and_fits_the_quadratic_test() {
        for params in spread() {
            assert!(params.dimension > params.message_length + OPENED_COLUMNS, "{params:?}");
            assert_eq!(params.length, 4 * params.dimension);
            assert!(params.dimension.is_power_of_two() && params.message_length.is_power_of_two());
        }
    }

    #[test]
    fn sixteen_times_the_circuit_costs_about_four_times_the_proof() {
        let bits =
            |entries: usize| Params::for_counts(entries, entries).unwrap().communication_bits();
        let ratio = bits(1 << 20) as f64 / bits(1 << 16) as f64;
        assert!((3.0..5.0).contains(&ratio), "{ratio}");
    }
}
