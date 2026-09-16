//! What the adapter itself promises around the upstream crate: live blinding, public inputs bound to
//! the proof, caller-supplied assignments, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_bulletproofs::{Bulletproofs, Field};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

fn instance(kind: InstanceKind) -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind, seed: [3; 32] }
}

/// Without the prover's blinding, two proofs of one claim would be byte-identical.
#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let second = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    assert_eq!(first.public, second.public);
    assert_ne!(first.proof, second.proof);
    assert_eq!(prepared.verify(&first.public, &first.proof, &control).unwrap(), Verdict::Accepted);
    assert_eq!(
        prepared.verify(&second.public, &second.proof, &control).unwrap(),
        Verdict::Accepted
    );
}

#[test]
fn a_caller_supplied_assignment_is_proved_without_being_checked() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    for (kind, expected) in
        [(InstanceKind::Honest, Verdict::Accepted), (InstanceKind::Dishonest, Verdict::Rejected)]
    {
        let sample = prepared.prove(&instance(kind), &control).unwrap();
        let private = sample.secrets.clone();
        let proven = prepared.prove_assignment(&sample.public, &private, &control).unwrap();
        assert_eq!(proven.public, sample.public);
        assert_eq!(proven.secrets, private);
        assert_eq!(prepared.verify(&proven.public, &proven.proof, &control).unwrap(), expected);
    }
}

#[test]
fn a_caller_supplied_input_at_or_above_the_group_order_is_refused() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let outcome = prepared.prove_assignment(&[vec![0xff; 32]], &sample.secrets, &control);
    assert!(matches!(outcome, Err(SystemError::Failed(_))), "{outcome:?}");
}

#[test]
fn a_public_input_at_or_above_the_group_order_is_malformed() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

#[test]
fn a_wrong_number_of_public_inputs_is_malformed() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.public.push(proven.public[0].clone());
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn a_proof_cut_short_is_malformed() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.proof.truncate(proven.proof.len() - 1);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}

/// Upstream also reads a one-phase proof written in the two-phase layout, with the three second-phase
/// commitments set to the identity, and that proof verifies. Only the bytes the prover wrote count.
#[test]
fn the_same_proof_in_the_two_phase_layout_is_malformed() {
    let control = Control::new();
    let mut prepared = Bulletproofs.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let mut rewritten = vec![1u8];
    rewritten.extend_from_slice(&proven.proof[1..1 + 3 * 32]);
    rewritten.extend_from_slice(&[0u8; 3 * 32]);
    rewritten.extend_from_slice(&proven.proof[1 + 3 * 32..]);
    let verdict = prepared.verify(&proven.public, &rewritten, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}
