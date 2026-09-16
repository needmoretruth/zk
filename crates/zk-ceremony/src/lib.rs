//! The trusted-setup ceremony, run on this machine: why a pairing SNARK's setup must forget its
//! secrets, and why many people taking turns is enough.
//!
//! Groth16 and KZG verifiers compare points that were computed from secret numbers. Whoever keeps
//! those numbers can prove false statements, and no verifier can tell. Zcash's 2016 parameter
//! ceremony and the Powers of Tau ceremonies after it exist because of that. This crate runs the
//! three facts behind them:
//!
//! - [`toxic`]: holding the secret numbers of a Groth16 setup, anyone can make a proof that the
//!   `one-plus-one` envelope holds 3, without any witness, and bellman's ordinary verifier accepts it.
//! - [`tau`]: participants in turn multiply a secret of their own into a Powers of Tau reference
//!   string and publish a proof that they did; [`tau::verify_chain`] checks every turn, so a single
//!   participant who does not keep their secret leaves the final τ unknown to everyone, and the two
//!   cheats in [`tau::cheat`] are caught.
//! - [`kzg`]: if instead every participant kept their secret, their product is τ, and it opens a
//!   KZG commitment over the final string to a value the committed polynomial does not have.
//!
//! Teaching implementation, not audited. The curve, the pairing, the hash and bellman's Groth16
//! come from crates; only the ceremony's protocol steps are written here. A secret that is not
//! kept on purpose is not stored after the function that sampled it returns; nothing here claims
//! to wipe it from memory.

mod field;
pub mod kzg;
mod pairing;
mod synthesis;
pub mod tau;
pub mod toxic;
