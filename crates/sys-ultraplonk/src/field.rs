//! The BN254 scalar field as the circuit layer sees it, and the one byte encoding the museum shares.

use ark_ff::{BigInt, BigInteger, Field as _, One, PrimeField, Zero};
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// The scalar field of BN254, the curve Aztec's UltraPlonk proofs were made over.
///
/// The museum's statements are written against [`ZkField`]; the orphan rule forbids implementing
/// that trait on arkworks' `ark_bn254::Fr` directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fr(pub ark_bn254::Fr);

impl ZkField for Fr {
    /// The scalar modulus of BN254 is a 254-bit prime.
    const MODULUS_BITS: u32 = <ark_bn254::Fr as PrimeField>::MODULUS_BIT_SIZE;
    const NAME: &'static str = "BN254 scalar";

    fn zero() -> Self {
        Self(ark_bn254::Fr::zero())
    }
    fn one() -> Self {
        Self(ark_bn254::Fr::one())
    }
    fn from_u64(value: u64) -> Self {
        Self(ark_bn254::Fr::from(value))
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
        self.0.inverse().map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.0)
    }
}

/// The canonical representative as 32 little-endian bytes, which is also how arkworks writes a
/// scalar into a proof (it leaves Montgomery form before serializing).
pub(crate) fn encode(value: &ark_bn254::Fr) -> FieldBytes {
    value.into_bigint().to_bytes_le()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or an integer at or above the modulus.
pub(crate) fn decode(bytes: &[u8]) -> Option<ark_bn254::Fr> {
    let bytes: &[u8; 32] = bytes.try_into().ok()?;
    let (chunks, _) = bytes.as_chunks::<8>();
    let mut limbs = [0u64; 4];
    for (limb, chunk) in limbs.iter_mut().zip(chunks) {
        *limb = u64::from_le_bytes(*chunk);
    }
    ark_bn254::Fr::from_bigint(BigInt::new(limbs))
}
