//! spongefish's codecs for Pallas: how a sigma-protocol message becomes transcript and proof bytes.
//!
//! The conventions follow spongefish's own drivers for the zkcrypto curves: a scalar is written as
//! its 32 big-endian bytes (I2OSP), a point in the curve's compressed form, and a challenge is drawn
//! by reducing 64 squeezed bytes read big-endian (OS2IP), which keeps it uniform.

use ff::{FromUniformBytes, PrimeField};
use group::GroupEncoding;
use pasta_curves::pallas;
use spongefish::{
    ByteArray, Decoding, Encoding, NargDeserialize, VerificationError, VerificationResult,
};

use crate::field::PallasScalar;
use crate::wrap::PallasPoint;

/// Bytes of one scalar or one compressed point.
const ELEMENT_BYTES: usize = 32;

impl Encoding<[u8]> for PallasScalar {
    fn encode(&self) -> impl AsRef<[u8]> {
        let mut bytes = self.0.to_repr();
        bytes.reverse();
        bytes
    }
}

impl NargDeserialize for PallasScalar {
    /// Refuses a value at or above the modulus, so every scalar has exactly one encoding.
    fn deserialize_from_narg(buf: &mut &[u8]) -> VerificationResult<Self> {
        let (head, tail) = buf.split_first_chunk::<ELEMENT_BYTES>().ok_or(VerificationError)?;
        let mut little_endian = *head;
        little_endian.reverse();
        let value: Option<pallas::Scalar> = pallas::Scalar::from_repr(little_endian).into();
        let value = value.ok_or(VerificationError)?;
        *buf = tail;
        Ok(Self(value))
    }
}

impl Decoding<[u8]> for PallasScalar {
    type Repr = ByteArray<{ 2 * ELEMENT_BYTES }>;

    fn decode(buf: Self::Repr) -> Self {
        let mut little_endian = *buf.as_ref();
        little_endian.reverse();
        Self(pallas::Scalar::from_uniform_bytes(&little_endian))
    }
}

impl Encoding<[u8]> for PallasPoint {
    fn encode(&self) -> impl AsRef<[u8]> {
        self.to_bytes()
    }
}

impl NargDeserialize for PallasPoint {
    /// Refuses bytes that are not a point on the curve.
    fn deserialize_from_narg(buf: &mut &[u8]) -> VerificationResult<Self> {
        let (head, tail) = buf.split_first_chunk::<ELEMENT_BYTES>().ok_or(VerificationError)?;
        let point: Option<Self> = Self::from_bytes(head).into();
        let point = point.ok_or(VerificationError)?;
        *buf = tail;
        Ok(point)
    }
}
