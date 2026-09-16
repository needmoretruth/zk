//! Schnorr proofs and sigma protocols, the family behind Zcash's spend authorization, as an exhibit.
//!
//! Since Sapling, every Zcash spend carries a RedDSA (Orchard: RedPallas) signature, which is a
//! Schnorr proof of knowledge of a discrete logarithm made non-interactive. [`schnorr`] runs that
//! proof with `sigma-proofs` on Pallas.
//!
//! A Schnorr proof alone proves no circuit, so the exhibit adds a teaching layer that compiles each
//! of the museum's statements into one AND of linear relations over Pedersen commitments
//! (`layout`, `relation`): every private input, hint output and product is committed to, linear
//! gates follow from the commitments' homomorphism, each product is a small linear proof, and each
//! assertion says a commitment holds zero. `sigma-proofs` then does all the proving, challenge
//! derivation and verifying (`prove`, `verify`); this crate writes no sigma protocol of its own.
//! The compiler is not audited, and proofs grow linearly with the circuit.

mod affine;
mod field;
mod layout;
mod meta;
mod pedersen;
mod prepared;
mod proof;
mod prove;
mod relation;
pub mod schnorr;
mod verify;
mod wrap;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::PallasScalar;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = PallasScalar;
pub use meta::META;
pub use pedersen::DOMAIN;

/// The Schnorr and sigma-protocol exhibit; stateless, since a sigma protocol has no setup to keep.
#[derive(Clone, Copy, Debug, Default)]
pub struct Schnorr;

impl ProofSystem for Schnorr {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        Ok(Box::new(prepared::Ready::build(example, control)?))
    }
}
