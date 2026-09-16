//! What the adapter itself promises around `sigma-proofs`: fresh prover randomness, undecodable bytes
//! reported as malformed, caller-supplied assignments proved as given, and the bare Schnorr proof.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_schnorr::schnorr::{prove_dlog, verify_dlog};
use sys_schnorr::{Field, Schnorr};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, Verdict};
use zk_examples::one_plus_one::sealed;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Schnorr.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Schnorr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

/// Cutting the last byte leaves the final response scalar one byte short.
#[test]
fn a_truncated_proof_is_malformed() {
    let control = Control::new();
    let mut prepared = Schnorr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.pop();
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}

/// The Toy Shielded Pool proves its own notes this way: the adapter proves exactly the values it is
/// handed, and a false assignment still becomes a proof for the verifier to turn down.
#[test]
fn a_caller_supplied_assignment_is_proved_as_given_and_never_pre_checked() {
    let control = Control::new();
    let mut prepared = Schnorr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let encode = |values: &[Field]| values.iter().map(|v| v.to_le_bytes()).collect::<Vec<_>>();
    let true_claim = sealed::<Field>(2, 424_242);
    let proven = prepared
        .prove_assignment(&encode(&true_claim.public), &encode(&true_claim.private), &control)
        .unwrap();
    assert_eq!(proven.public, encode(&true_claim.public));
    assert_eq!(
        prepared.verify(&proven.public, &proven.proof, &control).unwrap(),
        Verdict::Accepted
    );
    let false_claim = sealed::<Field>(3, 424_242);
    let forged = prepared
        .prove_assignment(&encode(&false_claim.public), &encode(&false_claim.private), &control)
        .unwrap();
    assert_eq!(
        prepared.verify(&forged.public, &forged.proof, &control).unwrap(),
        Verdict::Rejected
    );
}

#[test]
fn a_schnorr_proof_of_a_discrete_logarithm_verifies() {
    let (public, proof) = prove_dlog(Field::from_u64(0x5eed_1234_abcd)).unwrap();
    assert_eq!(public.len(), 32);
    assert_eq!(proof.len(), 64);
    assert!(verify_dlog(&public, &proof));
}

#[test]
fn a_schnorr_proof_does_not_verify_for_a_different_public_point() {
    let (_, proof) = prove_dlog(Field::from_u64(0x5eed_1234_abcd)).unwrap();
    let (other, _) = prove_dlog(Field::from_u64(0x5eed_1234_abce)).unwrap();
    assert!(!verify_dlog(&other, &proof));
}
