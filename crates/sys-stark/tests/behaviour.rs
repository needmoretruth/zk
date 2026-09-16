//! What the adapter itself promises around lambdaworks' STARK: caller-supplied assignments, Stark252
//! arithmetic on public inputs, undecodable bytes reported as malformed, and a flipped byte that
//! lands on a wire value.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use lambdaworks_math::field::element::FieldElement;
use lambdaworks_math::traits::ByteConversion;
use sys_stark::{Field, Stark, Stark252};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

/// Stark252's modulus `2^251 + 17·2^192 + 1` as 32 little-endian bytes.
fn modulus() -> Vec<u8> {
    let mut bytes = vec![0u8; 32];
    bytes[0] = 0x01;
    bytes[24] = 0x11;
    bytes[31] = 0x08;
    bytes
}

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn an_assignment_given_as_bytes_is_proved_and_verifies() {
    let control = Control::new();
    let mut prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&honest(), &control).unwrap();
    let mut private = sample.secrets.clone();
    private[0] = modulus();
    let refused = prepared.prove_assignment(&sample.public, &private, &control);
    assert!(matches!(refused, Err(SystemError::Failed(_))), "{refused:?}");
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict =
        prepared.verify(std::slice::from_ref(&modulus()), &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&modulus()).is_err());
}

#[test]
fn bumping_the_largest_element_wraps_to_zero() {
    let control = Control::new();
    let prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut largest = modulus();
    largest[0] = 0x00;
    assert_eq!(prepared.bump_public(&largest).unwrap(), Field::zero().to_le_bytes());
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// A proof of one statement shown to the verifier of another with the same public-input count.
#[test]
fn a_proof_for_a_trace_of_another_width_is_malformed() {
    let control = Control::new();
    let mut one_plus_one = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut password = Stark.prepare(ExampleId::Password, &control).unwrap();
    let proven = one_plus_one.prove(&honest(), &control).unwrap();
    let verdict = password.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// The flip attack's byte ends the out-of-domain row, and that row is the wires verbatim in
/// Montgomery form (`value · 2^256 mod p`, big-endian): the envelope (column 0), the answer, then
/// the salt, each behind bincode's one-byte length of 32.
#[test]
fn the_flipped_byte_ends_the_envelope_in_the_out_of_domain_row_in_montgomery_form() {
    let control = Control::new();
    let mut prepared = Stark.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let montgomery = |canonical: &[u8]| {
        let value = Stark252(FieldElement::from_bytes_le(canonical).unwrap());
        let r = (0..256).fold(Field::one(), |acc, _| acc.add(acc));
        let mut bytes = value.mul(r).to_le_bytes();
        bytes.reverse();
        bytes
    };
    let start = prepared.tamper_offset(&proven.proof) - 31;
    let element = |column: usize| &proven.proof[start + 33 * column..start + 33 * column + 32];
    assert_eq!(element(0), montgomery(&proven.public[0]));
    assert_eq!(element(1), montgomery(&Field::from_u64(2).to_le_bytes()));
    assert_eq!(element(2), montgomery(&proven.secrets[1]));
    assert_eq!(proven.proof[start + 32], 32);
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
