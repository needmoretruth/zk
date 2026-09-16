//! STARK (Ben-Sasson, Bentov, Horesh and Riabzev, 2018) as ethSTARK describes it, run by lambdaworks'
//! prover over Stark252, as a museum exhibit.
//!
//! The original STARK and its engineering description, ethSTARK, are what StarkWare's Stone prover
//! ran for StarkEx and Starknet until S-two replaced it. Stone is C++, so this exhibit runs
//! [lambdaworks](https://github.com/lambdaclass/lambdaworks)' STARK prover (`stark-platinum-prover`,
//! Apache-2.0, pinned to one commit), which follows the same description over the same field: an
//! AIR, the trace's low-degree extension committed in a Keccak Merkle tree, a composition
//! polynomial, the DEEP composition polynomial and FRI, with a Stone-compatible transcript. Each
//! statement is built over Stark252, lowered to the shared wide AIR and implemented as a
//! lambdaworks `AIR` (`air`); proving calls `Prover::prove` and verifying `Verifier::verify`, with
//! proofs travelling as `bincode::serde` bytes (`stark`, `prepared`).
//!
//! The trace is four rows, every wire column holding its value on all of them, because four is the
//! fewest rows lambdaworks' FRI can verify (`stark`). The protocol parameters are lambdaworks' preset
//! for 128 bits of conjectured security.
//!
//! # Not zero-knowledge, and the museum shows it
//!
//! lambdaworks interpolates and commits to the trace exactly as given, and nothing in the prover
//! draws randomness. (Two proofs of one claim can still differ: with `parallel`, the grinding nonce
//! is whichever valid one a thread finds first.) Each wire column's polynomial is constant, so
//! every place a proof carries a trace value carries the wire value itself: the out-of-domain row
//! (each column at `z`) and the trace rows the verifier opens against the Merkle commitment (each
//! column at both points of every FRI query), each as the element's Montgomery form in 32
//! big-endian bytes. The leak scan finds every private input of at least 2^16.
//!
//! # When the prover refuses
//!
//! It does not. In builds with debug assertions lambdaworks validates the trace against the AIR
//! before proving, but only logs what fails; it proves the false claim and `Verifier::verify`
//! rejects the proof.

mod air;
mod field;
mod meta;
mod prepared;
mod stark;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Stark252;
pub use meta::META;

/// The field the statements are built over, for activities that compute values in it.
pub type Field = Stark252;

/// The STARK exhibit; stateless, since a STARK has no keys to keep.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stark;

impl ProofSystem for Stark {
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
