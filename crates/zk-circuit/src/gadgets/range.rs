//! Range checks by bit decomposition.

use crate::builder::CircuitBuilder;
use crate::error::CircuitError;
use crate::field::{ZkField, power_of_two};
use crate::gadgets::assert_boolean;
use crate::lc::{LinearCombination, Wire};

/// Requires `0 ≤ value < 2^bits` and returns the bits, least significant first.
///
/// Field elements wrap around, so "less than" has no meaning until a value is written as a sum of
/// boolean-constrained bits. `bits` must stay below the modulus bit length, otherwise two different
/// bit strings could sum to the same field element and the check would accept wrapped values.
///
/// Assertion labels: `"{label}: bit {i} is 0 or 1"` for each bit, then `label` for the
/// recomposition.
pub fn range_check<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    value: impl Into<LinearCombination<F>>,
    bits: u32,
    label: &str,
) -> Result<Vec<Wire>, CircuitError> {
    if bits >= F::MODULUS_BITS {
        return Err(CircuitError::RangeTooWide { bits, modulus_bits: F::MODULUS_BITS });
    }
    let value = value.into();
    let bit_wires = builder.hint_bits(value.clone(), bits);
    let mut recomposed = LinearCombination::zero();
    for (index, bit) in (0u32..).zip(&bit_wires) {
        assert_boolean(builder, *bit, &format!("{label}: bit {index} is 0 or 1"));
        recomposed = recomposed.add_term(*bit, power_of_two(index));
    }
    builder.assert_zero(value - recomposed, label);
    Ok(bit_wires)
}
