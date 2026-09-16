//! Every statement through the shared harness: Remainder's Hyrax verifier accepts the honest proof,
//! every attack fails, and no secret appears in the proof bytes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_gkr::Gkr;
use zk_core::{AttackKind, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Gkr.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} / {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("layers"),
        count("gates"),
        report.timings.setup_micros / 1000,
        report.timings.prove_micros / 1000,
        report.timings.verify_micros / 1000,
        report.proof_bytes,
        outcomes.join(" / "),
        report.secret_scan,
    );
    assert_eq!(report.verdict, Verdict::Accepted);
    assert_eq!(report.attacks.len(), AttackKind::ALL.len());
    assert!(report.attacks.iter().all(|attack| attack.outcome.held()), "{:?}", report.attacks);
    assert_eq!(report.secret_scan, hiding_scan(&report.example));
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

/// Fails when an eighth statement is added, so it cannot go unproved here.
#[test]
fn the_list_above_covers_every_example() {
    assert_eq!(ExampleId::ALL.len(), 7);
}

/// What the leak scan reports for a hiding proof: nothing found, except on the statements whose
/// secrets are all below 2^16, which the scan does not search for.
fn hiding_scan(example: &str) -> SecretScan {
    match example {
        "sudoku" | "factoring" => SecretScan::Inconclusive,
        _ => SecretScan::NotFound,
    }
}
