//! The ristretto255 scalar field, as the circuit layer sees it, and the encodings Spartan uses.

use curve25519_dalek::scalar::Scalar;
use zk_circuit::ZkField;
use zk_core::FieldBytes;

/// Bytes in one scalar, and in one compressed Ristretto point.
pub(crate) const ELEMENT_BYTES: usize = 32;

/// The scalar field of ristretto255, the prime-order group Spartan's commitments live in.
///
/// Its order is `ℓ = 2^252 + 27742317777372353535851937790883648493`. Spartan computes with its own
/// Montgomery-form scalar, but keeps that type in a private module and takes every value as canonical
/// little-endian bytes; its example builds those bytes with `curve25519_dalek::scalar::Scalar`, the
/// same field, and so does the museum. The orphan rule forbids implementing [`ZkField`] on that
/// foreign type directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RistrettoScalar(pub Scalar);

impl ZkField for RistrettoScalar {
    const MODULUS_BITS: u32 = 253;
    const NAME: &'static str = "ristretto255 scalar field";

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
    /// `Scalar::invert` returns zero for zero instead of failing, so zero is answered here.
    fn inverse(self) -> Option<Self> {
        (self.0 != Scalar::ZERO).then(|| Self(self.0.invert()))
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.0)
    }
}

/// `Scalar::to_bytes`: the canonical representative as 32 little-endian bytes, the form Spartan's
/// `Instance::new` and `Assignment::new` take.
pub(crate) fn encode(value: &Scalar) -> FieldBytes {
    value.to_bytes().to_vec()
}

/// `Scalar::from_canonical_bytes`, which refuses any 32 bytes that encode an integer at or above `ℓ`.
pub(crate) fn decode(bytes: &[u8]) -> Result<Scalar, String> {
    let repr: [u8; ELEMENT_BYTES] = bytes
        .try_into()
        .map_err(|_| format!("a ristretto255 scalar is 32 bytes, got {}", bytes.len()))?;
    Option::from(Scalar::from_canonical_bytes(repr))
        .ok_or_else(|| "not a canonical ristretto255 scalar (at or above the group order)".into())
}

/// The bytes Spartan's serializer writes for `value`: its Montgomery form `value · 2^256 mod ℓ`.
///
/// Spartan's scalar is a `[u64; 4]` holding the Montgomery form and derives `Serialize`, so bincode
/// writes those four limbs little-endian: 32 bytes that differ from the canonical encoding. A secret
/// sitting verbatim in a proof would look like this, so the secret scan searches for it too.
pub(crate) fn montgomery_bytes(value: &Scalar) -> FieldBytes {
    let mut radix = [0u8; 64];
    radix[ELEMENT_BYTES] = 1;
    encode(&(value * Scalar::from_bytes_mod_order_wide(&radix)))
}
