//! The Pasta field as the circuit layer sees it, and the one byte encoding the museum shares.

use ff::{Field, PrimeField};
use halo2_proofs::pasta::Fp;
use zk_circuit::ZkField;

/// The Pallas base field (the Vesta scalar field), wrapped so it can implement [`ZkField`].
///
/// Orchard's circuit is written over this field and committed to with Vesta points, so the seven
/// statements are built over it here too. The orphan rule forbids implementing the museum's trait
/// on `pasta_curves::Fp` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PastaFp(pub Fp);

impl ZkField for PastaFp {
    const MODULUS_BITS: u32 = Fp::NUM_BITS;
    const NAME: &'static str = "Pasta Fp";

    fn zero() -> Self {
        Self(Fp::ZERO)
    }
    fn one() -> Self {
        Self(Fp::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(Fp::from(value))
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
        encode(self.0)
    }
}

/// Canonical 32-byte little-endian form, which is also what Halo 2's transcript writes for a scalar.
pub(crate) fn encode(value: Fp) -> Vec<u8> {
    value.to_repr().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value not below the modulus, so a
/// caller can report undecodable input instead of silently reducing it.
pub(crate) fn decode(bytes: &[u8]) -> Option<Fp> {
    let repr: [u8; 32] = bytes.try_into().ok()?;
    Option::from(Fp::from_repr(repr))
}
