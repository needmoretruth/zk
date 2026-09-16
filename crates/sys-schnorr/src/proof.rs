//! The proof object — published commitments, then `sigma-proofs`' own bytes — and what it is bound to.

use group::GroupEncoding;
use spongefish::NargDeserialize;
use zk_core::ExampleId;

use crate::field::{PallasScalar, encode};
use crate::pedersen::DOMAIN;
use crate::wrap::PallasPoint;

/// Bytes of one compressed point or one scalar.
pub(crate) const ELEMENT_BYTES: usize = 32;

/// The commitments in layout order followed by the upstream proof.
pub(crate) fn assemble(commitments: &[PallasPoint], upstream: &[u8]) -> Vec<u8> {
    let mut proof = Vec::with_capacity(commitments.len() * ELEMENT_BYTES + upstream.len());
    for commitment in commitments {
        proof.extend_from_slice(&commitment.to_bytes());
    }
    proof.extend_from_slice(upstream);
    proof
}

/// Reads `count` commitments off the front; the rest is the upstream proof.
pub(crate) fn split(proof: &[u8], count: usize) -> Result<(Vec<PallasPoint>, &[u8]), String> {
    let mut rest = proof;
    let mut commitments = Vec::with_capacity(count);
    for index in 0..count {
        let point = PallasPoint::deserialize_from_narg(&mut rest)
            .map_err(|_| format!("commitment {index} is not a Pallas point"))?;
        commitments.push(point);
    }
    Ok((commitments, rest))
}

/// Checks the upstream bytes hold exactly `points` points then `scalars` canonical scalars, the shape
/// `sigma-proofs`' batchable proof has, so undecodable bytes are reported as such.
pub(crate) fn check_upstream_shape(
    bytes: &[u8],
    points: usize,
    scalars: usize,
) -> Result<(), String> {
    let mut rest = bytes;
    for index in 0..points {
        PallasPoint::deserialize_from_narg(&mut rest)
            .map_err(|_| format!("sigma commitment {index} is not a Pallas point"))?;
    }
    for index in 0..scalars {
        PallasScalar::deserialize_from_narg(&mut rest)
            .map_err(|_| format!("response {index} is not a canonical Pallas scalar"))?;
    }
    if rest.is_empty() { Ok(()) } else { Err(format!("{} trailing bytes", rest.len())) }
}

/// The session identifier handed to `sigma-proofs`: the domain, the example's ID, the public inputs
/// and every published commitment.
///
/// `sigma-proofs` hashes it into the challenge, so a proof for one example, one set of public inputs
/// or one set of commitments convinces no verifier of another — including commitments that no
/// equation happens to use.
pub(crate) fn session(
    example: ExampleId,
    public: &[PallasScalar],
    commitments: &[PallasPoint],
) -> Vec<u8> {
    let mut session = Vec::new();
    let mut field = |bytes: &[u8]| {
        session.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        session.extend_from_slice(bytes);
    };
    field(DOMAIN.as_bytes());
    field(example.id().as_bytes());
    field(&public.iter().flat_map(|value| encode(*value)).collect::<Vec<u8>>());
    field(&commitments.iter().flat_map(|point| point.to_bytes()).collect::<Vec<u8>>());
    session
}
