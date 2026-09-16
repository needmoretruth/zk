//! Spartan (Setty, CRYPTO 2020), run by Microsoft's own `spartan` crate over ristretto255.
//!
//! Spartan proves R1CS satisfiability with two sum-checks and a Hyrax-style polynomial commitment,
//! so nothing has to be trusted before the first proof: the generators are hashed from labels. Each
//! statement is lowered to the museum's R1CS and rearranged into Spartan's column order (`layout`);
//! `SNARKGens::new` derives the generators and `SNARK::encode` commits to the constraint matrices
//! (`prepared`); `SNARK::prove` and `SNARK::verify` do the rest, and the proof travels as the bincode
//! bytes Spartan itself measures (`proof`).
//!
//! The crate also offers `NIZK`. Both hide the witness — they share one blinded R1CS satisfiability
//! proof — but only `SNARK` preprocesses the matrices so the verifier does not re-read them, which
//! is the succinct system the paper names, so that is the one on show.

mod field;
mod layout;
mod meta;
mod prepared;
mod proof;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::RistrettoScalar;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = RistrettoScalar;
pub use meta::META;
pub use proof::TAMPER_OFFSET;

/// The Spartan exhibit; stateless, since every run derives its own generators and commitment.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spartan;

impl ProofSystem for Spartan {
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
