//! Ali Baba's cave, as Jean-Jacques Quisquater, Louis Guillou, their families and Thomas Berson
//! told it in "How to Explain Zero-Knowledge Protocols to Your Children" (CRYPTO '89).
//!
//! The cave is forked, not a ring: its entryway splits into two winding passages, left and right,
//! each ending in a dead end, and a wall between the two ends slides open for the magic words
//! ([`Wall`]). The prover goes in alone; the reporter goes only as far as the fork, flips a coin and
//! calls the exit; the camera films what can be seen from the fork ([`Tape`]). Forty scenes convince
//! the reporter, yet a court cannot tell Mick's tape from a jealous reporter's edited tape of a
//! look-alike double ([`court`]), which is why the genuine tape conveys no knowledge of the secret.
//!
//! This crate is the engine only: the rules, the characters and every way of filming, as data and
//! [`Event`]s. It holds no sentences; the screen narrates.
//!
//! The words are an example's private inputs, the wall's lock is the example's circuit set to its
//! public inputs, and the museum runs the whole demonstration through the shared harness as
//! [`Cave`].

mod cast;
pub mod court;
mod error;
mod event;
mod field;
pub mod modes;
pub mod odds;
mod reporter;
mod scene;
mod setting;
mod system;
mod tape;
mod wall;

pub use cast::{Character, Coin, OsRng, Prover, Side, WallOutcome};
pub use error::CaveError;
pub use event::{At, Event};
pub use field::Goldilocks;
pub use modes::agreement::prior_agreement;
pub use modes::building::{BuildingRound, Floor, apartment_building};
pub use modes::demonstration::{Demonstration, SCENES, demonstration};
pub use modes::edit::{Edit, MAX_TAKES_PER_KEPT_SCENE, jealous_edit};
pub use scene::Scene;
pub use setting::Setting;
pub use system::{Cave, META};
pub use tape::{TAPE_MAGIC, TAPE_VERSION, Tape, TapeScene};
pub use wall::{MagicWords, Wall};
