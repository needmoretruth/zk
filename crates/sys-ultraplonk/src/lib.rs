//! UltraPlonk as an exhibit, run by Espresso Systems' jellyfish prover.
//!
//! Aztec grew PLONK in two steps. TurboPlonk widened the gate: four input wires and one output, two
//! multiplication selectors and custom selectors, so one row does the work of several PLONK rows.
//! UltraPlonk added plookup: a wire can be proved to hold a value from a fixed table, which turns a
//! range check from dozens of bit gates into a few table lookups. UltraPlonk was Aztec's production
//! proof system until UltraHonk replaced it.
//!
//! Aztec's own implementation is C++ (barretenberg), so this exhibit runs
//! [jellyfish](https://github.com/EspressoSystems/jellyfish)'s `jf-plonk` (MIT), a Rust TurboPlonk and
//! UltraPlonk over KZG, pinned to one commit, with `PlonkType::UltraPlonk` and BN254. Each statement
//! is rebuilt from the museum's gates with jellyfish's constraint API (`translate`); range checks the
//! museum writes as bit decompositions become lookups into jellyfish's range table where that keeps
//! the statement the same (`ranges`); setup draws a reference string and preprocesses the circuit
//! (`setup`); proving and verifying call `PlonkKzgSnark::prove` and `verify` (`proof`, `prepared`).
//!
//! # A one-person setup
//!
//! The reference string is made on this machine from a trapdoor drawn from operating-system
//! randomness; see `setup` for what that does and does not protect.
//!
//! # Zero knowledge
//!
//! jellyfish's prover blinds every polynomial it commits to that depends on the witness: random
//! multiples of the vanishing polynomial are added to the six wire polynomials (two coefficients
//! each), the permutation product (three), the two sorted lookup polynomials and the lookup product
//! (three each), and the split quotient parts are tied together with random terms. The randomness is
//! ChaCha20 seeded from the operating system for every proof. The exhibit runs zero-knowledge as
//! configured.

mod field;
mod meta;
mod prepared;
mod proof;
mod ranges;
mod setup;
mod translate;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Fr;
pub use meta::{JELLYFISH_REV, META};
pub use proof::TAMPER_OFFSET;
pub use ranges::RANGE_BIT_LEN;

/// The field this exhibit's statements are built over, so an activity can compute values in it.
pub type Field = Fr;

/// The UltraPlonk exhibit; stateless, since every run makes its own reference string and keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct UltraPlonk;

impl ProofSystem for UltraPlonk {
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
