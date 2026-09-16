//! What the adapter itself promises around halo2_proofs: fresh prover randomness, and undecodable
//! bytes reported as malformed rather than as errors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_halo2::{Halo2, PastaFp};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, Verdict};

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Halo2.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Halo2.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

/// 64 bytes hold two of the three advice commitments, so reading the third runs out of bytes.
#[test]
fn a_transcript_cut_before_the_opening_argument_is_malformed() {
    let control = Control::new();
    let mut prepared = Halo2.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.truncate(64);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<PastaFp>().unwrap();
}
