//! Every statement through the shared harness: the proof verifies, every attack is rejected by the
//! verifier, no secret appears in the proof bytes, and the proof has the documented length.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_ligero::{Ligero, OPENED_COLUMNS, Statement, proof, tamper_offset};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Ligero.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let params = Statement::new(example).unwrap().params();
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("constraints"),
        count("variables"),
        count("quadratic-constraints"),
        count("linear-constraints"),
        params.message_length,
        params.dimension,
        params.length,
        params.rows(),
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
    // The flipped byte is a committed column entry, the bumped input changes `b` and the false
    // claim breaks a constraint: all three proofs still decode, and the verifier says no.
    for attack in &report.attacks {
        assert_eq!(attack.outcome, AttackOutcome::Rejected, "{:?}", attack.kind);
    }
    assert_eq!(report.attacks[0].offset, Some(tamper_offset(&params) as u64));
    assert_eq!(report.secret_scan, hiding_scan(&report.example));
    // The documented byte format: a fixed part, then whole sibling hashes, at most t·log₂ n of them.
    let siblings = (report.proof_bytes as usize).checked_sub(proof::fixed_bytes(&params)).unwrap();
    assert_eq!(siblings % 32, 0);
    assert!(siblings / 32 <= OPENED_COLUMNS * params.length.trailing_zeros() as usize);
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
