//! Every statement through the shared harness: the 109-round proof verifies, every attack fails,
//! and no secret appears in the proof bytes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_core::{AttackKind, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict};
use zk_trio::{PROOF_ROUNDS, TAMPER_OFFSET, Trio};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Trio.run(example, &options, &Control::new()).unwrap();
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
    assert_eq!(report.attacks[0].offset, Some(TAMPER_OFFSET as u64));
    assert_eq!(report.secret_scan, hiding_scan(&report.example));
    // The documented byte format: every round's opening lies between a dealer card's and a peek
    // card's that opens P3.
    let (m, s, a) = (count("multiplications"), count("witness values"), count("assertions"));
    let rounds = u64::from(PROOF_ROUNDS);
    let smallest = 4 + rounds * (192 + 96 + 8 * m);
    let largest = 4 + rounds * (192 + 160 + 24 * m + 8 * s + 8 * a);
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
