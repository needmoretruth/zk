//! `age`: I am at least 18, without revealing my birth year.
//!
//! Teaches range proofs by bit decomposition, the job Bulletproofs was designed for. An issuer
//! hands out `credential = ToyHash(birth_year, salt)`; the holder proves the sealed year is old
//! enough for this year's threshold.

use zk_circuit::gadgets::range_check;
use zk_circuit::toyhash::{toyhash, toyhash_gadget};
use zk_circuit::{Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, ZkField};

/// Permanent ID.
pub const ID: &str = "age";

/// Bits of a birth year: years below 4096.
pub const YEAR_BITS: u32 = 12;
/// Bits of `year − birth_year − threshold`: a non-negative margin below 256.
pub const MARGIN_BITS: u32 = 8;

const YEAR: u64 = 2026;
const THRESHOLD: u64 = 18;
const SALT: u64 = 271_828;
/// Birth year of the honest holder: 36 in 2026.
pub const ADULT_BIRTH_YEAR: u64 = 1990;
/// Birth year of the dishonest holder: 17 in 2026.
pub const SEVENTEEN_YEAR_OLD_BIRTH_YEAR: u64 = 2009;

/// `year`, `threshold`, `credential`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&["year", "threshold", "credential"])
}

/// `birth_year`, `salt`.
pub fn private_input_names() -> Vec<String> {
    crate::names(&["birth_year", "salt"])
}

/// Asserts `birth_year < 2^12`, `ToyHash(birth_year, salt) = credential` and
/// `0 ≤ year − birth_year − threshold < 2^8`.
///
/// A negative margin wraps to a huge field element, which has no 8-bit decomposition.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let year = builder.public_input("year");
    let threshold = builder.public_input("threshold");
    let credential = builder.public_input("credential");
    let birth_year = builder.private_input("birth_year");
    let salt = builder.private_input("salt");
    range_check(&mut builder, birth_year, YEAR_BITS, "birth_year fits in 12 bits")?;
    let sealed = toyhash_gadget(&mut builder, birth_year, salt);
    builder.assert_zero(
        LinearCombination::<F>::from(sealed) - credential,
        "credential seals the birth year",
    );
    let margin = LinearCombination::<F>::from(year) - birth_year - threshold;
    range_check(&mut builder, margin, MARGIN_BITS, "year - birth_year - threshold fits in 8 bits")?;
    builder.finish()
}

/// A holder born in `birth_year` whose credential was sealed with `salt`.
pub fn born_in<F: ZkField>(birth_year: u64, salt: u64) -> Assignment<F> {
    let [year, threshold, birth_year, salt] = [YEAR, THRESHOLD, birth_year, salt].map(F::from_u64);
    Assignment {
        public: vec![year, threshold, toyhash(birth_year, salt)],
        private: vec![birth_year, salt],
    }
}

/// Born in 1990: 36 in 2026.
pub fn honest<F: ZkField>() -> Assignment<F> {
    born_in(ADULT_BIRTH_YEAR, SALT)
}

/// Born in 2009 with a genuine credential: 17 in 2026.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    born_in(SEVENTEEN_YEAR_OLD_BIRTH_YEAR, SALT)
}
