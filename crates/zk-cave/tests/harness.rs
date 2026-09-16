//! Every statement through the shared harness: Mick convinces the reporter in forty scenes, a
//! false claim is caught, there is no proof object to flip, and the tape carries no secret.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_cave::{Cave, Goldilocks, MagicWords, Wall};
use zk_circuit::ZkField;
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, InstanceKind, ProofSystem, RunOptions,
    RunReport, SecretScan, Verdict,
};

const SEED: [u8; 32] = [42; 32];

/// The tape's 9-byte header plus two letters for each of the forty scenes.
const TAPE_BYTES: u64 = 9 + 2 * 40;

fn check(example: ExampleId, bumped: AttackOutcome) -> RunReport {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Cave.run(example, &options, &Control::new()).unwrap();
    let outcome = |kind| &report.attacks.iter().find(|a| a.kind == kind).unwrap().outcome;
    let outcomes: Vec<_> = report.attacks.iter().map(|a| format!("{:?}", a.outcome)).collect();
    println!(
        "| {} | {} | {} µs | {} µs | {} | {} | {} | {:?} |",
        report.example,
        report.shape.counts[0].1,
        report.timings.setup_micros,
        report.timings.prove_micros,
        report.rounds.unwrap(),
        report.proof_bytes,
        outcomes.join(" / "),
        report.secret_scan,
    );
    assert_eq!(report.verdict, Verdict::Accepted);
    assert_eq!(report.rounds, Some(40));
    assert_eq!(report.proof_bytes, TAPE_BYTES);
    assert!(matches!(outcome(AttackKind::FlipProofByte), AttackOutcome::NotApplicable(_)));
    assert_eq!(outcome(AttackKind::BumpPublicInput), &bumped);
    assert_eq!(outcome(AttackKind::DishonestWitness), &AttackOutcome::Rejected);
    assert_eq!(report.secret_scan, SecretScan::NotFound);
    report
}

#[test]
fn one_plus_one() {
    assert!(check(ExampleId::OnePlusOne, AttackOutcome::Rejected).sound());
}

#[test]
fn password() {
    assert!(check(ExampleId::Password, AttackOutcome::Rejected).sound());
}

#[test]
fn sudoku() {
    assert!(check(ExampleId::Sudoku, AttackOutcome::Rejected).sound());
}

/// The attack moves `age`'s credential, not its year (see `ExampleId::falsifying_public_index`).
#[test]
fn age() {
    assert!(check(ExampleId::Age, AttackOutcome::Rejected).sound());
}

/// Why the attack does not move the year: one year later the same holder is still old enough, so
/// that claim is true, Mick's words open its wall, and a live demonstration is right to convince.
#[test]
fn age_with_the_year_bumped_is_still_a_true_claim() {
    let claim = ExampleId::Age.instance::<Goldilocks>(InstanceKind::Honest, &SEED);
    let mut next_year = claim.public.clone();
    next_year[0] = next_year[0].add(Goldilocks::one());
    let wall = Wall::for_example(ExampleId::Age, next_year).unwrap();
    assert!(wall.whisper(&MagicWords::new(claim.private)));
}

#[test]
fn membership() {
    assert!(check(ExampleId::Membership, AttackOutcome::Rejected).sound());
}

#[test]
fn factoring() {
    assert!(check(ExampleId::Factoring, AttackOutcome::Rejected).sound());
}

#[test]
fn pool_spend() {
    assert!(check(ExampleId::PoolSpend, AttackOutcome::Rejected).sound());
}
