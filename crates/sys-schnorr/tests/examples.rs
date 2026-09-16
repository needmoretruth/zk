//! Every statement through the shared harness: `sigma-proofs` accepts the honest proof, every attack
//! fails, and no secret appears in the proof bytes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_schnorr::Schnorr;
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Schnorr.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} / {} / {} | {:.1} | {:.1} | {:.1} | {} | {} | {:?} |",
        report.example,
        count("commitments"),
        count("constraints"),
        count("variables"),
        report.timings.setup_micros as f64 / 1000.0,
        report.timings.prove_micros as f64 / 1000.0,
        report.timings.verify_micros as f64 / 1000.0,
        report.proof_bytes,
        outcomes.join(" / "),
        report.secret_scan,
    );
    assert_eq!(report.verdict, Verdict::Accepted, "{example:?}");
    assert_eq!(report.attacks.len(), AttackKind::ALL.len());
    for attack in &report.attacks {
        assert_eq!(attack.outcome, AttackOutcome::Rejected, "{example:?} {:?}", attack.kind);
    }
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.offset, Some(report.proof_bytes - 1), "{example:?}");
    assert_eq!(report.secret_scan, hiding_scan(&report.example), "{example:?}");
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
