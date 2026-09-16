//! The museum harness against BCTV14: every example proves, verifies, holds against all three
//! attacks, and leaks no secret. Pool-spend is the heavy one, so it gets its own test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_bctv14::Bctv14;
use zk_core::{Control, ExampleId, ProofSystem, RunOptions, RunReport, SecretScan, Verdict};

const SEED: [u8; 32] = [42; 32];

fn run(example: ExampleId) -> RunReport {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    Bctv14.run(example, &options, &Control::new()).expect("run should produce a report")
}

fn assert_sound_and_hiding(report: &RunReport) {
    assert_eq!(report.verdict, Verdict::Accepted, "honest proof must verify");
    assert_eq!(report.attacks.len(), 3, "all three attacks must run");
    for attack in &report.attacks {
        assert!(attack.outcome.held(), "attack {:?} must be held", attack.kind);
    }
    assert!(report.sound());
    assert_eq!(
        report.secret_scan,
        hiding_scan(&report.example),
        "a zero-knowledge proof leaks no secret"
    );
}

#[test]
fn every_light_example_is_sound_and_hiding() {
    for example in ExampleId::ALL {
        if example == ExampleId::PoolSpend {
            continue;
        }
        let report = run(example);
        assert_sound_and_hiding(&report);
    }
}

#[test]
fn pool_spend_is_sound_and_hiding() {
    let report = run(ExampleId::PoolSpend);
    assert_sound_and_hiding(&report);
}

#[test]
fn proof_is_the_constant_288_bytes() {
    let report = run(ExampleId::OnePlusOne);
    assert_eq!(report.proof_bytes, 288, "seven G1 (32 B) and one G2 (64 B) compressed");
    assert!(report.setup_bytes.is_some(), "the per-circuit keys have a meaningful size");
}

#[test]
fn the_flip_lands_inside_the_first_proof_element() {
    let report = run(ExampleId::Sudoku);
    let flip =
        report.attacks.iter().find(|a| a.kind == zk_core::AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.offset, Some(1), "byte 1 sits inside pi_A, which the verifier reads");
    assert!(flip.outcome.held());
}

#[test]
fn a_caller_supplied_assignment_is_proved_without_being_checked() {
    use zk_core::{Instance, InstanceKind};
    let control = Control::new();
    let mut prepared = Bctv14.prepare(ExampleId::OnePlusOne, &control).unwrap();
    for (kind, expected) in
        [(InstanceKind::Honest, Verdict::Accepted), (InstanceKind::Dishonest, Verdict::Rejected)]
    {
        let instance = Instance { example: ExampleId::OnePlusOne, kind, seed: SEED };
        let sample = prepared.prove(&instance, &control).unwrap();
        let proven = prepared.prove_assignment(&sample.public, &sample.secrets, &control).unwrap();
        assert_eq!(proven.secrets, sample.secrets);
        assert_eq!(prepared.verify(&proven.public, &proven.proof, &control).unwrap(), expected);
    }
}

/// What the leak scan reports for a hiding proof: nothing found, except on the statements whose
/// secrets are all below 2^16, which the scan does not search for.
fn hiding_scan(example: &str) -> SecretScan {
    match example {
        "sudoku" | "factoring" => SecretScan::Inconclusive,
        _ => SecretScan::NotFound,
    }
}
