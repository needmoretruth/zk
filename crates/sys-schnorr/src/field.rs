//! The Pallas scalar field as the circuit layer sees it, and the one byte encoding the museum shares.

use ff::{Field as _, PrimeField};
use pasta_curves::pallas;
use zk_circuit::ZkField;

/// An element of the Pallas scalar field, the exponents of Pallas points, wrapped for foreign traits.
///
/// Every committed value is an exponent of the generators, so the statements are built over this
/// field. The orphan rule forbids implementing the museum's [`ZkField`], or the `ff`, `group` and
/// spongefish traits `sigma-proofs` needs, on `pasta_curves`' own type, so this one newtype carries
/// all of them (the others live in [`crate::wrap`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PallasScalar(pub pallas::Scalar);

impl ZkField for PallasScalar {
    const MODULUS_BITS: u32 = <pallas::Scalar as PrimeField>::NUM_BITS;
    const NAME: &'static str = "Pallas scalar";

    fn zero() -> Self {
        Self(pallas::Scalar::ZERO)
    }
    fn one() -> Self {
        Self(pallas::Scalar::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(pallas::Scalar::from(value))
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
        encode(self)
    }
}

/// Canonical 32-byte little-endian form, the museum's shared encoding of a public input.
pub(crate) fn encode(value: PallasScalar) -> Vec<u8> {
    value.0.to_repr().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value not below the modulus, so a
/// caller can report undecodable input instead of silently reducing it.
pub(crate) fn decode(bytes: &[u8]) -> Option<PallasScalar> {
    let repr: [u8; 32] = bytes.try_into().ok()?;
    Option::from(pallas::Scalar::from_repr(repr)).map(PallasScalar)
}
