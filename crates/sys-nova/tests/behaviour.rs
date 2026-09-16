//! What the adapter itself promises around nova-snark: fresh prover randomness, the flip landing where
//! its doc comment says, the encodings it declares, and undecodable bytes reported as malformed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bincode::config::legacy;
use nova_snark::nova::nifs::{NIFS, NIFSRelaxed};
use nova_snark::provider::pasta::{pallas, vesta};
use nova_snark::provider::{PallasEngine, VestaEngine};
use nova_snark::r1cs::{R1CSInstance, RelaxedR1CSInstance};
use sys_nova::{Field, Nova, PallasScalar, TAMPER_OFFSET};
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
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let verdict = prepared.verify(&[vec![0xff; 32]], &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&vec![0xff; 32]).is_err());
}

/// Nova would refuse a `z0` of the wrong arity with an error; the adapter answers before decoding.
#[test]
fn a_wrong_number_of_public_inputs_is_malformed() {
    let control = Control::new();
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let mut proven = prepared.prove(&honest(), &control).unwrap();
    proven.public.push(proven.public[0].clone());
    let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
}

#[test]
fn bytes_after_the_proof_and_a_cut_proof_are_malformed() {
    let control = Control::new();
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let mut longer = proven.proof.clone();
    longer.push(0);
    let mut shorter = proven.proof.clone();
    shorter.truncate(TAMPER_OFFSET);
    for bytes in [longer, shorter] {
        let verdict = prepared.verify(&proven.public, &bytes, &control).unwrap();
        assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    }
}

/// Everything `CompressedSNARK` writes before `snark_primary`, in field order, as upstream types.
type BeforePrimarySnark = (
    RelaxedR1CSInstance<VestaEngine>,
    vesta::Scalar,
    R1CSInstance<VestaEngine>,
    NIFS<VestaEngine>,
    RelaxedR1CSInstance<VestaEngine>,
    NIFSRelaxed<VestaEngine>,
    RelaxedR1CSInstance<PallasEngine>,
    pallas::Scalar,
    RelaxedR1CSInstance<PallasEngine>,
    NIFSRelaxed<PallasEngine>,
    [pallas::Scalar; 2],
    [vesta::Scalar; 2],
);

/// The doc comment on the offset promises the constant term of the outer sum-check's first round,
/// and a flip there that still decodes and is rejected by the verifier itself.
#[test]
fn the_tampered_byte_opens_the_first_round_of_the_primary_outer_sum_check() {
    let control = Control::new();
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    assert_eq!(prepared.tamper_offset(&proven.proof), TAMPER_OFFSET);
    let (_, prefix) =
        bincode::serde::decode_from_slice::<BeforePrimarySnark, _>(&proven.proof, legacy())
            .unwrap();
    let length = |at: usize| u64::from_le_bytes(proven.proof[at..at + 8].try_into().unwrap());
    let augmented =
        prepared.shape().counts.iter().find(|(n, _)| n == "augmented-constraints").unwrap().1;
    assert_eq!(prefix + 16, TAMPER_OFFSET);
    assert!(augmented <= 1 << length(prefix) && 1 << length(prefix) < 4 * augmented);
    assert_eq!(length(prefix + 8), 3, "a cubic round polynomial without its linear term");
    let mut flipped = proven.proof.clone();
    flipped[TAMPER_OFFSET] ^= 0x01;
    assert_eq!(prepared.verify(&proven.public, &flipped, &control).unwrap(), Verdict::Rejected);
}

/// halo2curves' serde writes a scalar as its canonical little-endian bytes, the pattern the adapter
/// leaves to the default encodings: a witness serialized verbatim would be caught.
#[test]
fn a_scalar_serialized_the_way_nova_writes_proofs_is_found_by_the_scan() {
    let prepared = Nova.prepare(ExampleId::OnePlusOne, &Control::new()).unwrap();
    let secret = PallasScalar::from_u64(424_242);
    let serialized = bincode::serde::encode_to_vec(vec![secret.0], legacy()).unwrap();
    let encoded = secret.to_le_bytes();
    let scan = scan_secrets(prepared.as_ref(), ExampleId::OnePlusOne, &[encoded], &serialized);
    assert!(matches!(scan, SecretScan::Found(_)), "{scan:?}");
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    zk_circuit::check_field::<PallasScalar>().unwrap();
}

/// The Toy Shielded Pool proves its own notes this way: the adapter proves exactly the values it is
/// handed, and a false assignment still becomes a proof for the verifier to turn down.
#[test]
fn a_caller_supplied_assignment_is_proved_as_given_and_never_pre_checked() {
    let control = Control::new();
    let mut prepared = Nova.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
