//! What the verifier promises about swapped columns, broken constraints and malformed proofs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_ligero::{Checks, Fp, Ligero, MODULUS, Proof, Statement, check, prove};
use zk_circuit::{Assignment, CircuitBuilder, LinearCombination, ZkField, check_field};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, Verdict};

const ALL_PASS: Checks = Checks { merkle: true, proximity: true, linear: true, quadratic: true };

fn honest_age() -> (Statement, Vec<Fp>, Vec<u8>) {
    let example = ExampleId::Age;
    let statement = Statement::new(example).unwrap();
    let claim = statement.claim(&example.instance(InstanceKind::Honest, &[16; 32])).unwrap();
    let bytes = prove(&statement, &claim, &Control::new()).unwrap();
    (statement, claim.public, bytes)
}

#[test]
fn a_proof_whose_opened_column_is_swapped_for_another_fails_the_merkle_check() {
    let (statement, public, bytes) = honest_age();
    assert_eq!(check(&statement, &public, &bytes).unwrap(), ALL_PASS);
    let params = statement.params();
    let mut proof = Proof::decode(&params, &bytes).unwrap();
    proof.columns.swap(0, 1);
    let checks = check(&statement, &public, &proof.encode(&params)).unwrap();
    assert!(!checks.merkle, "{checks:?}");
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
fn a_prover_that_breaks_one_quadratic_constraint_is_rejected_by_the_quadratic_test() {
    let control = Control::new();
    let statement = Statement::from_circuit("product", product_circuit("x times y")).unwrap();
    let [x, y] = [5, 7].map(Fp::from_u64);
    let honest = Assignment { public: vec![Fp::from_u64(35)], private: vec![x, y] };
    let claim = statement.claim(&honest).unwrap();
    let bytes = prove(&statement, &claim, &control).unwrap();
    assert_eq!(check(&statement, &claim.public, &bytes).unwrap(), ALL_PASS);
    // Claim 36 and write 36 into the multiplication's output (w[2], after x and y) and its copy z[0]:
    // every linear constraint holds, and only 5 · 7 = 36 is false.
    let mut cheat =
        statement.claim(&Assignment { public: vec![Fp::from_u64(36)], ..honest }).unwrap();
    cheat.witness.w[2] = Fp::from_u64(36);
    cheat.witness.z[0] = Fp::from_u64(36);
    let bytes = prove(&statement, &cheat, &control).unwrap();
    let checks = check(&statement, &cheat.public, &bytes).unwrap();
    assert_eq!(checks, Checks { quadratic: false, ..ALL_PASS });
}

#[test]
fn a_proof_is_bound_to_the_example_id_and_to_the_gates() {
    let control = Control::new();
    let statement = Statement::from_circuit("product", product_circuit("x times y")).unwrap();
    let assignment = Assignment {
        public: vec![Fp::from_u64(35)],
        private: vec![Fp::from_u64(5), Fp::from_u64(7)],
    };
    let claim = statement.claim(&assignment).unwrap();
    let bytes = prove(&statement, &claim, &control).unwrap();
    let accepted = |statement: &Statement| check(statement, &claim.public, &bytes).unwrap().all();
    assert!(accepted(&statement));
    assert!(!accepted(&Statement::from_circuit("other", product_circuit("x times y")).unwrap()));
    assert!(!accepted(&Statement::from_circuit("product", product_circuit("y times x")).unwrap()));
}

fn one_plus_one(seed: u8) -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [seed; 32] }
}

#[test]
fn the_same_claim_proved_twice_gives_two_different_proofs_that_both_verify() {
    let control = Control::new();
    let mut prepared = Ligero.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let first = prepared.prove(&one_plus_one(18), &control).unwrap();
    let second = prepared.prove(&one_plus_one(18), &control).unwrap();
    assert_ne!(first.proof, second.proof);
    for proven in [first, second] {
        let verdict = prepared.verify(&proven.public, &proven.proof, &control).unwrap();
        assert_eq!(verdict, Verdict::Accepted);
    }
}

#[test]
fn a_wrong_header_a_cut_proof_or_a_partial_hash_is_malformed() {
    let control = Control::new();
    let mut prepared = Ligero.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&one_plus_one(19), &control).unwrap();
    let mut header = proven.proof.clone();
    header[0] ^= 1;
    let mut partial = proven.proof.clone();
    partial.push(0);
    let cut = proven.proof[..100].to_vec();
    for bytes in [header, partial, cut, Vec::new()] {
        let verdict = prepared.verify(&proven.public, &bytes, &control).unwrap();
        assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    }
}

#[test]
fn a_public_input_at_or_above_the_modulus_is_malformed() {
    let control = Control::new();
    let mut prepared = Ligero.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&one_plus_one(20), &control).unwrap();
    let too_big = MODULUS.to_le_bytes().to_vec();
    let verdict =
        prepared.verify(core::slice::from_ref(&too_big), &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&too_big).is_err());
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
    let mut prepared = Ligero.prepare(ExampleId::OnePlusOne, &control).unwrap();
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
