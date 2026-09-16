//! What the screens can show: the systems built in and the pages that describe them.
//!
//! The screens do not reach into the registry crate themselves. The program hands them a
//! [`Museum`], so tests can build screens around stand-in systems that never ship.

use zk_core::ProofSystem;
use zk_i18n::Language;

/// The systems and pages a screen works with.
#[derive(Clone, Copy)]
pub struct Museum {
    /// Every system, in shelf order.
    pub systems: &'static [&'static dyn ProofSystem],
    /// The markdown page for a system ID in a language, falling back to English.
    pub page: fn(&str, Language) -> Option<&'static str>,
}

impl Museum {
    /// A museum with nothing in it, which every screen must handle.
    pub fn empty() -> Museum {
        Museum { systems: &[], page: |_, _| None }
    }

    /// The system with this ID.
    pub fn system(&self, id: &str) -> Option<&'static dyn ProofSystem> {
        self.systems.iter().copied().find(|system| system.meta().id == id)
    }
}

impl core::fmt::Debug for Museum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ids: Vec<&str> = self.systems.iter().map(|system| system.meta().id).collect();
        f.debug_struct("Museum").field("systems", &ids).finish_non_exhaustive()
    }
}
