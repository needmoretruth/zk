//! BabyBear as the circuit layer sees it, and the byte forms the museum and Plonky3 use for it.

use p3_field::{Field as _, PrimeCharacteristicRing, PrimeField32};
use zk_circuit::ZkField;

/// Plonky3's own BabyBear, the field every value in this exhibit's trace lives in.
pub(crate) type Val = p3_baby_bear::BabyBear;

/// BabyBear, `p = 15·2^27 + 1`, wrapped so it can implement [`ZkField`].
///
/// The seven statements are built over the very field Plonky3 proves over, so no value is ever
/// converted between fields. The orphan rule forbids implementing the museum's trait on
/// `p3_baby_bear::BabyBear` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BabyBear(pub Val);

impl ZkField for BabyBear {
    const MODULUS_BITS: u32 = 31;
    const NAME: &'static str = "BabyBear";

    fn zero() -> Self {
        Self(Val::ZERO)
    }
    fn one() -> Self {
        Self(Val::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(Val::from_u64(value))
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
        self.0.try_inverse().map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(self.0)
    }
}

/// The museum's shared form: the canonical representative in `0..p` as 4 little-endian bytes.
pub(crate) fn encode(value: Val) -> Vec<u8> {
    value.as_canonical_u32().to_le_bytes().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value not below the modulus, so
/// a caller can report undecodable input instead of silently reducing it.
pub(crate) fn decode(bytes: &[u8]) -> Option<Val> {
    let array: [u8; 4] = bytes.try_into().ok()?;
    let integer = u32::from_le_bytes(array);
    (integer < Val::ORDER_U32).then(|| Val::new(integer))
}

/// The bytes a proof would carry if `value` itself were opened in it.
///
/// A proof opens trace values in the degree-4 challenge field, and `p3-monty-31` 0.7.0 serializes
/// each limb in its internal Montgomery form `value·2^32 mod p` as 4 little-endian bytes, so a
/// trace value appears as its Montgomery form followed by three zero limbs. Searching for all 16
/// bytes rather than the 4 of one limb matters: a 31-bit value is short enough to turn up by chance
/// in a proof of several hundred kilobytes, and small ones (a sudoku cell) match length and count
/// fields, while the zero limbs pin the match to an opened value. Asking postcard itself keeps this
/// in step with the upstream encoding.
pub(crate) fn serialized(value: Val) -> Option<Vec<u8>> {
    postcard::to_allocvec(&crate::config::Challenge::from(value)).ok()
}
