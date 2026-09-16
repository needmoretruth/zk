//! Instances for a run: the sample claims, with fresh secrets where a fixed one would give the game away.
//!
//! The deterministic `honest`/`dishonest` assignments are right for tests, but a salt written in
//! the source is a salt everyone knows: anyone could hash each candidate answer with it and open
//! the envelope without any proof. A run therefore derives its salts from a seed the program draws
//! from the operating system. Statements whose secret *is* the point of a later lesson (the PIN, the
//! sudoku solution, the factors) keep their sample values.

use sha2::{Digest, Sha256};
use zk_circuit::{Assignment, ZkField};

use crate::{ExampleId, age, one_plus_one};

/// Whether an instance makes a true claim or a false one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InstanceKind {
    /// A true claim with a valid witness; every sound system must accept its proof.
    Honest,
    /// A false claim; every sound system must refuse to prove it or reject the proof.
    Dishonest,
}

/// A 64-bit value derived from `seed` for one named purpose, so two salts drawn from one seed differ.
pub fn derive_u64(seed: &[u8; 32], purpose: &str) -> u64 {
    let digest = Sha256::new_with_prefix(b"zk/examples/v1/instance/")
        .chain_update(purpose.as_bytes())
        .chain_update(seed)
        .finalize();
    let mut first = [0u8; 8];
    first.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(first)
}

impl ExampleId {
    /// The claim of `kind` for this example, with salts derived from `seed`.
    ///
    /// On a 31-bit field a salt carries at most 31 bits however it is drawn, so an envelope there can
    /// still be opened by trying every salt; the museum says so where it shows those fields.
    pub fn instance<F: ZkField>(self, kind: InstanceKind, seed: &[u8; 32]) -> Assignment<F> {
        match (self, kind) {
            (Self::OnePlusOne, InstanceKind::Honest) => {
                one_plus_one::sealed(2, derive_u64(seed, "one-plus-one/salt"))
            }
            (Self::OnePlusOne, InstanceKind::Dishonest) => {
                one_plus_one::sealed(3, derive_u64(seed, "one-plus-one/salt"))
            }
            (Self::Age, InstanceKind::Honest) => {
                age::born_in(age::ADULT_BIRTH_YEAR, derive_u64(seed, "age/salt"))
            }
            (Self::Age, InstanceKind::Dishonest) => {
                age::born_in(age::SEVENTEEN_YEAR_OLD_BIRTH_YEAR, derive_u64(seed, "age/salt"))
            }
            (_, InstanceKind::Honest) => self.honest(),
            (_, InstanceKind::Dishonest) => self.dishonest(),
        }
    }
}
