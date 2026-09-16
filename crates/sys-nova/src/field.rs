//! The Pallas scalar field, as the circuit layer sees it, and the one byte encoding the museum shares.

use ff::{Field as _, PrimeField};
use nova_snark::provider::pasta::pallas;
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// The primary engine's scalar field, in which Nova's step function is written.
pub(crate) type Scalar = pallas::Scalar;

/// Bytes in one canonical scalar.
pub(crate) const SCALAR_BYTES: usize = 32;

/// The scalar field of the Pallas curve (the Vesta base field), wrapped so it can implement
/// [`ZkField`].
///
/// `PallasEngine` is Nova's primary engine here, so the step circuit, the public state `z` and every
/// wire live in this field. The orphan rule forbids implementing the museum's trait on
/// `halo2curves::pasta::Fq` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PallasScalar(pub Scalar);

impl ZkField for PallasScalar {
    const MODULUS_BITS: u32 = Scalar::NUM_BITS;
    const NAME: &'static str = "Pallas scalar field";

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

/// `to_repr`: the canonical representative as 32 little-endian bytes. halo2curves' serde writes a
/// scalar exactly this way inside a proof, so the default secret patterns already cover it.
pub(crate) fn encode(value: &Scalar) -> FieldBytes {
    value.to_repr().as_ref().to_vec()
}

/// `from_repr`, which refuses the wrong length and any integer at or above the modulus.
pub(crate) fn decode(bytes: &[u8]) -> Result<Scalar, String> {
    if bytes.len() != SCALAR_BYTES {
        return Err(format!("a Pallas scalar is {SCALAR_BYTES} bytes, got {}", bytes.len()));
    }
    let mut repr = <Scalar as PrimeField>::Repr::default();
    repr.as_mut().copy_from_slice(bytes);
    Option::from(Scalar::from_repr(repr))
        .ok_or_else(|| "not a canonical Pallas scalar (at or above the modulus)".to_string())
}
