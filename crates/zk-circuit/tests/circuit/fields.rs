//! Real fields for tests: the BN254 scalar field (254 bits) and BabyBear (31 bits), at the two ends
//! of the sizes the museum uses.
//!
//! The zk-examples tests include this file through `#[path]`, so both crates are tested on the
//! same field glue without a third crate.

use ark_ff::{AdditiveGroup, BigInteger, Field as _, PrimeField};
use p3_field::{Field as _, PrimeCharacteristicRing, PrimeField32};
use zk_circuit::ZkField;

/// The BN254 scalar field, as used by Groth16 on BN254.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bn254(pub ark_bn254::Fr);

impl ZkField for Bn254 {
    const MODULUS_BITS: u32 = ark_bn254::Fr::MODULUS_BIT_SIZE;
    const NAME: &'static str = "BN254 scalar field";

    fn zero() -> Self {
        Self(ark_bn254::Fr::ZERO)
    }
    fn one() -> Self {
        Self(ark_bn254::Fr::ONE)
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
        self.0.into_bigint().to_bytes_le()
    }
}

/// BabyBear, `p = 15·2^27 + 1`, as used by Plonky3 and RISC Zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BabyBear(pub p3_baby_bear::BabyBear);

impl ZkField for BabyBear {
    const MODULUS_BITS: u32 = 31;
    const NAME: &'static str = "BabyBear";

    fn zero() -> Self {
        Self(p3_baby_bear::BabyBear::ZERO)
    }
    fn one() -> Self {
        Self(p3_baby_bear::BabyBear::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(p3_baby_bear::BabyBear::from_u64(value))
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
        self.0.as_canonical_u32().to_le_bytes().to_vec()
    }
}
