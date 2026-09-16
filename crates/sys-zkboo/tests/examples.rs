//! Every statement through the shared harness: the 137-round proof verifies, every attack fails,
//! and no secret appears in the proof bytes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_zkboo::{PROOF_ROUNDS, Statement, ZkBoo, proof};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = ZkBoo.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("gates"),
        count("multiplications"),
        count("assertions"),
        count("witness values"),
        report.timings.setup_micros / 1000,
        report.timings.prove_micros / 1000,
        report.timings.verify_micros / 1000,
        report.proof_bytes,
        outcomes.join(" / "),
        report.secret_scan,
    );
    assert_eq!(report.verdict, Verdict::Accepted);
    let kinds: Vec<_> = report.attacks.iter().map(|attack| attack.kind).collect();
    assert_eq!(kinds, AttackKind::ALL);
    for attack in &report.attacks {
        assert!(attack.outcome.held(), "{:?} was accepted", attack.kind);
    }
    // The flipped byte is a seed the verifier hashes, so the proof still decodes and is rejected;
    // the false claim decodes too, and its output shares do not sum to zero.
    let statement = Statement::new(example).unwrap();
    assert_eq!(report.attacks[0].offset, Some(proof::tamper_offset(&statement) as u64));
    assert_eq!(report.attacks[0].outcome, AttackOutcome::Rejected);
    assert_eq!(report.attacks[2].outcome, AttackOutcome::Rejected);
    assert_eq!(report.secret_scan, hiding_scan(&report.example));
    // The documented byte format: every response lies between one that leaves P3 closed and one
    // that opens it.
    let (m, s, a) = (count("multiplications"), count("witness values"), count("assertions"));
    let rounds = u64::from(PROOF_ROUNDS);
    let smallest = 4 + rounds * (96 + 24 * a + 64 + 16 * m);
    let largest = smallest + rounds * 8 * s;
    assert!((smallest..=largest).contains(&report.proof_bytes), "{}", report.proof_bytes);
}

#[test]
fn one_plus_one() {
    check(ExampleId::OnePlusOne);
}

#[test]
fn password() {
    check(ExampleId::Password);
}

#[test]
fn sudoku() {
    check(ExampleId::Sudoku);
}

#[test]
fn age() {
    check(ExampleId::Age);
}

#[test]
fn membership() {
    check(ExampleId::Membership);
}

#[test]
fn factoring() {
    check(ExampleId::Factoring);
}

#[test]
fn pool_spend() {
    check(ExampleId::PoolSpend);
}

/// What the leak scan reports for a hiding proof: nothing found, except on the statements whose
/// secrets are all below 2^16, which the scan does not search for.
fn hiding_scan(example: &str) -> SecretScan {
    match example {
        "sudoku" | "factoring" => SecretScan::Inconclusive,
        _ => SecretScan::NotFound,
    }
}
