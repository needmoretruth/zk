//! Bulletproofs (Bünz, Bootle, Boneh, Poelstra, Wuille and Maxwell, 2018) as an exhibit.
//!
//! Bulletproofs proves statements about committed values with nothing but a group where discrete
//! logarithms are hard: no trusted setup, and proofs that grow with the logarithm of the circuit.
//! Monero adopted its range proofs in 2018. The same paper also proves arbitrary arithmetic circuits,
//! which is what lets the museum run it on the seven statements every other system proves.
//!
//! This exhibit runs [zkcrypto/bulletproofs](https://github.com/zkcrypto/bulletproofs) (MIT, pinned
//! to one commit) over ristretto255, with its R1CS prover behind the `yoloproofs` feature. Each
//! statement is lowered to the museum's R1CS and replayed, row for row, into the upstream `Prover`
//! and `Verifier` constraint systems (`synthesis`); `PedersenGens::default` and `BulletproofGens::new`
//! are the whole setup (`prepared`); `Prover::prove` and `Verifier::verify` do the rest (`proof`).
//!
//! # Statement binding
//!
//! Public inputs enter the constraints as constants, and the upstream protocol hashes commitments
//! but not constants into its transcript. The exhibit therefore starts both transcripts with the
//! example ID and every public input, so the challenges depend on the whole statement.
//!
//! # Zero knowledge
//!
//! Every private value is a low-level variable of the proof, committed only inside the blinded
//! vector commitments `A_I`, `A_O` and `S`, and the polynomial commitments `T_i` carry their own
//! blinding factors, all drawn from a transcript RNG keyed with operating-system randomness. The
//! exhibit therefore runs zero-knowledge as configured.

mod field;
mod meta;
mod prepared;
mod proof;
mod synthesis;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::RistrettoScalar;
pub use meta::{BULLETPROOFS_REV, META};
pub use proof::TAMPER_OFFSET;

/// The field this exhibit's statements are built over, so an activity can compute values in it.
pub type Field = RistrettoScalar;

/// The Bulletproofs exhibit; stateless, since every run derives its own generators.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bulletproofs;

impl ProofSystem for Bulletproofs {
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
