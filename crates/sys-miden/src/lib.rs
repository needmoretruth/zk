//! Miden VM (`miden-assembly`, `miden-processor`, `miden-prover` and `miden-verifier` 0.32.1) over
//! Goldilocks, as a museum exhibit.
//!
//! Every other exhibit lays a statement out as a circuit. Miden proves that a program ran: the
//! statement becomes Miden Assembly, the VM executes it, and the prover proves the execution trace
//! with a lifted STARK built on Plonky3 over the Goldilocks field. The verifier never sees the
//! program, only its hash, the values the program started with on its stack and the values it left.
//!
//! This crate writes each circuit, built over Goldilocks, as a program (`program`): public inputs
//! are the program's stack inputs, private inputs and hint outputs come from the advice stack, each
//! gate is stack arithmetic on memory cells, and each assert-zero is an `assertz`. Miden's own
//! assembler compiles the program; `miden_prover::prove_sync` executes and proves it, and
//! `miden_verifier::Verifier` checks the proof against the program hash, the stack inputs and the
//! stack outputs, which are sixteen zeros (`vm`). Proofs travel in Miden's own encoding.
//!
//! A false claim never reaches a proof: the processor runs every `assertz`, and execution stops at
//! the first one that fails. That is Miden's prover refusing, reported with the circuit label.
//!
//! # Not zero-knowledge, as found in the 0.32.1 code
//!
//! The default prover (`Prover::new`, Blake3) builds its configuration with
//! `miden_air::config::blake3_256_config`, whose commitment scheme is `LmcsConfig` with no salt
//! elements. `miden-lifted-stark` does ship a salted `HidingLmcsConfig`, but no Miden configuration
//! uses it, and the other hash choices (RPO, RPX, Poseidon2, Keccak) are built the same way. The
//! lifted STARK prover takes no random number generator: it commits to the execution trace as it
//! is, with no random rows or columns, and every challenge comes from the Fiat–Shamir transcript.
//! A proof is therefore a deterministic function of the program, its inputs and the advice.
//!
//! Measured on the five statements whose secrets the scan searches (all but `sudoku` and
//! `factoring`, whose secrets are below 2^16): no secret appears in the proof bytes. The proof
//! carries no trace cell itself. The prover opens each committed column's low-degree extension at
//! the out-of-domain points and at the low-degree test's query points, all off the trace domain,
//! so each opened value is a fixed linear combination of the whole column rather than one cell.
//! Without masking, those combinations still carry information about the witness; they are
//! simply not verbatim.
//!
//! Missing hiding shows another way, tested in `tests/behaviour.rs`: `factoring` has two witnesses
//! for one public claim, `(p, q)` and `(q, p)`. Their proofs differ, and proving a guessed witness
//! again reproduces its proof byte for byte, so anyone holding a proof and a guess can confirm the
//! guess.
//!
//! # Dependencies
//!
//! The `miden-vm` facade crate re-exports exactly these components but switches on the assembler's
//! `std` feature, which brings in the package resolver `pubgrub` under MPL-2.0. The assembler
//! without `std` compiles the same language, so the components are used directly.

mod field;
mod layout;
mod meta;
mod prepared;
mod program;
mod vm;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Goldilocks;
pub use meta::META;

/// The field the statements are built over, for activities that compute values in it.
pub type Field = Goldilocks;

/// The Miden VM exhibit; stateless, since a program has no keys or randomness.
#[derive(Clone, Copy, Debug, Default)]
pub struct MidenVm;

impl ProofSystem for MidenVm {
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
