//! Every statement through the shared harness: Winterfell's verifier accepts the honest proof, every
//! attack fails, and because Winterfell is not zero-knowledge the scan finds the secrets it can search
//! for. The two statements too wide for a Winterfell trace are refused before anything runs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_winterfell::{TRACE_TOO_WIDE, Winterfell};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunError, RunOptions, SecretScan,
    Support, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId, leak: SecretScan) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Winterfell.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("constraints"),
        count("columns"),
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

/// Milliseconds with two decimals: these proofs take a few milliseconds, and whole ones print as 0.
fn ms(micros: u64) -> String {
    format!("{:.2}", micros as f64 / 1000.0)
}

fn found(names: &[&str]) -> SecretScan {
    SecretScan::Found(names.iter().map(|name| (*name).to_string()).collect())
}

fn refused(example: ExampleId) {
    assert_eq!(Winterfell.support(example), Support::Unsupported { reason: TRACE_TOO_WIDE });
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let result = Winterfell.run(example, &options, &Control::new());
    assert_eq!(result.unwrap_err(), RunError::Unsupported { reason: TRACE_TOO_WIDE });
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
fn membership_is_too_wide() {
    refused(ExampleId::Membership);
}

#[test]
fn factoring_is_inconclusive() {
    check(ExampleId::Factoring, SecretScan::Inconclusive);
}

#[test]
fn pool_spend_is_too_wide() {
    refused(ExampleId::PoolSpend);
}

/// Fails when an eighth statement is added, so it cannot go unexamined here.
#[test]
fn the_list_above_covers_every_example() {
    assert_eq!(ExampleId::ALL.len(), 7);
}
