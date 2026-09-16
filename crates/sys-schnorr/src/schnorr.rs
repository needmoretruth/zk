//! The textbook Schnorr proof: knowledge of `x` with `X = x·G`, the core of a RedPallas signature.
//!
//! Orchard authorizes every spend with a RedPallas signature, and a Schnorr signature is exactly this
//! proof made non-interactive, with the message hashed into the challenge. Here `sigma-proofs`
//! proves the bare relation on Pallas. The 64-byte proof is the commitment `R` followed by the
//! response `s`, the same two parts as a RedPallas signature; the bytes are not interchangeable,
//! because RedPallas hashes a message and its own domain with BLAKE2b where `sigma-proofs` uses its
//! own sponge.

use group::{Group, GroupEncoding};
use rand_core::OsRng;
use sigma_proofs::LinearRelation;
use zk_core::SystemError;

use crate::field::PallasScalar;
use crate::pedersen::DOMAIN;
use crate::wrap::PallasPoint;

/// The session identifier every discrete-logarithm proof here is bound to.
fn session() -> Vec<u8> {
    format!("{DOMAIN}/dlog").into_bytes()
}

/// `X = x·G` with `x` secret and `X` given.
fn relation(public: PallasPoint) -> LinearRelation<PallasPoint> {
    let mut relation = LinearRelation::new();
    let x = relation.allocate_scalar();
    let g = relation.allocate_element_with(PallasPoint::generator());
    let image = relation.allocate_eq(x * g);
    relation.set_element(image, public);
    relation
}

/// Proves knowledge of `secret`; returns `X = secret·G` in Pallas' 32-byte compressed form and the
/// 64-byte proof.
///
/// A zero secret is an error: `X` is then the identity, and `sigma-proofs` refuses a relation whose
/// image is already explained by the zero witness (its "trivial kernel" check).
pub fn prove_dlog(secret: PallasScalar) -> Result<(Vec<u8>, Vec<u8>), SystemError> {
    let public = PallasPoint::generator() * secret;
    let refused = |error: sigma_proofs::errors::Error| SystemError::Unsatisfied(error.to_string());
    let nizk = relation(public).into_nizk(&session()).map_err(refused)?;
    let proof = nizk.prove_batchable(&vec![secret], &mut OsRng).map_err(refused)?;
    Ok((public.to_bytes().to_vec(), proof))
}

/// Whether `proof` shows knowledge of the discrete logarithm of `public`; bytes that are not a point
/// or not a proof simply fail.
pub fn verify_dlog(public: &[u8], proof: &[u8]) -> bool {
    let Ok(repr) = <[u8; 32]>::try_from(public) else { return false };
    let Some(point) = Option::<PallasPoint>::from(PallasPoint::from_bytes(&repr)) else {
        return false;
    };
    relation(point).into_nizk(&session()).is_ok_and(|nizk| nizk.verify_batchable(proof).is_ok())
}
