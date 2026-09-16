//! The Stark252 field as the circuit layer sees it, and the byte forms its elements take.

use lambdaworks_math::field::element::FieldElement;
use lambdaworks_math::traits::{AsBytes, ByteConversion};
use stark_platinum_prover::PrimeField;
use zk_circuit::ZkField;

/// lambdaworks' Stark252 element, the field every value in this exhibit's trace lives in.
pub(crate) type Val = FieldElement<PrimeField>;

/// Bytes in one element, in the museum's form and in lambdaworks' serialized form alike.
pub(crate) const ELEMENT_BYTES: usize = 32;

/// StarkWare's field, `p = 2^251 + 17·2^192 + 1`, wrapped so it can implement [`ZkField`].
///
/// Stone, StarkEx and Starknet prove over this prime, and lambdaworks' STARK prover takes it as its
/// base field with no extension, so the seven statements are built over it and no value is ever
/// converted between fields. The orphan rule forbids implementing the museum's trait on
/// lambdaworks' `FieldElement` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stark252(pub Val);

impl ZkField for Stark252 {
    /// `2^251 < p < 2^252`.
    const MODULUS_BITS: u32 = 252;
    const NAME: &'static str = "Stark252";

    fn zero() -> Self {
        Self(Val::zero())
    }
    fn one() -> Self {
        Self(Val::one())
    }
    fn from_u64(value: u64) -> Self {
        Self(Val::from(value))
    }
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
    fn neg(self) -> Self {
        Self(-self.0)
    }
    fn inverse(self) -> Option<Self> {
        self.0.inv().ok().map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.0)
    }
}

/// The museum's shared form: the canonical representative in `0..p` as 32 little-endian bytes.
pub(crate) fn encode(value: &Val) -> Vec<u8> {
    value.to_bytes_le().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value at or above the modulus.
///
/// lambdaworks' `from_bytes_le` silently reduces an oversized integer, so the result is encoded
/// again and compared: only the one canonical encoding of each element is accepted.
pub(crate) fn decode(bytes: &[u8]) -> Option<Val> {
    if bytes.len() != ELEMENT_BYTES {
        return None;
    }
    let value = Val::from_bytes_le(bytes).ok()?;
    (encode(&value) == bytes).then_some(value)
}

/// The bytes lambdaworks writes for `value` wherever a proof carries it.
///
/// With `lambdaworks-serde-binary`, which the prover crate switches on, a field element serializes
/// its internal representative, the Montgomery form `value·2^256 mod p`, as 32 big-endian bytes;
/// the Merkle leaves hash the same bytes (`AsBytes`). Asking lambdaworks rather than writing the
/// Montgomery multiplication here keeps the leak scan in step with the upstream encoding.
pub(crate) fn serialized(value: &Val) -> Vec<u8> {
    value.as_bytes()
}
