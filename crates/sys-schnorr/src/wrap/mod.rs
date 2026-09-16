//! Pallas in the shape `sigma-proofs` asks for.
//!
//! `sigma-proofs` works over any `group` 0.13 prime-order group whose points and scalars also carry
//! spongefish's codec traits, which say how a prover message becomes transcript bytes. spongefish
//! ships those codecs for arkworks, BLS12-381, Ristretto, secp256k1, P-256 and Plonky3's fields, but
//! not for `pasta_curves`, and the orphan rule forbids adding them to its types from here (arkworks'
//! Pallas has the codecs but not the `group` traits). So Pallas is wrapped
//! twice — [`crate::PallasScalar`] and [`PallasPoint`] — and every trait is forwarded to
//! `pasta_curves` unchanged. No arithmetic is written here; the wrappers only delegate.

mod codec;
mod point;
mod scalar;

pub use point::PallasPoint;
