//! The screens of `nmtzk`: a transcript of results above a composer where slash commands are typed.
//!
//! The layout follows chat-style terminal tools rather than panels: results pile up as cells,
//! a spinner line says what is running, and the composer offers commands as you type. Every value
//! the design fixes (colours, glyphs, keys, sizes) is applied here and nowhere else.
//!
//! Engine crates return numbers and enums; every word the program shows lives in this crate's
//! phrase tables. The same documents are drawn by the TUI ([`App`], [`run`]) and printed by the
//! command-line subcommands ([`plain::Printer`]), so both always say the same thing.
//!
//! Interactive pieces that stream story beats or custom cells — the cave, Trio rounds, the toy
//! shielded pool, a trusted-setup ceremony — implement [`Activity`] and are started with
//! [`App::start`].

pub mod activities;
pub mod activity;
mod app;
mod commands;
mod complete;
mod composer;
pub mod doc;
mod format;
mod layout;
mod look;
mod markdown;
mod museum;
mod pager;
mod paint;
mod phrases;
pub mod plain;
mod runs;
mod table;
mod terminal;
mod text;
mod transcript;
pub mod views;

pub use activity::{Activity, CellRef, Outbox};
pub use app::{App, Settings};
pub use look::Look;
pub use museum::Museum;
pub use terminal::run;
