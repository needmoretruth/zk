//! The BLS12-381 scalar field under the circuit layer's trait, so `one-plus-one` can be built here.

use bls12_381::Scalar;
use ff::{Field, PrimeField};
use zk_circuit::ZkField;

/// The BLS12-381 scalar field as the museum's circuits see it.
///
/// The ceremony makes Groth16 keys for the museum's own `one-plus-one` circuit, which is written
/// against [`ZkField`]; the orphan rule forbids implementing that trait on `bls12_381::Scalar`, so
/// this newtype carries it. It is the field and encoding of the Groth16 exhibit, repeated here so
/// the ceremony does not depend on another exhibit's internals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CircuitField(pub(crate) Scalar);

impl ZkField for CircuitField {
    const MODULUS_BITS: u32 = Scalar::NUM_BITS;
    const NAME: &'static str = "BLS12-381 scalar field";

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
        self.0.to_repr().to_vec()
    }
}
