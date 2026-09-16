//! Field elements as the pool stores them, and fresh random ones.
//!
//! Files hold an element as the hex of its canonical little-endian bytes, the same bytes every proof
//! system in the museum exchanges, so two equal elements always have equal strings and the ledger
//! can compare roots and nullifiers as text.

use zk_circuit::ZkField;

use crate::error::PoolError;

/// Lowercase hex of the canonical bytes.
pub(crate) fn to_hex<F: ZkField>(value: F) -> String {
    hex::encode(value.to_le_bytes())
}

/// Reads [`to_hex`]'s form back, refusing anything that is not a canonical element of `F`.
///
/// [`ZkField`] only offers `from_u64`, so the bytes are folded in most significant first
/// (`value · 256 + byte`), and the result must encode back to the same bytes.
pub(crate) fn from_hex<F: ZkField>(text: &str) -> Result<F, PoolError> {
    let bytes =
        hex::decode(text).map_err(|e| PoolError::Corrupt(format!("{text:?} is not hex: {e}")))?;
    let base = F::from_u64(256);
    let value = bytes
        .iter()
        .rev()
        .fold(F::zero(), |value, byte| value.mul(base).add(F::from_u64(u64::from(*byte))));
    if value.to_le_bytes() != bytes {
        return Err(PoolError::Corrupt(format!("{text} is not a canonical {} element", F::NAME)));
    }
    Ok(value)
}

/// A uniformly random element for a key or note randomness, from the operating system.
///
/// 320 random bits are reduced modulo `p`; for fields of up to 256 bits the distance from uniform
/// is below 2^-64.
pub(crate) fn random<F: ZkField>() -> Result<F, PoolError> {
    let mut bytes = [0u8; 40];
    getrandom::fill(&mut bytes).map_err(|e| PoolError::Randomness(e.to_string()))?;
    let half = F::from_u64(1 << 32);
    let base = half.mul(half);
    let (limbs, _) = bytes.as_chunks::<8>();
    Ok(limbs
        .iter()
        .fold(F::zero(), |value, limb| value.mul(base).add(F::from_u64(u64::from_le_bytes(*limb)))))
}

/// A signed amount as a field element: `−5` becomes `p − 5`, which is how a counterfeit output
/// balances extra value in a circuit without range checks.
pub(crate) fn signed<F: ZkField>(value: i64) -> F {
    let magnitude = F::from_u64(value.unsigned_abs());
    if value < 0 { magnitude.neg() } else { magnitude }
}

/// An amount a wallet can hold: a positive integer below 2^16.
pub(crate) fn note_value(value: i64) -> Option<u16> {
    u16::try_from(value).ok().filter(|value| *value > 0)
}
