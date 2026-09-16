//! Groth16 exactly as Zcash Sapling runs it: the `bellman` crate over BLS12-381.
//!
//! Sapling's spend and output proofs, and since Sapling also Sprout's JoinSplits, are Groth16 proofs
//! made by `bellman`. This exhibit runs that very code on the museum's seven statements: each circuit
//! is lowered to R1CS and replayed through `bellman::Circuit`, `generate_random_parameters` runs the
//! per-circuit setup, `create_random_proof` proves with fresh OS randomness, and `verify_proof`
//! checks the 192-byte proof.

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

/// The Groth16 exhibit; stateless, since every run builds its own parameters.
#[derive(Clone, Copy, Debug, Default)]
pub struct Groth16;

impl ProofSystem for Groth16 {
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
