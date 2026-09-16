//! The Toy Shielded Pool: a tiny, local, obviously-not-real shielded payment system.
//!
//! Inspired by the Zcash Sapling and Orchard designs; unofficial and not affiliated with Zcash.
//! It is not a cryptocurrency. There is no network and no value: one ledger file and one file per
//! wallet, in a directory the caller chooses.
//!
//! A reader creates wallets, takes transparent coins from a faucet, shields them into notes, sends
//! notes privately, unshields them, and compares what the world sees ([`Pool::ledger`]) with what
//! a wallet's owner sees ([`Pool::wallet`]). Then the reader attacks it ([`Pool::attack`]).
//!
//! Every shielded transaction is one `pool-spend` proof (see `zk_examples::pool::spend`): one input
//! note, two output notes, 16-bit amounts, `v_in + v_pub_in = v_out_1 + v_out_2 + v_pub_out`. A
//! shield spends a zero-value dummy input, as Sapling does. The ledger accepts a transaction only
//! when the proof verifies under its keys for the honest circuit, the anchor is a root it has seen,
//! the nullifier is new, and its turnstile stays non-negative.
//!
//! Where this toy differs from a real shielded pool, on purpose:
//! - **Note delivery.** A real system encrypts each output note to its recipient and publishes the
//!   ciphertext on-chain. Here the note is handed straight to the recipient's wallet file on this
//!   machine, and every note says so ([`NoteDelivery::HandedToLocalWallet`]).
//! - **The turnstile.** Like Zcash (ZIP 209, <https://zips.z.cash/zip-0209>), the ledger tracks the
//!   value inside the shielded pool and rejects a transaction that would make it negative. That is
//!   defence in depth: counterfeit value made by a broken circuit cannot leave the pool beyond what
//!   was put in.
//! - **Keys.** The proof system's keys are built the first time a pool proves, and are not stored.
//!   A reopened pool builds new ones, so the proofs it recorded earlier stay in the ledger as
//!   history but would not verify under the new keys.
//! - **Hashing.** Addresses, commitments, nullifiers and the tree use `ToyHash`, which is not a real
//!   hash.

mod attack;
mod element;
mod error;
mod keys;
mod ledger;
mod plan;
mod pool;
mod receipt;
mod storage;
mod view;
mod wallet;
mod world;

pub use attack::{Attack, AttackAction, AttackReport, AttackStep};
pub use error::PoolError;
pub use pool::Pool;
pub use receipt::{
    Activity, Decision, Direction, InputNote, NoteDelivery, OutputNote, PrivatePart, PublicPart,
    Receipt, Rejection, ShieldedPublic, SpendCircuit, TransactionKind, TransactionRecord,
    TransparentMove,
};
pub use view::{LedgerView, NoteRecord, WalletView};
