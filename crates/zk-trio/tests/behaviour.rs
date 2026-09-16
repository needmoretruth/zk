//! What the verifier promises about tampered rounds and malformed proofs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_circuit::{Assignment, CircuitBuilder, LinearCombination, ZkField, check_field};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, Verdict};
use zk_trio::{
    Card, CheckKind, Coins, Fp, Friend, HonestProver, MODULUS, Opening, RoundProver, Statement,
    Trio, check_round, proof,
};

fn honest_round(card: Card) -> (Statement, Vec<Fp>, zk_trio::Commitments, Opening) {
    let example = ExampleId::Age;
    let statement = Statement::new(example).unwrap();
    let claim = statement.claim(&example.instance(InstanceKind::Honest, &[16; 32])).unwrap();
    let public = claim.public.clone();
    let mut prover = HonestProver::new(&statement, claim, Coins::seeded([17; 32])).unwrap();
    let commitments = prover.commit().unwrap();
    let opening = prover.respond(card).unwrap();
    (statement, public, commitments, opening)
}

fn failed(checks: &[zk_trio::Check]) -> Vec<CheckKind> {
    checks.iter().filter(|check| !check.passed).map(|check| check.kind).collect()
}

#[test]
fn a_tampered_opened_view_fails_its_commitment_check() {
    let card = Card::Peek(Friend::P3);
    let (statement, public, commitments, mut opening) = honest_round(card);
    assert!(failed(&check_round(&statement, &public, &commitments, card, &opening)).is_empty());
    let Opening::Peek { opened, .. } = &mut opening else { panic!("a peek card opens friends") };
    opened[0].input_seed[5] ^= 1;
    let checks = check_round(&statement, &public, &commitments, card, &opening);
    assert!(failed(&checks).contains(&CheckKind::ViewCommitment(Friend::P1)), "{checks:?}");
}

#[test]
fn a_tampered_hidden_message_fails_its_commitment_check() {
    let card = Card::Peek(Friend::P2);
    let (statement, public, commitments, mut opening) = honest_round(card);
    let Opening::Peek { hidden_broadcasts, .. } = &mut opening else { panic!("a peek card") };
    hidden_broadcasts[0] = hidden_broadcasts[0].add(Fp::one());
    let checks = check_round(&statement, &public, &commitments, card, &opening);
    assert!(failed(&checks).contains(&CheckKind::ViewCommitment(Friend::P2)), "{checks:?}");
}

#[test]
fn an_opening_for_another_card_does_not_fit() {
    let (statement, public, commitments, opening) = honest_round(Card::DealerA);
    let checks = check_round(&statement, &public, &commitments, Card::Peek(Friend::P1), &opening);
    assert_eq!(failed(&checks), [CheckKind::OpeningFitsCard]);
}

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [18; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Trio.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&honest(), &control).unwrap();
    let second = prepared.prove(&honest(), &control).unwrap();
    assert_ne!(first.proof, second.proof);
    for proven in [first, second] {
        let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
        assert_eq!(verdict, Verdict::Accepted);
    }
}

#[test]
fn bytes_after_the_proof_or_a_wrong_round_count_are_malformed() {
    let control = Control::new();
    let mut prepared = Trio.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let mut longer = proven.proof.clone();
    longer.push(0);
    let mut fewer = proven.proof.clone();
    fewer[0] = 108;
    for bytes in [longer, fewer, Vec::new()] {
        let verdict = prepared.verify(&proven.public, &bytes, &control).unwrap();
        assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    }
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Trio.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let too_big = MODULUS.to_le_bytes().to_vec();
    let verdict =
        prepared.verify(core::slice::from_ref(&too_big), &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&too_big).is_err());
}

/// `x · y = product`, with a label to vary: the label is part of the circuit digest.
fn product_circuit(label: &str) -> zk_circuit::Circuit<Fp> {
    let mut builder = CircuitBuilder::<Fp>::new().unwrap();
    let product = builder.public_input("product");
    let x = builder.private_input("x");
    let y = builder.private_input("y");
    let z = builder.mul(x, y);
    builder.assert_zero(LinearCombination::from(z) - product, label);
    builder.finish().unwrap()
}

#[test]
fn a_proof_is_bound_to_the_example_id_and_to_the_gates() {
    let control = Control::new();
    let statement = Statement::from_circuit("product", product_circuit("x times y"));
    let [product, x, y] = [35, 5, 7].map(Fp::from_u64);
    let assignment = Assignment { public: vec![product], private: vec![x, y] };
    let claim = statement.claim(&assignment).unwrap();
    let bytes = proof::prove(&statement, &claim, &mut Coins::os(), &control).unwrap();
    let verdict = |statement: &Statement| proof::verify(statement, &[product], &bytes, &control);
    assert_eq!(verdict(&statement).unwrap(), Verdict::Accepted);
    let renamed = Statement::from_circuit("other", product_circuit("x times y"));
    assert_ne!(verdict(&renamed).unwrap(), Verdict::Accepted);
    let relabelled = Statement::from_circuit("product", product_circuit("y times x"));
    assert_ne!(verdict(&relabelled).unwrap(), Verdict::Accepted);
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    check_field::<Fp>().unwrap();
    assert_eq!(Fp::from_u64(MODULUS), Fp::zero());
    assert_eq!(Fp::decode(&Fp::from_u64(7).to_le_bytes()), Ok(Fp::from_u64(7)));
}

#[test]
fn a_caller_supplied_assignment_is_proved_without_being_checked() {
    let control = Control::new();
    let mut prepared = Trio.prepare(ExampleId::OnePlusOne, &control).unwrap();
    for (kind, expected) in
        [(InstanceKind::Honest, Verdict::Accepted), (InstanceKind::Dishonest, Verdict::Rejected)]
    {
        let instance = Instance { example: ExampleId::OnePlusOne, kind, seed: [21; 32] };
        let sample = prepared.prove(&instance, &control).unwrap();
        let proven = prepared.prove_assignment(&sample.public, &sample.secrets, &control).unwrap();
        assert_eq!(proven.secrets, sample.secrets);
        assert_eq!(prepared.verify(&proven.public, &proven.proof, &control).unwrap(), expected);
    }
}
