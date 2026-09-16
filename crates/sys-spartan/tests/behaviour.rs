//! What the adapter itself promises around Spartan: fresh prover randomness, the column layout and
//! padding it reports, the encodings it declares, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curve25519_dalek::ristretto::CompressedRistretto;
use libspartan::VarsAssignment;
use sys_spartan::{Field, RistrettoScalar, Spartan, TAMPER_OFFSET};
use zk_circuit::ZkField;
use zk_core::{
    Control, ExampleId, Instance, InstanceKind, ProofSystem, SecretScan, Verdict, scan_secrets,
};
use zk_examples::one_plus_one::sealed;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
fn the_shape_reports_real_sizes_and_the_powers_of_two_spartan_proves() {
    let prepared = Spartan.prepare(ExampleId::Sudoku, &Control::new()).unwrap();
    let counts = prepared.shape().counts;
    let count = |name: &str| counts.iter().find(|(n, _)| n == name).unwrap().1;
    let public = ExampleId::Sudoku.public_input_names().len() as u64;
    let witness = count("variables") - 1 - public;
    assert!(count("padded-constraints").is_power_of_two());
    assert!(count("padded-constraints") >= count("constraints").max(2));
    assert!(count("padded-constraints") < 2 * count("constraints").max(2));
    assert!(count("padded-witness-variables").is_power_of_two());
    assert!(count("padded-witness-variables") >= witness.max(public + 1));
    assert!(count("padded-witness-variables") < 2 * witness.max(public + 1));
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

/// Spartan's verifier asserts the input count and would panic; the adapter answers first.
#[test]
fn a_wrong_number_of_public_inputs_is_malformed() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.public.push(proven.public[0].clone());
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// The doc comment on the offset promises a valid point there that the flip turns into no point.
#[test]
fn the_tampered_byte_opens_a_point_the_flip_makes_undecodable() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    assert_eq!(prepared.tamper_offset(&proven.proof), TAMPER_OFFSET);
    let mut point: [u8; 32] = proven.proof[TAMPER_OFFSET..TAMPER_OFFSET + 32].try_into().unwrap();
    assert!(CompressedRistretto(point).decompress().is_some());
    point[0] ^= 0x01;
    assert!(CompressedRistretto(point).decompress().is_none());
}

/// Spartan's scalars serialize in Montgomery form: a witness it wrote out verbatim is still caught.
#[test]
fn a_witness_spartan_serialized_verbatim_is_found_by_the_scan() {
    let prepared = Spartan.prepare(ExampleId::OnePlusOne, &Control::new()).unwrap();
    let secret = RistrettoScalar::from_u64(424_242).to_le_bytes();
    let witness = VarsAssignment::new(&[secret.clone().try_into().unwrap()]).unwrap();
    let serialized = bincode::serialize(&witness).unwrap();
    assert!(!serialized.windows(32).any(|window| window == secret));
    let scan = scan_secrets(prepared.as_ref(), ExampleId::OnePlusOne, &[secret], &serialized);
    assert!(matches!(scan, SecretScan::Found(_)), "{scan:?}");
}

/// Why the canonical patterns are not searched: a false sudoku claim with no 3 anywhere in its
/// witness still yields a proof holding the canonical big-endian encoding of 3.
#[test]
fn canonical_encodings_of_small_values_sit_in_every_proof_whatever_the_witness() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::Sudoku, &control).unwrap();
    let encode = |values: &[Field]| values.iter().map(|v| v.to_le_bytes()).collect::<Vec<_>>();
    let claim = ExampleId::Sudoku.honest::<Field>();
    let no_threes = vec![Field::from_u64(2); claim.private.len()];
    let proven =
        prepared.prove_assignment(&encode(&claim.public), &encode(&no_threes), &control).unwrap();
    let three = Field::from_u64(3).to_le_bytes();
    let mut three_big_endian = three.clone();
    three_big_endian.reverse();
    assert!(proven.proof.windows(32).any(|window| window == three_big_endian));
    assert!(!prepared.secret_encodings(&three).contains(&three_big_endian));
    assert!(!prepared.secret_encodings(&three).contains(&three));
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<RistrettoScalar>().unwrap();
}

/// The Toy Shielded Pool proves its own notes this way: the adapter proves exactly the values it is
/// handed, and a false assignment still becomes a proof for the verifier to turn down.
#[test]
fn a_caller_supplied_assignment_is_proved_as_given_and_never_pre_checked() {
    let control = Control::new();
    let mut prepared = Spartan.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
