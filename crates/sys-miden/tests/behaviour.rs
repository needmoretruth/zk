//! What the adapter itself promises around Miden VM: proofs that give their witness away to a
//! guess, a proof bound to its program, caller-supplied assignments, Goldilocks arithmetic on
//! public inputs, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_miden::{Field, MidenVm};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

/// Goldilocks' modulus, `2^64 − 2^32 + 1`.
const MODULUS: u64 = 18_446_744_069_414_584_321;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

/// No hiding, shown without searching bytes: `n = p·q = q·p`, so `(p, q)` and `(q, p)` prove the
/// same public claim, yet their proofs differ, and proving a guessed witness again reproduces its
/// proof byte for byte. A verifier holding a proof learns which witness was used by trying each.
#[test]
fn a_guessed_witness_is_confirmed_by_proving_it_again() {
    let control = Control::new();
    let mut prepared = MidenVm.prepare(ExampleId::Factoring, &control).unwrap();
    let instance =
        Instance { example: ExampleId::Factoring, kind: InstanceKind::Honest, seed: [3; 32] };
    let sample = prepared.prove(&instance, &control).unwrap();
    let swapped: Vec<_> = sample.secrets.iter().rev().cloned().collect();
    assert_ne!(swapped, sample.secrets);
    let other = prepared.prove_assignment(&sample.public, &swapped, &control).unwrap();
    assert_eq!(other.public, sample.public);
    assert_eq!(prepared.verify(&other.public, &other.proof, &control).unwrap(), Verdict::Accepted);
    assert_ne!(other.proof, sample.proof);
    let guess = prepared.prove_assignment(&sample.public, &sample.secrets, &control).unwrap();
    assert_eq!(guess.proof, sample.proof);
}

#[test]
fn a_proof_is_rejected_for_another_program_with_the_same_inputs() {
    let control = Control::new();
    let mut one_plus_one = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut password = MidenVm.prepare(ExampleId::Password, &control).unwrap();
    let proven = one_plus_one.prove(&honest(), &control).unwrap();
    let verdict = password.verify(&proven.public, &proven.proof, &control).unwrap();
    assert_eq!(verdict, Verdict::Rejected);
}

#[test]
fn a_false_claim_stops_the_vm_at_the_assertion_it_breaks() {
    let control = Control::new();
    let mut prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let dishonest = Instance { kind: InstanceKind::Dishonest, ..honest() };
    let refused = prepared.prove(&dishonest, &control);
    let Err(SystemError::Unsatisfied(why)) = refused else { panic!("not refused: {refused:?}") };
    assert!(why.ends_with("answer equals 1 + 1"), "{why}");
}

#[test]
fn an_assignment_given_as_bytes_is_proved_and_verifies() {
    let control = Control::new();
    let mut prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&honest(), &control).unwrap();
    let mut private = sample.secrets.clone();
    private[0] = MODULUS.to_le_bytes().to_vec();
    let refused = prepared.prove_assignment(&sample.public, &private, &control);
    assert!(matches!(refused, Err(SystemError::Failed(_))), "{refused:?}");
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let largest = (MODULUS - 1).to_le_bytes().to_vec();
    assert_eq!(prepared.bump_public(&largest).unwrap(), Field::zero().to_le_bytes());
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = MidenVm.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
