//! What the adapter itself promises around jellyfish: live blinding, caller-supplied assignments, range
//! checks proved by lookups with the same boundary, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_plonk::proof_system::structs::Proof;
use sys_ultraplonk::{Field, JELLYFISH_REV, META, TAMPER_OFFSET, UltraPlonk};
use zk_circuit::ZkField;
use zk_circuit::toyhash::toyhash;
use zk_core::catalog::Implementation;
use zk_core::{
    Control, ExampleId, Instance, InstanceKind, Prepared, ProofSystem, SystemError, Verdict,
};

fn instance(kind: InstanceKind) -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind, seed: [3; 32] }
}

fn prepared(example: ExampleId) -> Box<dyn Prepared> {
    UltraPlonk.prepare(example, &Control::new()).unwrap()
}

fn count(prepared: &dyn Prepared, name: &str) -> u64 {
    prepared.shape().counts.iter().find(|(key, _)| key == name).unwrap().1
}

/// With the prover's blinding switched off, two proofs of one claim would be byte-identical.
#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
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

/// The dishonest assignment reaches jellyfish's prover, which refuses in round 3.
#[test]
fn a_caller_supplied_assignment_is_proved_without_being_checked() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
    let honest = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let proven = prepared.prove_assignment(&honest.public, &honest.secrets, &control).unwrap();
    assert_eq!((&proven.public, &proven.secrets), (&honest.public, &honest.secrets));
    assert_eq!(
        prepared.verify(&proven.public, &proven.proof, &control).unwrap(),
        Verdict::Accepted
    );
    let wrong = [Field::from_u64(3).to_le_bytes(), honest.secrets[1].clone()];
    let refused = prepared.prove_assignment(&honest.public, &wrong, &control);
    assert!(
        matches!(&refused, Err(SystemError::Unsatisfied(why)) if why.contains("quotient")),
        "{refused:?}"
    );
}

/// `pin < 2^20` is proved with two 8-bit table lookups plus a 4-bit decomposition; the largest PIN
/// in range must still prove and the first one out of range must not.
#[test]
fn a_range_checked_by_lookups_accepts_exactly_the_values_below_its_bound() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::Password);
    assert_eq!(count(prepared.as_ref(), "range-checks"), 1);
    assert_eq!(count(prepared.as_ref(), "range-lookups"), 2);
    for (pin, in_range) in [((1u64 << 20) - 1, true), (1u64 << 20, false)] {
        let pin = Field::from_u64(pin);
        let public = [toyhash(pin, Field::zero()).to_le_bytes()];
        let outcome = prepared.prove_assignment(&public, &[pin.to_le_bytes()], &control);
        let accepted = match outcome {
            Ok(proven) => {
                prepared.verify(&proven.public, &proven.proof, &control).unwrap().accepted()
            }
            Err(SystemError::Unsatisfied(_)) => false,
            Err(error) => panic!("{error}"),
        };
        assert_eq!(accepted, in_range, "{pin:?}");
    }
}

#[test]
fn a_statement_without_range_checks_uses_no_lookups() {
    let prepared = prepared(ExampleId::OnePlusOne);
    assert_eq!(count(prepared.as_ref(), "range-checks"), 0);
    assert_eq!(count(prepared.as_ref(), "range-lookups"), 0);
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
    let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
    let outcome = prepared.prove_assignment(&[vec![0xff; 32]], &proven.secrets, &control);
    assert!(matches!(outcome, Err(SystemError::Failed(_))), "{outcome:?}");
}

/// arkworks' decoder stops after the last field; the adapter refuses what follows it.
#[test]
fn bytes_after_the_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.proof.push(0);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn a_proof_cut_short_is_malformed() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
    let mut proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    proven.proof.truncate(proven.proof.len() - 1);
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

/// The flip attack must hit a value the verifier reads, not a length prefix or padding.
#[test]
fn the_tamper_offset_is_the_first_byte_of_the_first_wire_evaluation() {
    let control = Control::new();
    let mut prepared = prepared(ExampleId::OnePlusOne);
    let proven = prepared.prove(&instance(InstanceKind::Honest), &control).unwrap();
    let proof = Proof::<ark_bn254::Bn254>::deserialize_compressed(proven.proof.as_slice()).unwrap();
    let mut first_eval = Vec::new();
    proof.poly_evals.wires_evals[0].serialize_compressed(&mut first_eval).unwrap();
    assert_eq!(&proven.proof[TAMPER_OFFSET..TAMPER_OFFSET + 32], first_eval.as_slice());
}

/// Where the proof's lookup evaluations that are identically zero start: the key table, table
/// domain separation, domain separation selector and lookup selector at `ζ`, then the key table,
/// table domain separation and lookup selector at `ζ·g`. These circuits look nothing up in a
/// key-value table, so those polynomials are zero, yet jellyfish sends their evaluations.
const ZERO_EVALUATIONS: [usize; 7] = [1033, 1065, 1097, 1161, 1257, 1289, 1385];

/// The secret scan in `examples.rs` tolerates one- and two-byte secrets because the 32-byte encoding of
/// a small value is mostly zeros. This pins the only place a proof supplies such zeros: once the seven
/// zero evaluations are masked out, no run of 31 zero bytes is left.
#[test]
fn the_only_long_runs_of_zero_bytes_in_a_proof_are_the_zero_lookup_evaluations() {
    let control = Control::new();
    for example in [ExampleId::OnePlusOne, ExampleId::Password] {
        let mut prepared = prepared(example);
        let instance = Instance { example, kind: InstanceKind::Honest, seed: [3; 32] };
        let mut proof = prepared.prove(&instance, &control).unwrap().proof;
        for start in ZERO_EVALUATIONS {
            assert!(proof[start..start + 32].iter().all(|byte| *byte == 0), "{example:?} {start}");
            proof[start..start + 32].fill(0xff);
        }
        let longest = proof.split(|byte| *byte != 0).map(<[u8]>::len).max().unwrap_or(0);
        assert!(longest < 31, "{example:?}: {longest} zero bytes in a row");
    }
}

#[test]
fn the_catalogue_names_the_commit_the_manifest_pins() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains(&format!("rev = \"{JELLYFISH_REV}\"")));
    let Implementation::Upstream { version, .. } = META.implementation else { panic!() };
    assert!(JELLYFISH_REV.starts_with(version));
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<Field>().unwrap();
}
