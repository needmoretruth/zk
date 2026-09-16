//! `factoring`: I know two factors of `n`, neither of them 1.
//!
//! The classic "hard problem" statement, and a lesson in wrap-around: without range checks a
//! prover could use field inverses or a factor of 1. Each factor is proved to lie in `2..2^15`, so
//! the product stays below 2^30 and cannot wrap in any supported field.

use zk_circuit::gadgets::range_check;
use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, Wire, ZkField,
};

/// Permanent ID.
pub const ID: &str = "factoring";

/// Each factor is below 2^15.
pub const FACTOR_BITS: u32 = 15;

const P: u64 = 181;
const Q: u64 = 191;

/// `n`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&["n"])
}

/// `p`, `q`.
pub fn private_input_names() -> Vec<String> {
    crate::names(&["p", "q"])
}

/// Asserts `2 ≤ p, q < 2^15` (15-bit decompositions of `v` and of `v − 2`) and `p·q = n`.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let n = builder.public_input("n");
    let p = builder.private_input("p");
    let q = builder.private_input("q");
    bound_factor(&mut builder, p, "p")?;
    bound_factor(&mut builder, q, "q")?;
    let product = builder.mul(p, q);
    builder.assert_zero(LinearCombination::<F>::from(product) - n, "p times q equals n");
    builder.finish()
}

fn bound_factor<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    factor: Wire,
    name: &str,
) -> Result<(), CircuitError> {
    range_check(builder, factor, FACTOR_BITS, &format!("{name} fits in 15 bits"))?;
    let shifted = LinearCombination::<F>::from(factor).add_constant(F::from_u64(2).neg());
    range_check(builder, shifted, FACTOR_BITS, &format!("{name} - 2 fits in 15 bits"))?;
    Ok(())
}

/// `34571 = 181 · 191`.
pub fn honest<F: ZkField>() -> Assignment<F> {
    Assignment { public: crate::values(&[P * Q]), private: crate::values(&[P, Q]) }
}

/// The trivial factorisation `34571 = 1 · 34571`.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    Assignment { public: crate::values(&[P * Q]), private: crate::values(&[1, P * Q]) }
}
