//! What the adapter itself promises around Winterfell: a prover with no randomness, caller-supplied
//! assignments, f128 arithmetic on public inputs, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_winterfell::{Field, Winterfell};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

/// f128's modulus, `2^128 − 45·2^40 + 1`.
const MODULUS: u128 = 340_282_366_920_938_463_463_374_557_953_744_961_537;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

/// Nothing is masked, so nothing is random: the same claim gives the same bytes.
#[test]
fn the_same_claim_proved_twice_gives_the_same_proof() {
    let control = Control::new();
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&honest(), &control).unwrap();
    let second = prepared.prove(&honest(), &control).unwrap();
    assert_eq!(first, second);
    assert_eq!(prepared.verify(&first.public, &first.proof, &control).unwrap(), Verdict::Accepted);
}

#[test]
fn an_assignment_given_as_bytes_is_proved_and_verifies() {
    let control = Control::new();
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&honest(), &control).unwrap();
    let mut private = sample.secrets.clone();
    private[0] = MODULUS.to_le_bytes().to_vec();
    let refused = prepared.prove_assignment(&sample.public, &private, &control);
    assert!(matches!(refused, Err(SystemError::Failed(_))), "{refused:?}");
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let largest = (MODULUS - 1).to_le_bytes().to_vec();
    assert_eq!(prepared.bump_public(&largest).unwrap(), Field::zero().to_le_bytes());
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// A proof of one statement shown to the verifier of another with the same public-input count.
#[test]
fn a_proof_for_a_trace_of_another_width_is_malformed() {
    let control = Control::new();
    let mut one_plus_one = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut password = Winterfell.prepare(ExampleId::Password, &control).unwrap();
    let proven = one_plus_one.prove(&honest(), &control).unwrap();
    let verdict = password.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// The flip attack's byte starts the out-of-domain row, and that row is the wires verbatim: the
/// envelope (column 0), the answer, then the salt, each as 16 little-endian bytes.
#[test]
fn the_flipped_byte_starts_the_out_of_domain_row_which_is_the_wires_verbatim() {
    let control = Control::new();
    let mut prepared = Winterfell.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let offset = prepared.tamper_offset(&proven.proof);
    let row = &proven.proof[offset..offset + 48];
    assert_eq!(row[..16], proven.public[0][..]);
    assert_eq!(row[16..32], proven.secrets[0][..]);
    assert_eq!(row[32..], proven.secrets[1][..]);
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
