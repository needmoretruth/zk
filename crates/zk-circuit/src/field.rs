//! The arithmetic interface every circuit is written against.

use core::fmt::Debug;

use crate::error::CircuitError;

/// Smallest modulus bit length a circuit may be built over.
///
/// Every example keeps its integers below 2^30 (two 15-bit factors multiplied, sums of three
/// 16-bit amounts, a 20-bit PIN), so a prime above 2^30 never wraps them around. Every prime with
/// 31 bits exceeds 2^30, which admits M31 and BabyBear, the smallest fields in the museum.
pub const MIN_MODULUS_BITS: u32 = 31;

/// A prime field, as the circuit layer sees it.
///
/// The museum runs proof systems from several ecosystems (`ff`, arkworks, Plonky3, winterfell) that
/// pin different and incompatible field traits. Writing each statement against this one small
/// trait lets it be written once. A proof-system crate implements the trait on a newtype around its
/// own field, because the orphan rule forbids implementing it on the foreign type directly.
///
/// Implementations must uphold what the compiler cannot check:
/// - the modulus `p` is prime and exceeds 2^30 ([`check_field`] rejects smaller fields at runtime);
/// - `from_u64` maps an integer to its residue modulo `p`;
/// - `to_le_bytes` returns the canonical representative, the unique integer in `0..p`.
pub trait ZkField: Copy + Eq + Debug + Send + Sync + 'static {
    /// Bit length of the modulus, so range checks can refuse widths that would wrap around.
    const MODULUS_BITS: u32;
    /// Human name of the field, such as `"BabyBear"`, for comparison tables.
    const NAME: &'static str;

    /// The additive identity.
    fn zero() -> Self;
    /// The multiplicative identity.
    fn one() -> Self;
    /// The residue of `value` modulo `p`; the only way the circuit layer creates constants.
    fn from_u64(value: u64) -> Self;
    /// Field addition.
    fn add(self, rhs: Self) -> Self;
    /// Field subtraction.
    fn sub(self, rhs: Self) -> Self;
    /// Field multiplication.
    fn mul(self, rhs: Self) -> Self;
    /// Additive inverse.
    fn neg(self) -> Self;
    /// Multiplicative inverse, `None` for zero; used by the inverse hint.
    fn inverse(self) -> Option<Self>;
    /// Canonical little-endian bytes of the representative in `0..p`; bit decomposition hints
    /// read bits from here, so a non-canonical encoding would break range checks.
    fn to_le_bytes(self) -> Vec<u8>;
}

/// Reads a field element back as an integer when its canonical representative fits in 64 bits.
///
/// Bit-decomposition hints and native helpers need the integer behind a field element; the trait
/// only promises bytes, so this is the one place that turns them back into a number.
pub fn to_u64<F: ZkField>(value: F) -> Option<u64> {
    let bytes = value.to_le_bytes();
    if bytes.iter().skip(8).any(|byte| *byte != 0) {
        return None;
    }
    let mut buffer = [0u8; 8];
    for (slot, byte) in buffer.iter_mut().zip(bytes.iter()) {
        *slot = *byte;
    }
    Some(u64::from_le_bytes(buffer))
}

/// Refuses fields too small for the examples, before any circuit is built over them.
///
/// Two checks, because `MODULUS_BITS` is a claim made by the implementer: the declared bit length,
/// and a round trip of 2^30 through `from_u64` and `to_le_bytes`, which only survives when the
/// modulus really exceeds 2^30.
pub fn check_field<F: ZkField>() -> Result<(), CircuitError> {
    let probe = 1u64 << (MIN_MODULUS_BITS - 1);
    let round_trips = to_u64(F::from_u64(probe)) == Some(probe);
    if F::MODULUS_BITS < MIN_MODULUS_BITS || !round_trips {
        return Err(CircuitError::FieldTooSmall { field: F::NAME, modulus_bits: F::MODULUS_BITS });
    }
    Ok(())
}

/// 2^exponent in the field, computed by doubling so exponents past 63 work on large fields.
pub(crate) fn power_of_two<F: ZkField>(exponent: u32) -> F {
    let mut acc = F::one();
    for _ in 0..exponent {
        acc = acc.add(acc);
    }
    acc
}
