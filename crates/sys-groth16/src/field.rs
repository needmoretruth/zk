//! The BLS12-381 scalar field, as the circuit layer sees it, and its 32-byte encoding.

use bls12_381::Scalar;
use ff::{Field, PrimeField};
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// The scalar field of BLS12-381, the field Zcash Sapling's circuits are written over.
///
/// The museum's statements are written against [`ZkField`]; the orphan rule forbids implementing
/// that trait on `bls12_381::Scalar` directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fr(pub Scalar);

impl ZkField for Fr {
    const MODULUS_BITS: u32 = Scalar::NUM_BITS;
    const NAME: &'static str = "BLS12-381 scalar field";

    fn zero() -> Self {
        Self(Scalar::ZERO)
    }
    fn one() -> Self {
        Self(Scalar::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(Scalar::from(value))
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
        Option::from(self.0.invert()).map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.0)
    }
}

/// `Scalar::to_repr`: the canonical representative as 32 little-endian bytes.
pub(crate) fn encode(value: &Scalar) -> FieldBytes {
    value.to_repr().to_vec()
}

/// `Scalar::from_repr`, which refuses any 32 bytes that encode an integer at or above the modulus.
pub(crate) fn decode(bytes: &[u8]) -> Result<Scalar, String> {
    let repr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("a BLS12-381 scalar is 32 bytes, got {}", bytes.len()))?;
    Option::from(Scalar::from_repr(repr))
        .ok_or_else(|| "not a canonical BLS12-381 scalar (at or above the modulus)".to_string())
}
