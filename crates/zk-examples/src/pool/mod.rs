//! The Toy Shielded Pool: the skeleton of a shielded payment, inspired by the Zcash Sapling and
//! Orchard designs. Unofficial, and not affiliated with Zcash. Not real money: no network, no value.
//!
//! This module holds what a ledger computes outside the circuit (addresses, note commitments,
//! nullifiers, the commitment tree); [`spend`] is the circuit that proves a spend.

pub mod spend;

use zk_circuit::ZkField;
use zk_circuit::toyhash::toyhash;

use crate::merkle_tree::MerkleTree;

/// Depth of the note commitment tree: 256 notes.
pub const TREE_DEPTH: usize = 8;

/// Bits of every amount: values below 65 536.
pub const VALUE_BITS: u32 = 16;

/// The address that receives notes for spending key `sk`: `pk = ToyHash(sk, 0)`.
pub fn address<F: ZkField>(sk: F) -> F {
    toyhash(sk, F::zero())
}

/// The public commitment to a note: `cm = ToyHash(ToyHash(pk, value), rcm)`.
///
/// `rcm` is the note's randomness; it keeps two notes with the same owner and value from having
/// the same commitment.
pub fn note_commitment<F: ZkField>(pk: F, value: F, rcm: F) -> F {
    toyhash(toyhash(pk, value), rcm)
}

/// The tag published when a note is spent: `nf = ToyHash(sk, cm)`.
///
/// Only the key holder can compute it, and a note always yields the same one, so the ledger refuses
/// a second spend without learning which note was spent.
pub fn nullifier<F: ZkField>(sk: F, cm: F) -> F {
    toyhash(sk, cm)
}

/// An empty note commitment tree of depth [`TREE_DEPTH`].
pub fn new_note_tree<F: ZkField>() -> MerkleTree<F> {
    MerkleTree::new(TREE_DEPTH)
}
