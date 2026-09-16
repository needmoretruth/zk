use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, EvalError, LinearCombination, Visibility, ZkField,
};

use crate::fields::BabyBear;

type F = BabyBear;

/// Private `x` with the assertions `x = 1`, `x = 2`, `x = 3`, in that order.
fn three_claims() -> Circuit<F> {
    let mut builder = CircuitBuilder::new().unwrap();
    let x = builder.private_input("x");
    for (value, label) in [(1, "x is one"), (2, "x is two"), (3, "x is three")] {
        builder
            .assert_zero(LinearCombination::from(x).add_constant(F::from_u64(value).neg()), label);
    }
    builder.finish().unwrap()
}

fn private(values: &[u64]) -> Assignment<F> {
    Assignment { public: vec![], private: values.iter().map(|v| F::from_u64(*v)).collect() }
}

#[test]
fn checked_evaluation_names_the_first_violated_assertion() {
    assert_eq!(
        three_claims().evaluate(&private(&[3])),
        Err(EvalError::AssertionFailed { label: "x is one".to_string() })
    );
}

#[test]
fn unchecked_evaluation_lists_every_violation_and_keeps_the_wires() {
    let evaluation = three_claims().evaluate_unchecked(&private(&[3])).unwrap();
    assert_eq!(evaluation.violated, ["x is one", "x is two"]);
    assert_eq!(evaluation.values.as_slice(), [F::from_u64(3)]);
}

#[test]
fn a_wrong_number_of_inputs_is_rejected() {
    assert_eq!(
        three_claims().evaluate_unchecked(&private(&[1, 2])),
        Err(EvalError::InputCount { visibility: Visibility::Private, expected: 1, got: 2 })
    );
}

#[test]
fn interleaved_inputs_bind_in_declaration_order_per_visibility() {
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let p0 = builder.private_input("p0");
    let q0 = builder.public_input("q0");
    let p1 = builder.private_input("p1");
    let q1 = builder.public_input("q1");
    let circuit = builder.finish().unwrap();
    let values = |xs: [u64; 2]| xs.map(F::from_u64).to_vec();
    let assignment = Assignment { public: values([10, 11]), private: values([20, 21]) };
    let wires = circuit.evaluate(&assignment).unwrap();
    let read = [p0, q0, p1, q1].map(|wire| wires.get(wire));
    assert_eq!(read, [20, 10, 21, 11].map(F::from_u64));
}
