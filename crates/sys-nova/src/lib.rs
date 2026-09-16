//! Nova (Kothapalli, Setty, Tzialla; CRYPTO 2022), run by Microsoft's own `nova-snark` over the
//! Pasta cycle.
//!
//! Nova proves incrementally verifiable computation by folding: instead of verifying the previous
//! proof inside the next one, it folds two instances of a relaxed R1CS into one, so each step costs
//! about one multi-scalar multiplication. The museum makes every statement the step function `F`
//! of a two-step chain (`step`): the public inputs are the chain's state, which `F` passes through
//! unchanged, and every row of the statement's R1CS is enforced on the private advice. Setup runs
//! `PublicParams::setup` and `CompressedSNARK::setup` (`prepared`); proving runs
//! `RecursiveSNARK::new`, `prove_step` twice and `CompressedSNARK::prove`, which folds the running
//! instance with a random one and proves it with Spartan and an inner-product argument
//! (`config`); the proof travels as bincode with the legacy configuration (`proof`).
//!
//! Folding with a random instance before compression is what makes the compressed proof
//! zero-knowledge: the witness the Spartan proof speaks about is the real one plus a multiple of a
//! uniformly random one, and the commitments that travel with it are blinded.

mod config;
mod field;
mod meta;
mod prepared;
mod proof;
mod step;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use config::NUM_STEPS;
pub use field::PallasScalar;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = PallasScalar;
pub use meta::META;
pub use proof::TAMPER_OFFSET;

/// The Nova exhibit; stateless, since every statement gets its own public parameters and keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct Nova;

impl ProofSystem for Nova {
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
