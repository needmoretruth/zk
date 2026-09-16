//! The Goldilocks field, as the circuit layer sees it, and its 8-byte encoding.

use p3_field::integers::QuotientMap;
use p3_field::{Field, PrimeCharacteristicRing, PrimeField64};
use p3_goldilocks::Goldilocks;
use zk_circuit::ZkField;

/// `p = 2^64 − 2^32 + 1`. Trio runs over Goldilocks because 64-bit shares are cheap to compute
/// and a random share of a 64-bit field hides a small secret as well as a large field would.
pub const MODULUS: u64 = Goldilocks::ORDER_U64;

/// Bytes in one encoded field element: the canonical representative, little-endian.
pub const ELEMENT_BYTES: usize = 8;

/// An element of the Goldilocks field.
///
/// The museum's statements are written against [`ZkField`]; the orphan rule forbids implementing
/// that trait on `p3_goldilocks::Goldilocks` directly, so this newtype carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fp(pub Goldilocks);

impl ZkField for Fp {
    const MODULUS_BITS: u32 = 64;
    const NAME: &'static str = "Goldilocks";

    fn zero() -> Self {
        Self(Goldilocks::ZERO)
    }
    fn one() -> Self {
        Self(Goldilocks::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(Goldilocks::from_u64(value))
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
        self.encode().to_vec()
    }
}

impl Fp {
    /// The element whose canonical representative is `value`, or `None` at or above the modulus;
    /// the only way bytes from a seed or a proof become an element, so no value has two encodings.
    pub fn from_canonical(value: u64) -> Option<Self> {
        <Goldilocks as QuotientMap<u64>>::from_canonical_checked(value).map(Self)
    }

    /// The canonical representative in `0..p`.
    pub fn to_canonical(self) -> u64 {
        self.0.as_canonical_u64()
    }

    /// Canonical little-endian bytes, the encoding every hash and every proof uses.
    pub fn encode(self) -> [u8; ELEMENT_BYTES] {
        self.to_canonical().to_le_bytes()
    }

    /// Reads [`Fp::encode`] back, refusing a wrong length or an integer at or above the modulus.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let array: [u8; ELEMENT_BYTES] = bytes
            .try_into()
            .map_err(|_| format!("a Goldilocks element is 8 bytes, got {}", bytes.len()))?;
        Self::from_canonical(u64::from_le_bytes(array)).ok_or_else(|| {
            "not a canonical Goldilocks element (at or above the modulus)".to_string()
        })
    }
}
