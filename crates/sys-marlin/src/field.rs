//! The BLS12-381 scalar field as arkworks 0.3 defines it, seen by the circuit layer, and its
//! 32-byte encoding.

use ark_bls12_381::Fr as Scalar;
use ark_ff::{BigInteger, Field, FpParameters, One, PrimeField, Zero};
use ark_serialize::CanonicalDeserialize;
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// The scalar field of BLS12-381, the curve this exhibit pairs on.
///
/// The museum's statements are written against [`ZkField`]; the orphan rule forbids implementing
/// that trait on `ark_bls12_381::Fr` directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fr(pub Scalar);

impl ZkField for Fr {
    const MODULUS_BITS: u32 = <<Scalar as PrimeField>::Params as FpParameters>::MODULUS_BITS;
    const NAME: &'static str = "BLS12-381 scalar field";

    fn zero() -> Self {
        Self(Scalar::zero())
    }
    fn one() -> Self {
        Self(Scalar::one())
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
        Field::inverse(&self.0).map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.0)
    }
}

/// `into_repr` out of Montgomery form: the canonical representative as 32 little-endian bytes,
/// which is also what arkworks' own serializer writes for a scalar.
pub(crate) fn encode(value: &Scalar) -> FieldBytes {
    value.into_repr().to_bytes_le()
}

/// arkworks' `CanonicalDeserialize`, which refuses any 32 bytes that encode an integer at or above
/// the modulus; the length is checked first so trailing bytes are not silently ignored.
pub(crate) fn decode(bytes: &[u8]) -> Result<Scalar, String> {
    if bytes.len() != 32 {
        return Err(format!("a BLS12-381 scalar is 32 bytes, got {}", bytes.len()));
    }
    Scalar::deserialize(bytes)
        .map_err(|why| format!("not a canonical BLS12-381 scalar (at or above the modulus): {why}"))
}
