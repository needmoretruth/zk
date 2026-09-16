//! What the adapter itself promises around Plonky3: fresh hiding randomness, caller-supplied
//! assignments, BabyBear arithmetic on public inputs, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_plonky3::{Field, Plonky3};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

/// BabyBear's modulus, `15·2^27 + 1`.
const MODULUS: u32 = 2_013_265_921;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&honest(), &control).unwrap();
    let second = prepared.prove(&honest(), &control).unwrap();
    assert_eq!(first.public, second.public);
    assert_ne!(first.proof, second.proof);
    assert_eq!(prepared.verify(&first.public, &first.proof, &control).unwrap(), Verdict::Accepted);
    assert_eq!(
        prepared.verify(&second.public, &second.proof, &control).unwrap(),
        Verdict::Accepted
    );
}

#[test]
fn an_assignment_given_as_bytes_is_proved_and_verifies() {
    let control = Control::new();
    let mut prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&honest(), &control).unwrap();
    let proven = prepared.prove_assignment(&sample.public, &sample.secrets, &control).unwrap();
    assert_eq!(proven.public, sample.public);
    assert_eq!(proven.secrets, sample.secrets);
    assert_eq!(
        prepared.verify(&proven.public, &proven.proof, &control).unwrap(),
        Verdict::Accepted
    );
}

#[test]
fn an_assignment_with_a_value_at_the_modulus_is_refused_before_proving() {
    let control = Control::new();
    let mut prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&honest(), &control).unwrap();
    let mut private = sample.secrets.clone();
    private[0] = MODULUS.to_le_bytes().to_vec();
    let refused = prepared.prove_assignment(&sample.public, &private, &control);
    assert!(matches!(refused, Err(SystemError::Failed(_))), "{refused:?}");
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let at_modulus = MODULUS.to_le_bytes().to_vec();
    let verdict =
        prepared.verify(std::slice::from_ref(&at_modulus), &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&at_modulus).is_err());
}

#[test]
fn bumping_the_largest_element_wraps_to_zero() {
    let control = Control::new();
    let prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let largest = (MODULUS - 1).to_le_bytes().to_vec();
    assert_eq!(prepared.bump_public(&largest).unwrap(), Field::zero().to_le_bytes());
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Plonky3.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
