//! Pedersen commitments on Pallas: the two generators and `C = v·G + r·H`.

use group::Group;
use pasta_curves::arithmetic::CurveExt;
use pasta_curves::pallas;

use crate::field::PallasScalar;
use crate::wrap::PallasPoint;

/// The domain every derived value of this exhibit hangs off: the second generator `H` and the
/// session identifiers its proofs are bound to.
pub const DOMAIN: &str = "zk/schnorr/v1";

/// A committed value and the blinding that hides it; only the prover ever holds one.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Opening {
    /// The value committed to.
    pub(crate) value: PallasScalar,
    /// The random multiple of `H` that hides it.
    pub(crate) blinding: PallasScalar,
}

/// `G`, Pallas' standard generator, and `H`, hashed to the curve.
///
/// A commitment binds only if nobody knows `log_G H`. A point hashed to the curve from a fixed
/// domain string has no known discrete logarithm, not even for whoever chose the string.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Generators {
    /// Carries the committed value.
    pub(crate) g: PallasPoint,
    /// Carries the blinding.
    pub(crate) h: PallasPoint,
}

impl Generators {
    /// Derives both generators; `H` costs one hash to the curve, so a prepared statement keeps them.
    pub(crate) fn derive() -> Self {
        let h = pallas::Point::hash_to_curve(DOMAIN)(b"H");
        Self { g: PallasPoint::generator(), h: PallasPoint(h) }
    }

    /// `value·G + blinding·H`.
    pub(crate) fn commit(&self, opening: Opening) -> PallasPoint {
        self.g * opening.value + self.h * opening.blinding
    }
}
