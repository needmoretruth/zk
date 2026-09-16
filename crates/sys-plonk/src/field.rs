//! The BLS12-381 scalar field as the circuit layer sees it, and the one byte encoding the museum shares.

use core::fmt;

use lambdaworks_math::elliptic_curve::short_weierstrass::curves::bls12_381::default_types::FrElement;
use lambdaworks_math::traits::ByteConversion;
use lambdaworks_math::unsigned_integer::element::U256;
use zk_circuit::ZkField;

/// Bytes of one canonical scalar, in the museum's encoding and in lambdaworks' proof alike.
pub(crate) const SCALAR_BYTES: usize = 32;

/// The BLS12-381 scalar field, wrapped so it can implement [`ZkField`].
///
/// PLONK's wires, selectors and KZG commitments all live over this field, so the seven statements
/// are built over it. The orphan rule forbids implementing the museum's trait on lambdaworks'
/// `FieldElement` directly, and that type is not `Copy` (its derive demands `Copy` of the field
/// configuration marker), so the wrapper keeps the element's internal limbs, which are.
#[derive(Clone, Copy, Eq)]
pub struct Fr(U256);

impl Fr {
    /// Wraps a lambdaworks scalar.
    pub fn new(value: &FrElement) -> Self {
        Self(*value.value())
    }

    /// The lambdaworks scalar, for handing values to its prover.
    pub fn element(self) -> FrElement {
        FrElement::from_raw(self.0)
    }
}

impl PartialEq for Fr {
    fn eq(&self, other: &Self) -> bool {
        self.element() == other.element()
    }
}

impl fmt::Debug for Fr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fr({:?})", self.element().canonical())
    }
}

impl ZkField for Fr {
    /// The scalar modulus `r` of BLS12-381 is a 255-bit prime.
    const MODULUS_BITS: u32 = 255;
    const NAME: &'static str = "BLS12-381 scalar";

    fn zero() -> Self {
        Self::new(&FrElement::zero())
    }
    fn one() -> Self {
        Self::new(&FrElement::one())
    }
    fn from_u64(value: u64) -> Self {
        Self::new(&FrElement::from(value))
    }
    fn add(self, rhs: Self) -> Self {
        Self::new(&(self.element() + rhs.element()))
    }
    fn sub(self, rhs: Self) -> Self {
        Self::new(&(self.element() - rhs.element()))
    }
    fn mul(self, rhs: Self) -> Self {
        Self::new(&(self.element() * rhs.element()))
    }
    fn neg(self) -> Self {
        Self::new(&-self.element())
    }
    fn inverse(self) -> Option<Self> {
        self.element().inv().ok().map(|inverse| Self::new(&inverse))
    }
    fn to_le_bytes(self) -> Vec<u8> {
        encode(&self.element())
    }
}

/// Canonical 32-byte little-endian form. lambdaworks converts out of Montgomery form before writing.
pub(crate) fn encode(value: &FrElement) -> Vec<u8> {
    value.to_bytes_le()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value at or above the modulus.
///
/// lambdaworks' own `from_bytes_le` silently reduces an oversized integer, so the result is encoded
/// again and compared: only the one canonical encoding of each scalar is accepted.
pub(crate) fn decode(bytes: &[u8]) -> Option<FrElement> {
    if bytes.len() != SCALAR_BYTES {
        return None;
    }
    let value = FrElement::from_bytes_le(bytes).ok()?;
    (encode(&value) == bytes).then_some(value)
}
