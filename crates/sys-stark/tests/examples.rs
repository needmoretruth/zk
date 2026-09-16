//! Every statement through the shared harness: lambdaworks' verifier accepts the honest proof, every
//! attack fails, and because the prover does not hide the trace the scan finds the secrets it can
//! search for.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_stark::Stark;
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId, leak: SecretScan) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Stark.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("constraints"),
        count("columns"),
        count("rows"),
        ms(report.timings.setup_micros),
        ms(report.timings.prove_micros),
        ms(report.timings.verify_micros),
        report.proof_bytes,
        outcomes.join(" / "),
        report.secret_scan,
    );
    assert_eq!(report.verdict, Verdict::Accepted);
    assert_eq!(report.attacks.len(), AttackKind::ALL.len());
    assert!(report.attacks.iter().all(|attack| attack.outcome.held()), "{:?}", report.attacks);
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.outcome, AttackOutcome::Rejected);
    assert_eq!(report.secret_scan, leak);
}

/// Milliseconds with two decimals, so short steps do not print as 0.
fn ms(micros: u64) -> String {
    format!("{:.2}", micros as f64 / 1000.0)
}

fn found(names: &[&str]) -> SecretScan {
    SecretScan::Found(names.iter().map(|name| (*name).to_string()).collect())
}

#[test]
fn one_plus_one_leaks_the_salt() {
    check(ExampleId::OnePlusOne, found(&["salt"]));
}

#[test]
fn password_leaks_the_pin() {
    check(ExampleId::Password, found(&["pin"]));
}

#[test]
fn sudoku_is_inconclusive() {
    check(ExampleId::Sudoku, SecretScan::Inconclusive);
}

#[test]
fn age_leaks_the_salt() {
    check(ExampleId::Age, found(&["salt"]));
}

#[test]
fn membership_leaks_the_secret_and_the_siblings() {
    check(
        ExampleId::Membership,
        found(&["secret", "sibling_0", "sibling_1", "sibling_2", "sibling_3"]),
    );
}

#[test]
fn factoring_is_inconclusive() {
    check(ExampleId::Factoring, SecretScan::Inconclusive);
}

#[test]
fn pool_spend_leaks_every_large_secret() {
    let large = [
        "sk",
        "rcm_in",
        "sibling_1",
        "sibling_2",
        "sibling_3",
        "sibling_4",
        "sibling_5",
        "sibling_6",
        "sibling_7",
        "pk_out_1",
        "pk_out_2",
    ];
    check(ExampleId::PoolSpend, found(&large));
}

/// Fails when an eighth statement is added, so it cannot go unexamined here.
#[test]
fn the_list_above_covers_every_example() {
    assert_eq!(ExampleId::ALL.len(), 7);
}
