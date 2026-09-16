//! Run instances: fresh salts from the seed, the same truth or falsehood as the samples.

use zk_circuit::ZkField;
use zk_examples::{ExampleId, InstanceKind};

use crate::fields::{BabyBear, Bn254};

fn check<F: ZkField>() {
    for example in ExampleId::ALL {
        let circuit = example.circuit::<F>().unwrap();
        let honest = example.instance::<F>(InstanceKind::Honest, &[3; 32]);
        assert!(circuit.evaluate(&honest).is_ok(), "{} honest instance", example.id());
        let dishonest = example.instance::<F>(InstanceKind::Dishonest, &[3; 32]);
        assert!(circuit.evaluate(&dishonest).is_err(), "{} dishonest instance", example.id());
    }
}

#[test]
fn instances_keep_the_claims_true_or_false_on_both_fields() {
    check::<Bn254>();
    check::<BabyBear>();
}

#[test]
fn two_seeds_seal_the_same_answer_in_different_envelopes() {
    let one = ExampleId::OnePlusOne.instance::<Bn254>(InstanceKind::Honest, &[1; 32]);
    let two = ExampleId::OnePlusOne.instance::<Bn254>(InstanceKind::Honest, &[2; 32]);
    assert_eq!(one.private[0], two.private[0], "the answer is 2 both times");
    assert_ne!(one.public[0], two.public[0], "the envelopes differ");
    let age_one = ExampleId::Age.instance::<Bn254>(InstanceKind::Honest, &[1; 32]);
    let age_two = ExampleId::Age.instance::<Bn254>(InstanceKind::Honest, &[2; 32]);
    assert_ne!(age_one.public[2], age_two.public[2], "the credentials differ");
}

fn moving_the_falsifying_input_breaks_the_claim<F: ZkField>() {
    for example in ExampleId::ALL {
        let circuit = example.circuit::<F>().unwrap();
        let mut claim = example.instance::<F>(InstanceKind::Honest, &[4; 32]);
        let index = example.falsifying_public_index();
        claim.public[index] = claim.public[index].add(F::one());
        assert!(circuit.evaluate(&claim).is_err(), "{} still holds", example.id());
    }
}

#[test]
fn the_attacked_public_input_turns_every_claim_false_on_both_fields() {
    moving_the_falsifying_input_breaks_the_claim::<Bn254>();
    moving_the_falsifying_input_breaks_the_claim::<BabyBear>();
}
