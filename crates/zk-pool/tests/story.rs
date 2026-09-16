//! The reader's story, end to end, on both proof systems Zcash's shielded pools used: two wallets,
//! a faucet, a shield, a private send and an unshield, then the pool closed and reopened from disk
//! and used again.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;

use common::{ScratchDir, timing_line};
use zk_circuit::ZkField;
use zk_pool::{
    Direction, NoteDelivery, Pool, PoolError, Receipt, TransactionKind, TransparentMove,
};

fn accepted(system: &str, label: &str, receipt: Receipt) -> Receipt {
    println!("{}", timing_line(system, label, &receipt));
    assert!(receipt.accepted(), "{label}: {:?}", receipt.decision);
    receipt
}

fn story<F: ZkField>(open: fn(&Path) -> Result<Pool<F>, PoolError>) {
    let dir = ScratchDir::new("story");
    let mut pool = open(dir.path()).unwrap();
    let system = pool.system_id();
    pool.new_wallet("alice").unwrap();
    pool.new_wallet("bob").unwrap();
    pool.faucet("alice", 500).unwrap();
    accepted(system, "shield 300", pool.shield("alice", 300).unwrap());
    let send = accepted(system, "send 120", pool.send("alice", "bob", 120).unwrap());
    accepted(system, "unshield 50", pool.unshield("bob", 50).unwrap());

    // The world sees a shielded transfer happen, but neither who was paid nor how much.
    assert_eq!(send.public.kind, Some(TransactionKind::ShieldedTransfer));
    assert_eq!(send.public.transparent, None);
    let outputs = &send.private.outputs;
    assert_eq!(outputs[0].recipient.as_deref(), Some("bob"));
    assert_eq!(outputs[0].value, 120);
    assert_eq!(outputs[0].delivery, Some(NoteDelivery::HandedToLocalWallet));

    let ledger = pool.ledger();
    assert_eq!(ledger.transparent.get("alice"), Some(&200));
    assert_eq!(ledger.transparent.get("bob"), Some(&50));
    let (alice, bob) = (pool.wallet("alice").unwrap(), pool.wallet("bob").unwrap());
    assert_eq!((alice.balance, bob.balance), (180, 70));
    // The turnstile: value that entered minus value that left equals the notes still unspent.
    assert_eq!(ledger.pool_balance, 250);
    assert_eq!(ledger.pool_balance, alice.balance + bob.balance);
    assert_eq!((ledger.commitments.len(), ledger.nullifiers.len()), (6, 3));
    assert_eq!(ledger.transactions.len(), 4);
    let unshield = ledger.transactions[3].transparent.clone();
    let credit =
        TransparentMove { account: "bob".into(), direction: Direction::Credit, amount: 50 };
    assert_eq!(unshield, Some(credit));
    assert_eq!(
        pool.unshield("bob", 71),
        Err(PoolError::NoNoteLargeEnough { name: "bob".into(), amount: 71, largest: 70 })
    );
    drop(pool);

    let mut reopened = open(dir.path()).unwrap();
    assert_eq!(reopened.ledger(), ledger);
    assert_eq!(reopened.wallet("alice").unwrap(), alice);
    assert_eq!(reopened.wallet("bob").unwrap(), bob);
    accepted(system, "send 20 after reopening", reopened.send("bob", "alice", 20).unwrap());
    let (alice, bob) = (reopened.wallet("alice").unwrap(), reopened.wallet("bob").unwrap());
    assert_eq!((alice.balance, bob.balance, reopened.ledger().pool_balance), (200, 50, 250));
}

#[test]
fn groth16_story() {
    story(|dir| Pool::groth16(dir));
}

#[test]
fn halo2_story() {
    story(|dir| Pool::halo2(dir));
}
