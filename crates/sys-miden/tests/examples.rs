//! Every statement through the shared harness: Miden's verifier accepts the honest proof, every
//! attack fails, a false claim stops the VM before any proof exists, and no searched secret appears
//! verbatim in the proof bytes, even though the proofs do not hide (see `behaviour.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_miden::MidenVm;
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = MidenVm.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("program-operations"),
        count("trace-rows"),
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
    let outcome = |kind| &report.attacks.iter().find(|a| a.kind == kind).unwrap().outcome;
    assert_eq!(outcome(AttackKind::FlipProofByte), &AttackOutcome::Rejected);
    assert_eq!(outcome(AttackKind::BumpPublicInput), &AttackOutcome::Rejected);
    assert!(
        matches!(outcome(AttackKind::DishonestWitness), AttackOutcome::ProverRefused(_)),
        "{:?}",
        report.attacks
    );
    assert_eq!(report.secret_scan, scan(example));
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

/// What the leak scan reports: no searched secret appears in a Miden proof, and `sudoku` and
/// `factoring`, whose secrets are all below 2^16 and never searched, are inconclusive.
fn scan(example: ExampleId) -> SecretScan {
    match example {
        ExampleId::Sudoku | ExampleId::Factoring => SecretScan::Inconclusive,
        _ => SecretScan::NotFound,
    }
}
