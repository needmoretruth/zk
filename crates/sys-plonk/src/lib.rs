//! PLONK (Gabizon, Williamson and Ciobotaru, 2019) as an exhibit, run by lambdaworks' prover.
//!
//! PLONK proves that a table of wire values satisfies one gate equation per row,
//! `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C + PI = 0`, and that cells wired together hold equal values,
//! with a permutation argument, all committed to with KZG over BLS12-381. Its setup is universal:
//! one reference string of powers of a secret `τ` serves every circuit up to a size, and each circuit
//! then only needs preprocessing. Aztec built its rollups on PLONK and its descendants.
//!
//! The permissively licensed Rust PLONK implementations are few, so this exhibit runs
//! [lambdaworks](https://github.com/lambdaclass/lambdaworks)' `lambdaworks-plonk` (Apache-2.0,
//! pinned to one commit), which follows the paper's five prover rounds, with the gnark-style
//! quotient split its documentation describes. Each statement is lowered to the museum's PLONKish
//! table and laid out as lambdaworks' preprocessed circuit description (`layout`); setup draws the
//! reference string and commits to the circuit (`setup`); proving and verifying call lambdaworks'
//! `Prover::prove` and `Verifier::verify_with_validation` (`proof`, `prepared`).
//!
//! # A one-person setup
//!
//! The reference string is generated on this machine from a fresh random `τ` that is dropped once
//! the powers are computed. Nobody else contributed, so anyone who could read this process's memory
//! during setup could forge proofs for that run. PLONK's setup is updatable precisely so that real
//! deployments can avoid this with a ceremony of many participants.
//!
//! # Zero knowledge
//!
//! lambdaworks' prover adds the paper's blinding: random multiples of the vanishing polynomial
//! `Z_H` on the wire polynomials `a`, `b`, `c` (two coefficients each) and on the permutation
//! polynomial `z` (three), plus random terms tying the three quotient parts together, all drawn
//! from operating-system randomness (`SecureRandomFieldGenerator`). The exhibit therefore runs
//! zero-knowledge as configured.

mod field;
mod layout;
mod meta;
mod prepared;
mod proof;
mod setup;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Fr;
pub use meta::{LAMBDAWORKS_REV, META};
pub use proof::TAMPER_OFFSET;

/// The field this exhibit's statements are built over, so an activity can compute values in it.
pub type Field = Fr;

/// The PLONK exhibit; stateless, since every run makes its own reference string and preprocessing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Plonk;

impl ProofSystem for Plonk {
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
