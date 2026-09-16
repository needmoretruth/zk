//! Every statement through the shared harness: the circle STARK's verifier accepts the honest
//! proof, every attack fails, and without hiding every secret the scan searches for is in the proof.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_circle_stark::{CircleStark, Field};
use zk_circuit::ZkField;
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, InstanceKind, ProofSystem, RunOptions,
    SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn check(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = CircleStark.run(example, &options, &Control::new()).unwrap();
    let count = |name: &str| report.shape.counts.iter().find(|(n, _)| n == name).unwrap().1;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:?} |",
        report.example,
        count("constraints"),
        count("columns"),
        count("rows"),
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
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.outcome, AttackOutcome::Rejected);
    assert_eq!(report.secret_scan, leaking_scan(example));
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

/// What the leak scan reports for a proof that does not hide: every secret it searches for is
/// found, and `sudoku` and `factoring`, whose secrets are all below 2^16 and never searched, are
/// inconclusive.
fn leaking_scan(example: ExampleId) -> SecretScan {
    match example {
        ExampleId::Sudoku | ExampleId::Factoring => SecretScan::Inconclusive,
        _ => SecretScan::Found(searched_names(example)),
    }
}

/// The names of the honest instance's private inputs of 2^16 or more, in declaration order and
/// without repeats, as the scan lists what it finds.
fn searched_names(example: ExampleId) -> Vec<String> {
    let assignment = example.instance::<Field>(InstanceKind::Honest, &SEED);
    let names = example.private_input_names();
    let mut searched = Vec::new();
    for (value, name) in assignment.private.iter().zip(names) {
        let small = value.to_le_bytes().iter().skip(2).all(|byte| *byte == 0);
        if !small && !searched.contains(&name) {
            searched.push(name);
        }
    }
    searched
}
