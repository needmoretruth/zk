use zk_circuit::toyhash::{toyhash, toyhash_gadget};
use zk_circuit::{Assignment, CircuitBuilder, ZkField};

use crate::fields::{BabyBear, Bn254};

fn native_matches_gadget<F: ZkField>() {
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let a = builder.private_input("a");
    let b = builder.private_input("b");
    let output = toyhash_gadget(&mut builder, a, b);
    let circuit = builder.finish().unwrap();
    let samples = [(0, 0), (1, 2), (2, 1), (u64::MAX, 7), (1 << 40, 123_456_789)];
    for (a, b) in samples {
        let (a, b) = (F::from_u64(a), F::from_u64(b));
        let assignment = Assignment { public: vec![], private: vec![a, b] };
        let wires = circuit.evaluate(&assignment).unwrap();
        assert_eq!(wires.get(output), toyhash(a, b));
    }
}

fn hex<F: ZkField>(value: F) -> String {
    value.to_le_bytes().iter().rev().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn gadget_agrees_with_native_toyhash_on_bn254() {
    native_matches_gadget::<Bn254>();
}

#[test]
fn gadget_agrees_with_native_toyhash_on_babybear() {
    native_matches_gadget::<BabyBear>();
}

// FROZEN. If this constant changes, ToyHash changed, and every public value of every example
// changed with it.
#[test]
fn toyhash_known_answer_on_bn254() {
    let digest = toyhash(Bn254::from_u64(1), Bn254::from_u64(2));
    assert_eq!(hex(digest), "19dcd0ce67444eb8dedb5904ae1a3554a3c1f73af1a59584a62bef7ae5541501");
}

// FROZEN. If this constant changes, ToyHash changed, and every public value of every example
// changed with it.
#[test]
fn toyhash_known_answer_on_babybear() {
    let digest = toyhash(BabyBear::from_u64(1), BabyBear::from_u64(2));
    assert_eq!(hex(digest), "3599c80f");
}
