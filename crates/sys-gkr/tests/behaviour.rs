//! What the adapter itself promises around Remainder: fresh blinding, the flip landing on a
//! commitment, the one encoding Remainder writes scalars in, false claims reaching the verifier, and
//! undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hyrax::gkr::HyraxProof;
use shared_types::config::ProofConfig;
use shared_types::{Bn256Point, Fr};
use sys_gkr::{Bn254Scalar, Field, Gkr};
use zk_circuit::ZkField;
use zk_core::{
    Control, ExampleId, Instance, InstanceKind, ProofSystem, SecretScan, Verdict, scan_secrets,
};
use zk_examples::one_plus_one::sealed;

fn claim(kind: InstanceKind) -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind, seed: [3; 32] }
}

fn encode(values: &[Field]) -> Vec<Vec<u8>> {
    values.iter().map(|v| v.to_le_bytes()).collect()
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    let second = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    assert_eq!(first.public, second.public);
    assert_ne!(first.proof, second.proof);
    assert_eq!(prepared.verify(&first.public, &first.proof, &control).unwrap(), Verdict::Accepted);
    assert_eq!(
        prepared.verify(&second.public, &second.proof, &control).unwrap(),
        Verdict::Accepted
    );
}

/// Remainder's Hyrax prover does not check the witness: the false claim becomes bytes, and the
/// verifier is the one that turns it down.
#[test]
fn a_false_claim_is_proved_and_then_rejected_by_the_verifier() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let forged = prepared.prove(&claim(InstanceKind::Dishonest), &control).unwrap();
    assert_eq!(
        prepared.verify(&forged.public, &forged.proof, &control).unwrap(),
        Verdict::Rejected
    );
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

#[test]
fn a_wrong_number_of_public_inputs_is_malformed() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    proven.public.push(proven.public[0].clone());
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// The doc comment on the offset promises the first sum-check message commitment of the first layer
/// proof sits there.
#[test]
fn the_tampered_byte_opens_the_first_sum_check_message_commitment() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&claim(InstanceKind::Honest), &control).unwrap();
    let offset = prepared.tamper_offset(&proven.proof);
    let (proof, _): (HyraxProof<Bn256Point>, ProofConfig) =
        bincode::deserialize(&proven.proof).unwrap();
    let message = proof.circuit_proof.layer_proofs[0].1.proof_of_sumcheck.messages[0];
    let point = bincode::serialize(&message).unwrap();
    assert_eq!(point.len(), 32);
    assert_eq!(&proven.proof[offset..offset + 32], point.as_slice());
}

/// halo2curves writes a scalar as its canonical little-endian bytes, which the default scan patterns
/// already cover: a witness Remainder serialized verbatim would be caught.
#[test]
fn a_witness_remainder_serialized_verbatim_is_found_by_the_scan() {
    let prepared = Gkr.prepare(ExampleId::OnePlusOne, &Control::new()).unwrap();
    let secret = Field::from_u64(424_242).to_le_bytes();
    let serialized =
        bincode::serialize(&vec![Fr::from(7), Fr::from(424_242), Fr::from(9)]).unwrap();
    let scan = scan_secrets(prepared.as_ref(), ExampleId::OnePlusOne, &[secret], &serialized);
    assert!(matches!(scan, SecretScan::Found(_)), "{scan:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer_and_encodes_little_endian() {
    zk_circuit::check_field::<Bn254Scalar>().unwrap();
    let mut expected = vec![0u8; 32];
    expected[..2].copy_from_slice(&[2, 1]);
    assert_eq!(Field::from_u64(258).to_le_bytes(), expected);
}

/// The Toy Shielded Pool proves its own notes this way: the adapter proves exactly the values it is
/// handed, and a false assignment still becomes a proof for the verifier to turn down.
#[test]
fn a_caller_supplied_assignment_is_proved_as_given_and_never_pre_checked() {
    let control = Control::new();
    let mut prepared = Gkr.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
