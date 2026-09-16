//! Winterfell's 128-bit prime field as the circuit layer sees it, and its byte forms.

use winter_math::fields::f128::BaseElement;
use winter_math::{FieldElement, StarkField};
use winter_utils::Serializable;
use zk_circuit::ZkField;

/// Winterfell's own f128 element, the field every value in this exhibit's trace lives in.
pub(crate) type Val = BaseElement;

/// Bytes in one element, in the museum's form and in Winterfell's.
const ELEMENT_BYTES: usize = 16;

/// Winterfell's f128, `p = 2^128 − 45·2^40 + 1`, wrapped so it can implement [`ZkField`].
///
/// The seven statements are built over the very field Winterfell proves over, so no value is ever
/// converted between fields. The orphan rule forbids implementing the museum's trait on
/// `winter_math::fields::f128::BaseElement` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct F128(pub Val);

impl ZkField for F128 {
    const MODULUS_BITS: u32 = Val::MODULUS_BITS;
    const NAME: &'static str = "Winterfell f128";

    fn zero() -> Self {
        Self(Val::ZERO)
    }
    fn one() -> Self {
        Self(Val::ONE)
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
        (self.0 != Val::ZERO).then(|| Self(self.0.inv()))
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(self.0)
    }
}

/// The museum's shared form: the canonical representative in `0..p` as 16 little-endian bytes.
pub(crate) fn encode(value: Val) -> Vec<u8> {
    value.as_int().to_le_bytes().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value not below the modulus, so
/// a caller can report undecodable input instead of silently reducing it.
pub(crate) fn decode(bytes: &[u8]) -> Option<Val> {
    let array: [u8; ELEMENT_BYTES] = bytes.try_into().ok()?;
    Val::try_from(u128::from_le_bytes(array)).ok()
}

/// The bytes Winterfell's own serializer writes for `value` wherever a proof carries it.
///
/// Asking `Serializable` rather than writing the layout by hand keeps the leak scan in step with
/// the upstream encoding. For f128 in 0.13.1 it happens to be the canonical 16 little-endian bytes.
pub(crate) fn serialized(value: Val) -> Vec<u8> {
    value.to_bytes()
}
