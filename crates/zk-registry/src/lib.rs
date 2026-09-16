//! The proof systems this program knows, and the pages that describe them.
//!
//! Every screen and every command asks this crate what is on the shelves, so wiring a new system
//! in is one line in [`systems`]'s list and nothing anywhere else. A build with no systems at all is
//! a valid build: the screens say so instead of failing.

mod pages;
mod systems;

use zk_core::ProofSystem;
use zk_i18n::Language;

/// Every system built into this binary, in shelf order (Zcash, Aztec, Polygon, Others, Homemade).
///
/// The order is the order people meet systems in `/list` and in `/run all`, so it is fixed here
/// rather than sorted at each screen.
pub fn systems() -> &'static [&'static dyn ProofSystem] {
    systems::SYSTEMS
}

/// The system with this permanent ID, if this binary has it.
pub fn system(id: &str) -> Option<&'static dyn ProofSystem> {
    systems().iter().copied().find(|system| system.meta().id == id)
}

/// The markdown page for `id` in `language`, falling back to English; `None` when no page was
/// present under `catalog/` when this crate was compiled.
///
/// A page can exist for a system that is not built in (the museum describes what it cannot run
/// yet), so this does not look at [`systems`].
pub fn page(id: &str, language: Language) -> Option<&'static str> {
    pages::find(id, language.code()).or_else(|| pages::find(id, Language::ENGLISH.code()))
}

#[cfg(test)]
mod tests {
    use zk_core::catalog::Shelf;

    use super::*;

    #[test]
    fn systems_stand_in_shelf_order() {
        let shelves: Vec<Shelf> = systems().iter().map(|system| system.meta().shelf).collect();
        let mut sorted = shelves.clone();
        sorted.sort();
        assert_eq!(shelves, sorted, "systems() must list shelves in display order");
    }

    #[test]
    fn system_ids_are_unique_and_command_friendly() {
        let mut ids: Vec<&str> = systems().iter().map(|system| system.meta().id).collect();
        for id in &ids {
            assert!(
                id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{id} is not lowercase letters, digits and hyphens"
            );
            assert_eq!(system(id).map(|s| s.meta().id), Some(*id));
        }
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "two systems share an id");
    }

    #[test]
    fn every_built_in_system_has_an_english_page() {
        for system in systems() {
            let id = system.meta().id;
            assert!(
                page(id, Language::ENGLISH).is_some(),
                "{id} is built in but catalog/{id}/README.md does not exist"
            );
        }
    }

    #[test]
    fn unknown_ids_have_neither_system_nor_page() {
        assert!(system("no-such-system").is_none());
        assert!(page("no-such-system", Language::ENGLISH).is_none());
        assert!(page("no-such-system", Language::KOREAN).is_none());
    }

    #[test]
    fn a_missing_translation_falls_back_to_the_english_page() {
        for (id, code, text) in pages::PAGES {
            assert!(!text.trim().is_empty(), "catalog/{id} ({code}) is empty");
            let korean = page(id, Language::KOREAN);
            let has_korean = pages::find(id, Language::KOREAN.code()).is_some();
            if !has_korean {
                assert_eq!(korean, page(id, Language::ENGLISH));
            }
        }
    }
}
