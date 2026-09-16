//! The proof system's keys for each spend circuit, built once per pool and reused by every proof,
//! verification and attack.

use std::time::Instant;

use zk_core::{Control, Prepared, ProofSystem};

use crate::error::PoolError;
use crate::receipt::SpendCircuit;

/// A proof system and the circuits it has been prepared for so far.
pub(crate) struct Keys {
    system: &'static dyn ProofSystem,
    ready: Vec<(SpendCircuit, Box<dyn Prepared>)>,
}

impl Keys {
    /// No keys yet: setup costs seconds, so it waits for the first proof that needs it.
    pub(crate) fn new(system: &'static dyn ProofSystem) -> Self {
        Self { system, ready: Vec::new() }
    }

    /// The proof system's ID.
    pub(crate) fn system_id(&self) -> &'static str {
        self.system.meta().id
    }

    /// The keys for `circuit`, running setup first if this pool has none; the second value is that
    /// setup's duration.
    pub(crate) fn prepared(
        &mut self,
        circuit: SpendCircuit,
        control: &Control,
    ) -> Result<(&mut dyn Prepared, Option<u64>), PoolError> {
        let mut setup_micros = None;
        if !self.ready.iter().any(|(built, _)| *built == circuit) {
            let started = Instant::now();
            let prepared = self.system.prepare(circuit.example(), control)?;
            setup_micros = Some(micros(started));
            self.ready.push((circuit, prepared));
        }
        let prepared = self
            .ready
            .iter_mut()
            .find(|(built, _)| *built == circuit)
            .map(|(_, prepared)| prepared.as_mut())
            .ok_or_else(|| PoolError::Circuit("keys vanished after setup".into()))?;
        Ok((prepared, setup_micros))
    }
}

/// Microseconds since `started`.
pub(crate) fn micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}
