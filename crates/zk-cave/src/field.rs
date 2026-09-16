//! The field the wall checks the magic words in.

use p3_field::{Field as _, PrimeCharacteristicRing, PrimeField64};
use zk_circuit::ZkField;

/// Goldilocks, `p = 2^64 − 2^32 + 1`, from Plonky3.
///
/// The cave does no proof arithmetic: the wall only evaluates the example's circuit, so any field
/// above 2^30 would do. Goldilocks keeps every example integer far from wrapping around and encodes
/// in eight bytes. A newtype, because the orphan rule forbids implementing [`ZkField`] on
/// Plonky3's type directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goldilocks(pub p3_goldilocks::Goldilocks);

impl Goldilocks {
    /// Reads back what [`ZkField::to_le_bytes`] wrote: exactly eight bytes holding an integer below
    /// `p`. Anything else is `None`, so a malformed public input cannot quietly become another value.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Option<Self> {
        let array: [u8; 8] = bytes.try_into().ok()?;
        let value = u64::from_le_bytes(array);
        (value < p3_goldilocks::Goldilocks::ORDER_U64).then(|| Self::from_u64(value))
    }
}

impl ZkField for Goldilocks {
    const MODULUS_BITS: u32 = 64;
    const NAME: &'static str = "Goldilocks";

    fn zero() -> Self {
        Self(p3_goldilocks::Goldilocks::ZERO)
    }

    fn one() -> Self {
        Self(p3_goldilocks::Goldilocks::ONE)
    }

    fn from_u64(value: u64) -> Self {
        Self(p3_goldilocks::Goldilocks::from_u64(value))
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
        self.0.as_canonical_u64().to_le_bytes().to_vec()
    }
}
