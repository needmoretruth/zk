//! Every statement through the shared harness on Bulletproofs: the honest proof verifies, the upstream
//! verifier (not the adapter, not the prover) turns down all three attacks, and no secret shows up in
//! the proof.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_bulletproofs::{Bulletproofs, TAMPER_OFFSET};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunOptions, RunReport, SecretScan,
    Verdict,
};

const SEED: [u8; 32] = [42; 32];

fn assert_sound(example: ExampleId) {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    let report = Bulletproofs.run(example, &options, &Control::new()).unwrap();
    println!("{}", summary(&report));
    assert_eq!(report.verdict, Verdict::Accepted, "{example:?}");
    assert_eq!(report.proof_bytes, logarithmic_size(&report), "{example:?}");
    assert_eq!(report.attacks.len(), AttackKind::ALL.len());
    for attack in &report.attacks {
        assert_eq!(attack.outcome, AttackOutcome::Rejected, "{example:?} {:?}", attack.kind);
    }
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.offset, Some(TAMPER_OFFSET as u64), "{example:?}");
    assert_eq!(report.secret_scan, SecretScan::NotFound, "{example:?}");
}

/// A version byte, eleven 32-byte elements, `2·log2(n)` inner-product points and two final scalars,
/// with `n` the multiplier count rounded up to a power of two (upstream's `R1CSProof::to_bytes`).
fn logarithmic_size(report: &RunReport) -> u64 {
    let multipliers = report.shape.counts.iter().find(|(name, _)| name == "multipliers").unwrap().1;
    let rounds = u64::from(multipliers.next_power_of_two().trailing_zeros());
    1 + 32 * (11 + 2 * rounds + 2)
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

/// Fails when an eighth statement is added, so it cannot go unproved here.
#[test]
fn the_list_above_covers_every_example() {
    assert_eq!(ExampleId::ALL.len(), 7);
}
