//! Every statement through the shared harness on PLONK: the honest proof verifies, lambdaworks'
//! verifier (not the adapter, not the prover) turns down all three attacks, and no secret shows up
//! in the proof.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_plonk::{Plonk, TAMPER_OFFSET};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, RunReport, SecretScan,
    Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn assert_sound(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Plonk.run(example, &options, &Control::new()).unwrap();
    println!("{}", summary(&report));
    assert_eq!(report.verdict, Verdict::Accepted, "{example:?}");
    assert_eq!(report.attacks.len(), AttackKind::ALL.len());
    for attack in &report.attacks {
        assert_eq!(attack.outcome, AttackOutcome::Rejected, "{example:?} {:?}", attack.kind);
    }
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.offset, Some(TAMPER_OFFSET as u64), "{example:?}");
    assert_eq!(report.secret_scan, SecretScan::NotFound, "{example:?}");
}

/// One line per run with the numbers the exhibit's report table needs.
fn summary(report: &RunReport) -> String {
    let t = report.timings;
    format!(
        "{} counts={:?} setup_ms={:.1} prove_ms={:.1} verify_ms={:.1} setup_bytes={:?} proof_bytes={} attacks={:?} scan={:?}",
        report.example,
        report.shape.counts,
        t.setup_micros as f64 / 1000.0,
        t.prove_micros as f64 / 1000.0,
        t.verify_micros as f64 / 1000.0,
        report.setup_bytes,
        report.proof_bytes,
        report.attacks.iter().map(|a| (a.kind, &a.outcome)).collect::<Vec<_>>(),
        report.secret_scan,
    )
}

#[test]
fn one_plus_one() {
    assert_sound(ExampleId::OnePlusOne);
}

#[test]
fn password() {
    assert_sound(ExampleId::Password);
}

#[test]
fn sudoku() {
    assert_sound(ExampleId::Sudoku);
}

#[test]
fn age() {
    assert_sound(ExampleId::Age);
}

#[test]
fn membership() {
    assert_sound(ExampleId::Membership);
}

#[test]
fn factoring() {
    assert_sound(ExampleId::Factoring);
}

#[test]
fn pool_spend() {
    assert_sound(ExampleId::PoolSpend);
}
