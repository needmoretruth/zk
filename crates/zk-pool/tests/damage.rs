//! A pool shared between processes, and pool files that were damaged or edited: nothing is lost to
//! a race, and nothing damaged is believed or silently replaced.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[allow(dead_code, reason = "the timing line is for the attack and story reports")]
mod common;

use std::fs;
use std::path::Path;
use std::thread;

use common::ScratchDir;
use serde_json::Value;
use zk_core::Control;
use zk_pool::{Pool, PoolError};

fn ledger_file(dir: &Path) -> std::path::PathBuf {
    dir.join("ledger.json")
}

fn wallet_file(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join("wallets").join(format!("{name}.json"))
}

/// Rewrites a JSON file through `edit`.
fn edit(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    edit(&mut value);
    fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

/// Groth16 pool with alice holding a shielded note of 30 and 10 transparent coins.
fn funded(label: &str) -> ScratchDir {
    let dir = ScratchDir::new(label);
    let mut pool = Pool::groth16(dir.path()).unwrap();
    pool.new_wallet("alice").unwrap();
    pool.faucet("alice", 40).unwrap();
    assert!(pool.shield("alice", 30).unwrap().accepted());
    dir
}

/// A named change to a stored ledger.
type Edit = (&'static str, fn(&mut Value));

fn corrupt(result: Result<Pool<sys_groth16::Field>, PoolError>) -> String {
    match result {
        Err(PoolError::Corrupt(why)) => why,
        Err(other) => panic!("expected a corrupt pool, got {other:?}"),
        Ok(_) => panic!("expected a corrupt pool, but it opened"),
    }
}

#[test]
fn writers_that_hold_the_lock_take_turns_and_lose_nothing() {
    let dir = ScratchDir::new("lock");
    Pool::groth16(dir.path()).unwrap().new_wallet("alice").unwrap();
    let (threads, rounds) = (8, 10);
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                for _ in 0..rounds {
                    let _lock = zk_pool::lock(dir.path(), &Control::new(), || {}).unwrap();
                    let mut pool = Pool::groth16(dir.path()).unwrap();
                    pool.faucet("alice", 1).unwrap();
                }
            });
        }
    });
    let ledger = Pool::groth16(dir.path()).unwrap().ledger();
    assert_eq!(ledger.transparent.get("alice"), Some(&(threads * rounds)));
    assert_eq!(ledger.transactions.len() as u64, threads * rounds);
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .chain(fs::read_dir(dir.path().join("wallets")).unwrap())
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn a_lost_ledger_is_refused_while_a_wallet_holds_notes_and_a_reset_starts_over() {
    let dir = funded("lost");
    let before = fs::read(wallet_file(dir.path(), "alice")).unwrap();
    fs::remove_file(ledger_file(dir.path())).unwrap();
    let why = corrupt(Pool::groth16(dir.path()));
    assert!(why.contains("missing") && why.contains("alice"), "{why}");
    assert_eq!(fs::read(wallet_file(dir.path(), "alice")).unwrap(), before, "the notes are kept");

    zk_pool::remove_pool(dir.path()).unwrap();
    let pool = Pool::groth16(dir.path()).unwrap();
    assert!(pool.wallet_names().is_empty() && pool.ledger().transactions.is_empty());
}

#[test]
fn totals_that_do_not_follow_from_the_transactions_are_refused() {
    let dir = funded("totals");
    let ledger = ledger_file(dir.path());
    let honest = fs::read(&ledger).unwrap();
    let edits: [Edit; 5] = [
        ("an emptied nullifier set", |v| v["nullifiers"] = Value::Array(Vec::new())),
        ("a raised turnstile", |v| v["pool_balance"] = 1_000.into()),
        ("a transparent balance at the limit", |v| v["transparent"]["alice"] = u64::MAX.into()),
        ("a renumbered transaction", |v| v["transactions"][1]["index"] = 7.into()),
        ("a nullifier that is not hex", |v| {
            v["transactions"][1]["shielded"]["nullifier"] = "가나다".into();
            v["nullifiers"][0] = "가나다".into();
        }),
    ];
    for (label, change) in edits {
        fs::write(&ledger, &honest).unwrap();
        edit(&ledger, change);
        let why = corrupt(Pool::groth16(dir.path()));
        println!("{label}: {why}");
    }
    fs::write(&ledger, &honest).unwrap();
    assert_eq!(Pool::groth16(dir.path()).unwrap().wallet("alice").unwrap().balance, 30);
}

#[test]
fn a_note_edited_in_its_wallet_is_refused_rather_than_shown() {
    let dir = funded("note");
    let wallet = wallet_file(dir.path(), "alice");
    let honest = fs::read(&wallet).unwrap();
    edit(&wallet, |v| v["notes"][0]["value"] = 65_535.into());
    assert!(corrupt(Pool::groth16(dir.path())).contains("does not open"));
    fs::write(&wallet, &honest).unwrap();
    edit(&wallet, |v| {
        let note = v["notes"][0].clone();
        v["notes"].as_array_mut().unwrap().push(note);
    });
    assert!(corrupt(Pool::groth16(dir.path())).contains("two notes"));
}

#[test]
fn the_word_that_creates_wallets_cannot_name_one() {
    let dir = ScratchDir::new("reserved");
    let mut pool = Pool::groth16(dir.path()).unwrap();
    assert_eq!(pool.new_wallet("new").unwrap_err(), PoolError::ReservedName("new".into()));
    assert!(pool.new_wallet("newt").is_ok());
}
