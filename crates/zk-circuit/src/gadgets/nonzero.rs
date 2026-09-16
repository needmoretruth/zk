//! Proving a value is not zero.

use crate::builder::CircuitBuilder;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};

/// Requires `value ≠ 0` and returns the wire holding its inverse.
///
/// There is no gate for "not equal"; a value is non-zero exactly when some `inverse` satisfies
/// `value · inverse = 1`, and the prover supplies that inverse as a hint.
pub fn assert_nonzero<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    value: impl Into<LinearCombination<F>>,
    label: &str,
) -> Wire {
    let value = value.into();
    let inverse = builder.hint_inverse(value.clone());
    let product = builder.mul(value, inverse);
    builder.assert_zero(LinearCombination::<F>::from(product).add_constant(F::one().neg()), label);
    inverse
}
