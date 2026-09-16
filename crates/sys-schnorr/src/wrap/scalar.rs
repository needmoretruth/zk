//! `ff`'s field traits on [`PallasScalar`], each forwarded to `pasta_curves`.

use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use ff::{Field, PrimeField};
use pasta_curves::pallas;
use rand_core::RngCore;
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};

use crate::field::PallasScalar;

type Inner = pallas::Scalar;

/// Forwards one binary operator, by value and by reference, and its assigning form.
macro_rules! forward_binary {
    ($Op:ident, $op:ident, $OpAssign:ident, $op_assign:ident) => {
        impl $Op for PallasScalar {
            type Output = Self;
            fn $op(self, rhs: Self) -> Self {
                Self($Op::$op(self.0, rhs.0))
            }
        }
        impl<'a> $Op<&'a PallasScalar> for PallasScalar {
            type Output = Self;
            fn $op(self, rhs: &'a Self) -> Self {
                Self($Op::$op(self.0, rhs.0))
            }
        }
        impl $OpAssign for PallasScalar {
            fn $op_assign(&mut self, rhs: Self) {
                $OpAssign::$op_assign(&mut self.0, rhs.0);
            }
        }
        impl<'a> $OpAssign<&'a PallasScalar> for PallasScalar {
            fn $op_assign(&mut self, rhs: &'a Self) {
                $OpAssign::$op_assign(&mut self.0, rhs.0);
            }
        }
    };
}

forward_binary!(Add, add, AddAssign, add_assign);
forward_binary!(Sub, sub, SubAssign, sub_assign);
forward_binary!(Mul, mul, MulAssign, mul_assign);

impl Neg for PallasScalar {
    type Output = Self;
    fn neg(self) -> Self {
        Self(Neg::neg(self.0))
    }
}

impl Sum for PallasScalar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|value| value.0).sum())
    }
}

impl<'a> Sum<&'a PallasScalar> for PallasScalar {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|value| value.0).sum())
    }
}

impl Product for PallasScalar {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|value| value.0).product())
    }
}

impl<'a> Product<&'a PallasScalar> for PallasScalar {
    fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|value| value.0).product())
    }
}

impl ConditionallySelectable for PallasScalar {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        Self(Inner::conditional_select(&a.0, &b.0, choice))
    }
}

impl ConstantTimeEq for PallasScalar {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}

impl From<u64> for PallasScalar {
    fn from(value: u64) -> Self {
        Self(Inner::from(value))
    }
}

impl Field for PallasScalar {
    const ZERO: Self = Self(Inner::ZERO);
    const ONE: Self = Self(Inner::ONE);

    fn random(rng: impl RngCore) -> Self {
        Self(<Inner as Field>::random(rng))
    }
    fn square(&self) -> Self {
        Self(Field::square(&self.0))
    }
    fn double(&self) -> Self {
        Self(Field::double(&self.0))
    }
    fn invert(&self) -> CtOption<Self> {
        Field::invert(&self.0).map(Self)
    }
    fn sqrt_ratio(num: &Self, div: &Self) -> (Choice, Self) {
        let (is_square, root) = Inner::sqrt_ratio(&num.0, &div.0);
        (is_square, Self(root))
    }
    fn sqrt(&self) -> CtOption<Self> {
        Field::sqrt(&self.0).map(Self)
    }
}

impl PrimeField for PallasScalar {
    type Repr = <Inner as PrimeField>::Repr;

    fn from_repr(repr: Self::Repr) -> CtOption<Self> {
        Inner::from_repr(repr).map(Self)
    }
    fn to_repr(&self) -> Self::Repr {
        self.0.to_repr()
    }
    fn is_odd(&self) -> Choice {
        self.0.is_odd()
    }

    const MODULUS: &'static str = <Inner as PrimeField>::MODULUS;
    const NUM_BITS: u32 = <Inner as PrimeField>::NUM_BITS;
    const CAPACITY: u32 = <Inner as PrimeField>::CAPACITY;
    const TWO_INV: Self = Self(<Inner as PrimeField>::TWO_INV);
    const MULTIPLICATIVE_GENERATOR: Self = Self(<Inner as PrimeField>::MULTIPLICATIVE_GENERATOR);
    const S: u32 = <Inner as PrimeField>::S;
    const ROOT_OF_UNITY: Self = Self(<Inner as PrimeField>::ROOT_OF_UNITY);
    const ROOT_OF_UNITY_INV: Self = Self(<Inner as PrimeField>::ROOT_OF_UNITY_INV);
    const DELTA: Self = Self(<Inner as PrimeField>::DELTA);
}
