//! A Pallas point with `group`'s traits forwarded to `pasta_curves`, over [`PallasScalar`].

use core::iter::Sum;
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use group::prime::PrimeGroup;
use group::{Group, GroupEncoding};
use pasta_curves::pallas;
use rand_core::RngCore;
use sigma_proofs::MultiScalarMul;
use subtle::{Choice, CtOption};

use crate::field::PallasScalar;

type Inner = pallas::Point;

/// A point on Pallas, the curve Orchard's RedPallas spend-authorization signatures live on.
///
/// Wrapped only so spongefish's codec traits can be implemented on it (see [`crate::wrap`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PallasPoint(pub pallas::Point);

/// Forwards one group operator, by value and by reference, and its assigning form.
macro_rules! forward_group_op {
    ($Op:ident, $op:ident, $OpAssign:ident, $op_assign:ident) => {
        impl $Op for PallasPoint {
            type Output = Self;
            fn $op(self, rhs: Self) -> Self {
                Self($Op::$op(self.0, rhs.0))
            }
        }
        impl<'a> $Op<&'a PallasPoint> for PallasPoint {
            type Output = Self;
            fn $op(self, rhs: &'a Self) -> Self {
                Self($Op::$op(self.0, rhs.0))
            }
        }
        impl $OpAssign for PallasPoint {
            fn $op_assign(&mut self, rhs: Self) {
                $OpAssign::$op_assign(&mut self.0, rhs.0);
            }
        }
        impl<'a> $OpAssign<&'a PallasPoint> for PallasPoint {
            fn $op_assign(&mut self, rhs: &'a Self) {
                $OpAssign::$op_assign(&mut self.0, rhs.0);
            }
        }
    };
}

forward_group_op!(Add, add, AddAssign, add_assign);
forward_group_op!(Sub, sub, SubAssign, sub_assign);

impl Mul<PallasScalar> for PallasPoint {
    type Output = Self;
    fn mul(self, rhs: PallasScalar) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl<'a> Mul<&'a PallasScalar> for PallasPoint {
    type Output = Self;
    fn mul(self, rhs: &'a PallasScalar) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign<PallasScalar> for PallasPoint {
    fn mul_assign(&mut self, rhs: PallasScalar) {
        self.0 *= rhs.0;
    }
}

impl<'a> MulAssign<&'a PallasScalar> for PallasPoint {
    fn mul_assign(&mut self, rhs: &'a PallasScalar) {
        self.0 *= rhs.0;
    }
}

impl Neg for PallasPoint {
    type Output = Self;
    fn neg(self) -> Self {
        Self(Neg::neg(self.0))
    }
}

impl Sum for PallasPoint {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|point| point.0).sum())
    }
}

impl<'a> Sum<&'a PallasPoint> for PallasPoint {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        Self(iter.map(|point| point.0).sum())
    }
}

impl Group for PallasPoint {
    type Scalar = PallasScalar;

    fn random(rng: impl RngCore) -> Self {
        Self(<Inner as Group>::random(rng))
    }
    fn identity() -> Self {
        Self(<Inner as Group>::identity())
    }
    fn generator() -> Self {
        Self(<Inner as Group>::generator())
    }
    fn is_identity(&self) -> Choice {
        Group::is_identity(&self.0)
    }
    fn double(&self) -> Self {
        Self(Group::double(&self.0))
    }
}

/// Pallas' own 32-byte compressed encoding: the x-coordinate with the sign of y in the top bit.
impl GroupEncoding for PallasPoint {
    type Repr = <Inner as GroupEncoding>::Repr;

    fn from_bytes(bytes: &Self::Repr) -> CtOption<Self> {
        Inner::from_bytes(bytes).map(Self)
    }
    fn from_bytes_unchecked(bytes: &Self::Repr) -> CtOption<Self> {
        Inner::from_bytes_unchecked(bytes).map(Self)
    }
    fn to_bytes(&self) -> Self::Repr {
        GroupEncoding::to_bytes(&self.0)
    }
}

impl PrimeGroup for PallasPoint {}

/// `sigma-proofs`' own naive multi-scalar multiplication; `pasta_curves` exposes no faster one.
impl MultiScalarMul for PallasPoint {}
