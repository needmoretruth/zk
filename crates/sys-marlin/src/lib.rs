//! Marlin, the preprocessing zkSNARK with a universal and updatable SRS: `ark-marlin` over BLS12-381.
//!
//! Chiesa, Hu, Maller, Mishra, Vesely and Ward (EUROCRYPT 2020) split Groth16's per-circuit ceremony
//! in two. One structured reference string, sized only by how large a circuit may be, is generated
//! once and can be updated by anyone; each circuit is then *indexed* against it by a deterministic
//! algorithm that needs no secret at all. Aleo's Varuna is its batched descendant. This exhibit runs
//! arkworks' implementation on the museum's seven statements: each circuit is lowered to R1CS and
//! replayed through `ark_relations::r1cs::ConstraintSynthesizer`, `Marlin::universal_setup` builds
//! the SRS over `MarlinKZG10`, `Marlin::index` derives the circuit's keys from it, `Marlin::prove`
//! proves with fresh OS randomness behind a BLAKE2s Fiat–Shamir transcript, and `Marlin::verify`
//! checks the proof.

mod field;
mod meta;
mod prepared;
mod prover;
mod setup;
mod synthesis;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Fr;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = Fr;
pub use meta::META;
pub use prepared::TAMPER_OFFSET;
pub use setup::SRS_BYTES_COUNT;

/// The Marlin exhibit; stateless, since every run generates its own SRS and index.
#[derive(Clone, Copy, Debug, Default)]
pub struct Marlin;

impl ProofSystem for Marlin {
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
