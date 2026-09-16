//! The circuit layer's [`ZkField`] on top of the BN254 scalar field.
//!
//! BCTV14 proves over the scalar field of the BN254 pairing-friendly curve — the very curve libff
//! sampled for the paper and the one Zcash Sprout used. The circuit layer speaks its own small
//! [`ZkField`] trait, so the orphan rule forces a newtype: [`Bn254Fr`] wraps [`ark_bn254::Fr`] and
//! forwards every operation.

use ark_bn254::Fr;
use ark_ff::{AdditiveGroup, BigInteger, Field, PrimeField};
use zk_circuit::ZkField;

/// The BN254 scalar field as the circuit layer sees it.
///
/// A transparent newtype: it exists only so [`ZkField`] can be implemented on a local type, and
/// [`Bn254Fr::inner`] hands the underlying element back to the arkworks code that does the real
/// arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bn254Fr(pub Fr);

impl Bn254Fr {
    /// The underlying arkworks element, for the SNARK code that computes over it directly.
    pub fn inner(self) -> Fr {
        self.0
    }
}

impl ZkField for Bn254Fr {
    const MODULUS_BITS: u32 = Fr::MODULUS_BIT_SIZE;
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
        self.0.inverse().map(Self)
    }
    fn to_le_bytes(self) -> Vec<u8> {
        self.0.into_bigint().to_bytes_le()
    }
}

/// Reads a field element from its canonical little-endian encoding, the museum's shared form for a
/// public input. Returns `None` when the bytes are not the canonical representative of an element
/// (too long, or numerically `>= p`), so the verifier can report malformed input rather than
/// silently accepting a non-canonical encoding.
pub fn fr_from_canonical_le(bytes: &[u8]) -> Option<Fr> {
    // The canonical encoding is exactly `ceil(MODULUS_BITS / 8) = 32` bytes.
    if bytes.len() != 32 {
        return None;
    }
    let element = Fr::from_le_bytes_mod_order(bytes);
    // Reject any encoding that is not already reduced: re-encoding must reproduce the input.
    if element.into_bigint().to_bytes_le() == bytes { Some(element) } else { None }
}

/// Canonical little-endian encoding of an element, matching [`Bn254Fr::to_le_bytes`].
pub fn fr_to_canonical_le(element: Fr) -> Vec<u8> {
    element.into_bigint().to_bytes_le()
}
