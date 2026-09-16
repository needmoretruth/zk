//! The two views the pool is about, which are also its two file formats: the ledger (what the world
//! sees) and a wallet (what its owner sees).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::receipt::{NoteDelivery, SpendCircuit, TransactionRecord};

/// Version of both file formats; a file with another number is refused rather than misread.
pub(crate) const FORMAT: u32 = 1;

/// The world's view: everything public, which is also exactly what `ledger.json` holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerView {
    /// File format version.
    pub format: u32,
    /// Proof system ID whose keys verify this ledger's transactions.
    pub system: String,
    /// Field every element is encoded in.
    pub field: String,
    /// The spend circuit transactions are verified against.
    pub circuit: SpendCircuit,
    /// Transparent balances by account name, in the open.
    pub transparent: BTreeMap<String, u64>,
    /// Value inside the shielded pool: Σ `v_pub_in` − Σ `v_pub_out` (ZIP 209's turnstile).
    pub pool_balance: u64,
    /// Leaves the commitment tree can hold.
    pub tree_capacity: u64,
    /// Note commitments in tree order.
    pub commitments: Vec<String>,
    /// The current root.
    pub root: String,
    /// Every root the tree has had, oldest first; a spend may anchor to any of them.
    pub roots_seen: Vec<String>,
    /// Nullifiers of spent notes, in the order published.
    pub nullifiers: Vec<String>,
    /// Accepted transactions in order.
    pub transactions: Vec<TransactionRecord>,
}

/// A note as its owner's wallet keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRecord {
    /// Its value.
    pub value: u16,
    /// Its commitment randomness, which only the owner knows.
    pub rcm: String,
    /// Its public commitment.
    pub commitment: String,
    /// The nullifier that spending it will publish; only the key holder can compute it.
    pub nullifier: String,
    /// Its position in the commitment tree.
    pub position: u64,
    /// Whether its nullifier is in the ledger's nullifier set.
    pub spent: bool,
    /// Index of the transaction that created it.
    pub received_in: u64,
    /// How it arrived.
    pub delivery: NoteDelivery,
}

/// An owner's view of one wallet, which is also exactly what `wallets/<name>.json` holds.
///
/// It carries the spending key in the clear: this is a toy with no value, and seeing the key is part
/// of seeing what the ledger never sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletView {
    /// File format version.
    pub format: u32,
    /// The wallet's name, also its transparent account's name.
    pub name: String,
    /// The spending key `sk`.
    pub spending_key: String,
    /// The address `pk = ToyHash(sk, 0)` that notes are paid to.
    pub address: String,
    /// Sum of unspent notes.
    pub balance: u64,
    /// Every note received, spent or not.
    pub notes: Vec<NoteRecord>,
}
