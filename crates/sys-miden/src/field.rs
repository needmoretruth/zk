//! Goldilocks as the circuit layer sees it, and the byte form the museum and Miden use for it.

use miden_core::Felt;
use miden_core::field::{Field as _, PrimeCharacteristicRing};
use zk_circuit::ZkField;

/// Goldilocks, `p = 2^64 − 2^32 + 1`, wrapped so it can implement [`ZkField`].
///
/// Miden VM computes over this field: every stack element, memory cell and trace cell is one
/// `Felt`. The statements are built over the very field the VM runs on, so a wire value is a VM
/// value with no conversion. The orphan rule forbids implementing the museum's trait on
/// `miden_core::Felt` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goldilocks(pub Felt);

impl ZkField for Goldilocks {
    const MODULUS_BITS: u32 = 64;
    const NAME: &'static str = "Goldilocks";

    fn zero() -> Self {
        Self(Felt::ZERO)
    }
    fn one() -> Self {
        Self(Felt::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(<Felt as PrimeCharacteristicRing>::from_u64(value))
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
        encode(self.0)
    }
}

/// The museum's shared form: the canonical representative in `0..p` as 8 little-endian bytes.
///
/// It is also the form Miden writes a field element in, inside both the proof and the program, so
/// the harness's default search patterns already cover a secret written out in a proof.
pub(crate) fn encode(value: Felt) -> Vec<u8> {
    value.as_canonical_u64().to_le_bytes().to_vec()
}

/// Reads [`encode`]'s form back; `None` for the wrong length or a value not below the modulus, so
/// a caller can report undecodable input instead of silently reducing it.
pub(crate) fn decode(bytes: &[u8]) -> Option<Felt> {
    let array: [u8; 8] = bytes.try_into().ok()?;
    Felt::new(u64::from_le_bytes(array)).ok()
}

/// Renders a value the way Miden Assembly accepts it as an immediate: its canonical integer.
pub(crate) fn literal(value: Goldilocks) -> String {
    value.0.as_canonical_u64().to_string()
}
