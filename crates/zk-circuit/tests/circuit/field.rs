use ark_ff::{AdditiveGroup, BigInteger, Field as _, Fp64, MontBackend, MontConfig, PrimeField};
use zk_circuit::{CircuitBuilder, CircuitError, ZkField, check_field, to_u64};

use crate::fields::{BabyBear, Bn254};

#[derive(MontConfig)]
#[modulus = "65521"]
#[generator = "17"]
pub struct TinyConfig;

/// A 16-bit prime field that reports `CLAIMED_BITS` as its modulus size, so one type tests both
/// an honest small field and one that lies about its size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tiny<const CLAIMED_BITS: u32>(Fp64<MontBackend<TinyConfig, 1>>);

impl<const CLAIMED_BITS: u32> ZkField for Tiny<CLAIMED_BITS> {
    const MODULUS_BITS: u32 = CLAIMED_BITS;
    const NAME: &'static str = "tiny";

    fn zero() -> Self {
        Self(AdditiveGroup::ZERO)
    }
    fn one() -> Self {
        Self(ark_ff::Field::ONE)
    }
    fn from_u64(value: u64) -> Self {
        Self(value.into())
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

#[test]
fn bn254_and_babybear_are_accepted() {
    check_field::<Bn254>().unwrap();
    check_field::<BabyBear>().unwrap();
}

#[test]
fn a_16_bit_field_is_rejected_when_building() {
    assert_eq!(
        CircuitBuilder::<Tiny<16>>::new().err(),
        Some(CircuitError::FieldTooSmall { field: "tiny", modulus_bits: 16 })
    );
}

#[test]
fn a_16_bit_field_claiming_64_bits_is_rejected() {
    assert_eq!(
        check_field::<Tiny<64>>(),
        Err(CircuitError::FieldTooSmall { field: "tiny", modulus_bits: 64 })
    );
}

#[test]
fn to_u64_reads_values_that_fit_and_refuses_the_rest() {
    assert_eq!(to_u64(Bn254::from_u64(u64::MAX)), Some(u64::MAX));
    assert_eq!(to_u64(Bn254::one().neg()), None);
    assert_eq!(to_u64(BabyBear::one().neg()), Some(15 * (1 << 27)));
}
