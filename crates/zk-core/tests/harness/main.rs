//! The harness against small stand-in systems whose behaviour is known: one sound, one that leaks
//! its secrets into the proof, one whose verifier accepts anything, and one interactive.

#![allow(clippy::unwrap_used, clippy::expect_used)]

// Only the BN254 field is used here; the shared file also defines BabyBear.
#[allow(dead_code)]
#[path = "../../../zk-circuit/tests/circuit/fields.rs"]
mod fields;
mod mocks;

use fields::Bn254;
use mocks::{Behaviour, Mock};
use zk_core::{
    AttackKind, AttackOutcome, Control, ExampleId, ProofSystem, RunError, RunOptions, RunReport,
    SecretScan, Verdict,
};

const SEED: [u8; 32] = [7; 32];

fn run(behaviour: Behaviour, example: ExampleId) -> RunReport {
    let options = RunOptions { seed: Some(SEED), attacks: true };
    Mock::<Bn254>::new(behaviour).run(example, &options, &Control::new()).unwrap()
}

fn outcome(report: &RunReport, kind: AttackKind) -> &AttackOutcome {
    &report.attacks.iter().find(|attack| attack.kind == kind).unwrap().outcome
}

#[test]
fn a_sound_system_verifies_the_honest_claim_and_holds_against_all_three_attacks() {
    let report = run(Behaviour::Sound, ExampleId::OnePlusOne);
    assert_eq!(report.verdict, Verdict::Accepted);
    assert_eq!(report.attacks.len(), 3);
    assert_eq!(outcome(&report, AttackKind::FlipProofByte), &AttackOutcome::Rejected);
    assert_eq!(outcome(&report, AttackKind::BumpPublicInput), &AttackOutcome::Rejected);
    assert!(matches!(
        outcome(&report, AttackKind::DishonestWitness),
        AttackOutcome::ProverRefused(why) if why.contains("answer equals 1 + 1")
    ));
    assert!(report.sound());
    assert_eq!(report.secret_scan, SecretScan::NotFound);
}

#[test]
fn a_verifier_that_accepts_anything_is_reported_as_broken() {
    let report = run(Behaviour::AcceptsAnything, ExampleId::OnePlusOne);
    assert_eq!(outcome(&report, AttackKind::FlipProofByte), &AttackOutcome::Accepted);
    assert_eq!(outcome(&report, AttackKind::BumpPublicInput), &AttackOutcome::Accepted);
    assert_eq!(outcome(&report, AttackKind::DishonestWitness), &AttackOutcome::Accepted);
    assert!(!report.sound());
}

#[test]
fn a_proof_that_carries_its_secrets_is_caught_by_the_scan() {
    let report = run(Behaviour::Leaky, ExampleId::OnePlusOne);
    assert_eq!(report.secret_scan, SecretScan::Found(vec!["answer".into(), "salt".into()]));
}

#[test]
fn the_flip_lands_on_the_byte_the_system_chose() {
    let report = run(Behaviour::Sound, ExampleId::Sudoku);
    let flip = report.attacks.iter().find(|a| a.kind == AttackKind::FlipProofByte).unwrap();
    assert_eq!(flip.offset, Some(mocks::TAMPER_OFFSET));
}

#[test]
fn a_fresh_seed_gives_the_envelope_a_fresh_salt() {
    let options = RunOptions { seed: Some([1; 32]), attacks: false };
    let one =
        Mock::<Bn254>::new(Behaviour::Sound).run(ExampleId::OnePlusOne, &options, &Control::new());
    let options = RunOptions { seed: Some([2; 32]), attacks: false };
    let two =
        Mock::<Bn254>::new(Behaviour::Sound).run(ExampleId::OnePlusOne, &options, &Control::new());
    assert_ne!(one.unwrap().proof_head, two.unwrap().proof_head);
}

#[test]
fn an_interactive_system_plays_the_conversation_and_has_no_proof_byte_to_flip() {
    let report = run(Behaviour::Interactive, ExampleId::Age);
    assert_eq!(report.verdict, Verdict::Accepted);
    assert_eq!(report.rounds, Some(mocks::ROUNDS));
    assert!(matches!(outcome(&report, AttackKind::FlipProofByte), AttackOutcome::NotApplicable(_)));
    assert_eq!(outcome(&report, AttackKind::BumpPublicInput), &AttackOutcome::Rejected);
    assert_eq!(outcome(&report, AttackKind::DishonestWitness), &AttackOutcome::Rejected);
    assert_eq!(report.timings.verify_micros, 0);
}

#[test]
fn a_cancelled_run_says_it_stopped_instead_of_failing() {
    let control = Control::new();
    control.cancel();
    let result = Mock::<Bn254>::new(Behaviour::Sound).run(
        ExampleId::OnePlusOne,
        &RunOptions::default(),
        &control,
    );
    assert_eq!(result.unwrap_err(), RunError::Cancelled { after: None });
}

#[test]
fn an_unsupported_example_is_refused_before_any_work() {
    let result = Mock::<Bn254>::new(Behaviour::Sound).run(
        ExampleId::PoolSpend,
        &RunOptions::default(),
        &Control::new(),
    );
    assert_eq!(result.unwrap_err(), RunError::Unsupported { reason: mocks::UNSUPPORTED });
}

#[test]
fn a_report_survives_json_so_a_companion_process_can_send_it() {
    let report = run(Behaviour::Sound, ExampleId::Membership);
    let json = serde_json::to_string(&report).unwrap();
    let back: RunReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back, report);
}

#[test]
fn progress_reports_every_stage_in_order() {
    use std::sync::{Arc, Mutex};
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let control = Control::with_progress(move |stage| sink.lock().unwrap().push(stage));
    let options = RunOptions { seed: Some(SEED), attacks: true };
    Mock::<Bn254>::new(Behaviour::Sound).run(ExampleId::Factoring, &options, &control).unwrap();
    use zk_core::Stage;
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            Stage::Setup,
            Stage::Prove,
            Stage::Verify,
            Stage::Attack(AttackKind::FlipProofByte),
            Stage::Attack(AttackKind::BumpPublicInput),
            Stage::Attack(AttackKind::DishonestWitness),
        ]
    );
}
