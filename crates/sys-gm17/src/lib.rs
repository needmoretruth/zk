//! GM17, Groth16's simulation-extractable sibling: the `ark-gm17` crate over BLS12-381.
//!
//! Groth and Maller (CRYPTO 2017) traded a larger key and a slower prover for proofs that cannot be
//! mauled into other valid proofs, which Groth16's can. ZoKrates offered it as a backend. This
//! exhibit runs arkworks' implementation on the museum's seven statements so it can stand next to
//! Groth16: each circuit is lowered to R1CS and replayed through
//! `ark_relations::r1cs::ConstraintSynthesizer`, `generate_random_parameters` runs the per-circuit
//! setup, `create_random_proof` proves with fresh OS randomness, and `verify_proof` checks the
//! 192-byte proof.

mod field;
mod meta;
mod prepared;
mod synthesis;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Fr;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = Fr;
pub use meta::META;
pub use prepared::TAMPER_OFFSET;

/// The GM17 exhibit; stateless, since every run builds its own proving key.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gm17;

impl ProofSystem for Gm17 {
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
