//! ToyHash: the one hash every example uses. **Not a real hash.**
//!
//! FROZEN. Changing anything here changes every public value in every example.
//!
//! A 2-to-1 compression in the MiMC-Feistel style: `L = a, R = b`; for round `i` in `0..16`,
//! `t = L + c_i` and `(L, R) = (R + t^3, L)`; the output is `L + a`. The Feistel shape keeps it
//! defined on fields where cubing is not a permutation, such as M31. It exists to give every proof
//! system the same small, algebraic workload. It has no security analysis, sixteen rounds are far
//! too few, and on a 31-bit field its output can be inverted by brute force. Do not use it to
//! protect anything.

use std::sync::LazyLock;

use sha2::{Digest, Sha256};

use crate::builder::CircuitBuilder;
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};

/// Number of Feistel rounds.
pub const ROUNDS: usize = 16;

static ROUND_CONSTANTS: LazyLock<[u64; ROUNDS]> = LazyLock::new(|| {
    let mut constants = [0u64; ROUNDS];
    for (round, constant) in constants.iter_mut().enumerate() {
        let digest = Sha256::digest(format!("zk/toyhash/v1/round/{round}").as_bytes());
        let mut first = [0u8; 8];
        first.copy_from_slice(&digest[..8]);
        *constant = u64::from_le_bytes(first);
    }
    constants
});

/// Round constants as integers: `c_i` is the first 8 bytes, read little-endian, of
/// SHA-256(`"zk/toyhash/v1/round/{i}"`).
///
/// Derived from a public string so anyone can recompute them; each field maps them in with
/// [`ZkField::from_u64`].
pub fn round_constants() -> &'static [u64; ROUNDS] {
    &ROUND_CONSTANTS
}

/// ToyHash on field values, for computing public inputs and honest witnesses. Not a real hash.
pub fn toyhash<F: ZkField>(a: F, b: F) -> F {
    let (mut left, mut right) = (a, b);
    for constant in round_constants() {
        let t = left.add(F::from_u64(*constant));
        let cube = t.mul(t).mul(t);
        (left, right) = (right.add(cube), left);
    }
    left.add(a)
}

/// ToyHash as gates; agrees with [`toyhash`] on every input. Not a real hash.
///
/// Each round costs two multiplications (`t^2`, `t^3`) and one linear gate for the new `L`;
/// `t = L + c_i` stays a linear combination so it adds no wire.
pub fn toyhash_gadget<F: ZkField>(builder: &mut CircuitBuilder<F>, a: Wire, b: Wire) -> Wire {
    let (mut left, mut right) = (a, b);
    for constant in round_constants() {
        let t = LinearCombination::<F>::from(left).add_constant(F::from_u64(*constant));
        let square = builder.mul(t.clone(), t.clone());
        let cube = builder.mul(square, t);
        let next_left = builder.linear(LinearCombination::<F>::from(right) + cube);
        (left, right) = (next_left, left);
    }
    builder.linear(LinearCombination::<F>::from(left) + a)
}
