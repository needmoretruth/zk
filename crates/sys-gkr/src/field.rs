//! The BN254 scalar field, as the circuit layer sees it, and the one encoding Remainder uses for it.

use shared_types::Fr;
use shared_types::halo2curves::ff::{Field as _, PrimeField as _};
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// Bytes in one scalar, and in one compressed BN254 point.
pub(crate) const ELEMENT_BYTES: usize = 32;

/// The scalar field of BN254, the field Remainder's circuits compute in and the exponent group of
/// the curve its Hyrax commitments live on.
///
/// Remainder computes with halo2curves' `Fr`, re-exported by its `shared-types` crate, so the museum
/// builds its statements over that same type. The orphan rule forbids implementing [`ZkField`] on
/// the foreign type directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bn254Scalar(pub Fr);

impl ZkField for Bn254Scalar {
    const MODULUS_BITS: u32 = Fr::NUM_BITS;
    const NAME: &'static str = "BN254 scalar field";

    fn zero() -> Self {
        Self(Fr::ZERO)
    }
    fn one() -> Self {
        Self(Fr::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(Fr::from(value))
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

/// `Fr::to_repr`: the canonical representative as 32 little-endian bytes. It is also exactly what
/// halo2curves' serde implementation writes, so a scalar inside a Remainder proof looks like this.
pub(crate) fn encode(value: &Fr) -> FieldBytes {
    value.to_repr().to_vec()
}

/// `Fr::from_repr`, which refuses any 32 bytes that encode an integer at or above the modulus.
pub(crate) fn decode(bytes: &[u8]) -> Result<Fr, String> {
    let repr: [u8; ELEMENT_BYTES] =
        bytes.try_into().map_err(|_| format!("a BN254 scalar is 32 bytes, got {}", bytes.len()))?;
    Option::from(Fr::from_repr(repr))
        .ok_or_else(|| "not a canonical BN254 scalar (at or above the modulus)".into())
}

/// Decodes a list of inputs, refusing a list of the wrong length before any element is read.
pub(crate) fn decode_all(values: &[FieldBytes], expected: usize) -> Result<Vec<Fr>, String> {
    if values.len() != expected {
        return Err(format!("expected {expected} public inputs, got {}", values.len()));
    }
    values.iter().map(|bytes| decode(bytes)).collect()
}
