//! The list itself.
//!
//! To wire a system in: add its crate to this crate's `Cargo.toml`, then add `&its_crate::System`
//! below in shelf order. The tests in `lib.rs` check the order, the IDs and that a page exists.

use zk_core::ProofSystem;

/// Every system built into this binary: shelf by shelf, and on each shelf by the year it was first
/// published, so `/list` reads as a history.
pub(crate) static SYSTEMS: &[&dyn ProofSystem] = &[
    // Zcash
    &sys_schnorr::Schnorr,
    &sys_bctv14::Bctv14,
    &sys_groth16::Groth16,
    &sys_halo2::Halo2,
    // Aztec
    &sys_plonk::Plonk,
    &sys_ultraplonk::UltraPlonk,
    // Polygon
    &sys_winterfell::Winterfell,
    &sys_miden::MidenVm,
    &sys_plonky3::Plonky3,
    &sys_circle_stark::CircleStark,
    // Others
    &sys_gm17::Gm17,
    &sys_bulletproofs::Bulletproofs,
    &sys_stark::Stark,
    &sys_spartan::Spartan,
    &sys_marlin::Marlin,
    &sys_nova::Nova,
    // Homemade
    &zk_cave::Cave,
    &zk_trio::Trio,
];
