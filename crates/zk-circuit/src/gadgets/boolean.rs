//! Constraining a value to 0 or 1.

use crate::builder::CircuitBuilder;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};

/// Requires `value ∈ {0, 1}` through `value · (value − 1) = 0`.
///
/// Selections and bit decompositions are only sound on booleans; a "bit" of 2 would let a prover
/// pick values outside the intended range.
pub fn assert_boolean<F: ZkField>(builder: &mut CircuitBuilder<F>, value: Wire, label: &str) {
    let shifted = LinearCombination::<F>::from(value).add_constant(F::one().neg());
    let product = builder.mul(value, shifted);
    builder.assert_zero(product, label);
}
