//! Merkle roots over ToyHash, natively and as gates.

use crate::builder::CircuitBuilder;
use crate::error::CircuitError;
use crate::field::ZkField;
use crate::gadgets::assert_boolean;
use crate::lc::{LinearCombination, Wire};
use crate::toyhash::{toyhash, toyhash_gadget};

/// Root of the tree containing `leaf`, as gates.
///
/// Convention shared with [`merkle_root_native`]: level 0 is the leaf level; `path_bits[i]` is bit
/// `i` of the leaf's index, where 0 means the current node is the left child
/// (`node = ToyHash(current, sibling)`) and 1 means it is the right child
/// (`node = ToyHash(sibling, current)`). The gadget constrains every path bit to be boolean with
/// label `"{label}: path bit {i} is 0 or 1"`, because a non-boolean bit would let a prover hash a
/// blend of the two children.
pub fn merkle_root<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    leaf: Wire,
    path_bits: &[Wire],
    siblings: &[Wire],
    label: &str,
) -> Result<Wire, CircuitError> {
    if path_bits.len() != siblings.len() {
        return Err(CircuitError::PathLengthMismatch {
            bits: path_bits.len(),
            siblings: siblings.len(),
        });
    }
    let mut current = leaf;
    for (level, (bit, sibling)) in path_bits.iter().zip(siblings).enumerate() {
        assert_boolean(builder, *bit, &format!("{label}: path bit {level} is 0 or 1"));
        // One multiplication orders both children: left = current + bit·(sibling − current),
        // right = sibling − bit·(sibling − current).
        let swap = builder.mul(*bit, LinearCombination::<F>::from(*sibling) - current);
        let left = builder.linear(LinearCombination::<F>::from(current) + swap);
        let right = builder.linear(LinearCombination::<F>::from(*sibling) - swap);
        current = toyhash_gadget(builder, left, right);
    }
    Ok(current)
}

/// Root of the tree containing `leaf`, on field values, with the convention of [`merkle_root`].
pub fn merkle_root_native<F: ZkField>(
    leaf: F,
    path_bits: &[bool],
    siblings: &[F],
) -> Result<F, CircuitError> {
    if path_bits.len() != siblings.len() {
        return Err(CircuitError::PathLengthMismatch {
            bits: path_bits.len(),
            siblings: siblings.len(),
        });
    }
    Ok(path_bits.iter().zip(siblings).fold(leaf, |current, (is_right, sibling)| {
        if *is_right { toyhash(*sibling, current) } else { toyhash(current, *sibling) }
    }))
}
