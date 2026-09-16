//! What the adapter itself promises around lambdaworks: live blinding, caller-supplied assignments,
//! and undecodable bytes reported as malformed rather than as errors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_plonk::{Field, Plonk};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};

fn instance(kind: InstanceKind) -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind, seed: [3; 32] }
}

/// With the prover's blinding switched off, two proofs of one claim would be byte-identical.
#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
fn a_caller_supplied_input_at_or_above_the_modulus_is_refused() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let sample = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let outcome = prepared.prove_assignment(&[vec![0xff; 32]], &sample.secrets, &control);
    assert!(matches!(outcome, Err(SystemError::Failed(_))), "{outcome:?}");
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

/// The BLS12-381 scalar modulus `r`, big-endian.
const MODULUS_BE: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

/// `value + r` when it still fits in 32 bytes.
fn plus_modulus(value: &[u8]) -> Option<Vec<u8>> {
    let mut sum = vec![0u8; 32];
    let mut carry = 0u16;
    for index in (0..32).rev() {
        let total = u16::from(value[index]) + u16::from(MODULUS_BE[index]) + carry;
        sum[index] = total as u8;
        carry = total >> 8;
    }
    (carry == 0).then_some(sum)
}

/// lambdaworks reduces a scalar written as `s + r` to `s`, so those bytes would verify as the
/// same proof; the adapter accepts only the canonical encoding. About half of all scalars leave
/// room for `+ r` in 32 bytes, so a few proofs always offer one of the eight.
#[test]
fn a_proof_scalar_written_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let (public, forged) = (0..16)
        .find_map(|_| {
            let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
            (0..8).find_map(|element| {
                let start = 4 + element * 36;
                let raised = plus_modulus(&proven.proof[start..start + 32])?;
                let mut forged = proven.proof.clone();
                forged[start..start + 32].copy_from_slice(&raised);
                Some((proven.public.clone(), forged))
            })
        })
        .unwrap();
    let verdict = prepared.verify(&public, &forged, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// lambdaworks' decoder stops after the last opening proof; the adapter refuses what follows it.
#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn a_proof_cut_short_is_malformed() {
    let control = Control::new();
    let mut prepared = Plonk.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.proof.truncate(proven.proof.len() - 1);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
