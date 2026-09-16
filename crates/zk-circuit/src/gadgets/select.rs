//! Choosing between two values by a boolean.

use crate::builder::CircuitBuilder;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};

/// `bit ? if_true : if_false`, computed as `if_false + bit · (if_true − if_false)`.
///
/// Circuits have no branches, so both values are always computed and the bit blends them. The
/// caller must constrain `bit` with [`crate::gadgets::assert_boolean`]; any other value would
/// produce a mixture of the two.
pub fn select<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    bit: Wire,
    if_true: impl Into<LinearCombination<F>>,
    if_false: impl Into<LinearCombination<F>>,
) -> Wire {
    let (if_true, if_false) = (if_true.into(), if_false.into());
    let blended = builder.mul(bit, if_true - if_false.clone());
    builder.linear(if_false + blended)
}
