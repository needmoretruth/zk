//! The ways the story is filmed. Each returns what happened scene by scene plus every step as an
//! [`crate::Event`], and takes the random source as a parameter: [`crate::OsRng`] for the museum,
//! a seeded or scripted generator for tests.

pub mod agreement;
pub mod building;
pub mod demonstration;
pub mod edit;
