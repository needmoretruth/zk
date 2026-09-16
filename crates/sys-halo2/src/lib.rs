//! Halo 2, the proof system of Zcash's Orchard (2022) and Ironwood (2026) pools, as an exhibit.
//!
//! Halo 2 is PLONKish arithmetization with an inner-product-argument polynomial commitment over
//! the Pasta curves: no trusted setup, only generators derived by hashing. This crate runs Zcash's
//! own `halo2_proofs` 0.3.5 on the museum's seven statements.
//!
//! Each statement is built over the Pallas base field, lowered to the shared PLONKish table, and
//! laid out as a Halo 2 circuit (`circuit`). Setup derives `Params` and both keys (`setup`); proving
//! and verifying call `create_proof` and `verify_proof` with Blake2b transcripts (`prepared`).
//! Nothing Zcash-specific beyond the proof system is used: no Orchard gadgets, no Sinsemilla.

mod circuit;
mod field;
mod meta;
mod prepared;
mod setup;

use std::sync::Arc;

use zk_circuit::lower::plonkish::Plonkish;
use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::PastaFp;
pub use meta::META;

use crate::prepared::Ready;

/// Halo 2 on the Pasta curves, as `halo2_proofs` implements it for Zcash.
///
/// A unit struct because everything per-statement is built in [`ProofSystem::prepare`].
pub struct Halo2;

impl ProofSystem for Halo2 {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<PastaFp>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let table = Arc::new(Plonkish::from_circuit(&circuit));
        control.checkpoint()?;
        let keys = setup::generate(&table, control)?;
        Ok(Box::new(Ready::new(circuit, table, keys)))
    }
}
