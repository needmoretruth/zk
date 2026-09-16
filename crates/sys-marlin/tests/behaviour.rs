//! What the adapter itself promises around ark-marlin: fresh prover randomness, undecodable bytes
//! reported as malformed rather than as errors, and caller-supplied assignments never pre-checked.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_marlin::{Field, Fr, Marlin, TAMPER_OFFSET};
use zk_circuit::ZkField;
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, SystemError, Verdict};
use zk_examples::one_plus_one::sealed;

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [3; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Marlin.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Marlin.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Marlin.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// ark-marlin's verifier indexes the proof's three rounds without counting them; a proof that
/// decodes with a round missing must come back malformed, not take the caller down with a panic.
#[test]
fn a_proof_that_decodes_without_marlins_three_rounds_is_malformed() {
    let control = Control::new();
    let mut prepared = Marlin.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    // An empty commitments vector (an 8-byte zero length) followed by the proof's own evaluations,
    // messages and opening proof, which start 8 bytes before the tamper offset.
    let evaluations_start = TAMPER_OFFSET - 8;
    let mut roundless = vec![0u8; 8];
    roundless.extend_from_slice(&proven.proof[evaluations_start..]);
    let verdict = prepared.verify(&proven.public, &roundless, &control).unwrap();
    assert!(matches!(&verdict, Verdict::Malformed(why) if why.contains("panicked")), "{verdict:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Fr>().unwrap();
}

/// The Toy Shielded Pool proves its own notes this way: the adapter proves exactly the values it is
/// handed, and a false assignment still reaches ark-marlin. Its prover's only reaction is a
/// `debug_assert!` on the outer sum-check, so with debug assertions on arkworks itself refuses, and
/// without them the verifier turns the proof down.
#[test]
fn a_caller_supplied_assignment_is_proved_as_given_and_never_pre_checked() {
    let control = Control::new();
    let mut prepared = Marlin.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let forged = prepared.prove_assignment(
        &encode(&false_claim.public),
        &encode(&false_claim.private),
        &control,
    );
    match forged {
        Err(SystemError::Unsatisfied(why)) if cfg!(debug_assertions) => {
            assert!(why.contains("outer_sumcheck"), "{why}");
        }
        Ok(forged) if !cfg!(debug_assertions) => assert_eq!(
            prepared.verify(&forged.public, &forged.proof, &control).unwrap(),
            Verdict::Rejected
        ),
        other => panic!("unexpected result for a false claim: {other:?}"),
    }
}
