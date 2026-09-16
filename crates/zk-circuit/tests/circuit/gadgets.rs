use zk_circuit::gadgets::{
    assert_boolean, assert_nonzero, merkle_root, merkle_root_native, range_check, select,
};
use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, CircuitError, EvalError, LinearCombination, Wire, ZkField,
};

use crate::fields::{BabyBear, Bn254};

type F = BabyBear;

/// A circuit with one private input `v` passed through `gadget`.
fn with_one_input(gadget: impl FnOnce(&mut CircuitBuilder<F>, Wire)) -> Circuit<F> {
    let mut builder = CircuitBuilder::new().unwrap();
    let v = builder.private_input("v");
    gadget(&mut builder, v);
    builder.finish().unwrap()
}

fn run(circuit: &Circuit<F>, value: F) -> Result<(), EvalError> {
    circuit.evaluate(&Assignment { public: vec![], private: vec![value] }).map(|_| ())
}

fn failed(label: &str) -> Result<(), EvalError> {
    Err(EvalError::AssertionFailed { label: label.to_string() })
}

#[test]
fn range_check_accepts_the_largest_value_and_rejects_the_next_and_negatives() {
    let circuit = with_one_input(|b, v| {
        range_check(b, v, 8, "v fits in 8 bits").unwrap();
    });
    assert_eq!(run(&circuit, F::from_u64(255)), Ok(()));
    assert_eq!(run(&circuit, F::from_u64(256)), failed("v fits in 8 bits"));
    assert_eq!(run(&circuit, F::one().neg()), failed("v fits in 8 bits"));
}

#[test]
fn range_check_refuses_a_width_that_could_wrap() {
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let v = builder.private_input("v");
    assert!(range_check(&mut builder, v, 30, "fits").is_ok());
    assert_eq!(
        range_check(&mut builder, v, 31, "wraps").err(),
        Some(CircuitError::RangeTooWide { bits: 31, modulus_bits: 31 })
    );
}

#[test]
fn boolean_accepts_zero_and_one_and_rejects_two() {
    let circuit = with_one_input(|b, v| assert_boolean(b, v, "v is a bit"));
    assert_eq!(run(&circuit, F::zero()), Ok(()));
    assert_eq!(run(&circuit, F::one()), Ok(()));
    assert_eq!(run(&circuit, F::from_u64(2)), failed("v is a bit"));
}

#[test]
fn nonzero_accepts_five_and_rejects_zero() {
    let circuit = with_one_input(|b, v| {
        assert_nonzero(b, v, "v is not zero");
    });
    assert_eq!(run(&circuit, F::from_u64(5)), Ok(()));
    assert_eq!(run(&circuit, F::zero()), failed("v is not zero"));
}

#[test]
fn select_returns_the_value_chosen_by_the_bit() {
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let bit = builder.private_input("bit");
    let (seven, nine) = (F::from_u64(7), F::from_u64(9));
    let chosen = select(
        &mut builder,
        bit,
        LinearCombination::constant(seven),
        LinearCombination::constant(nine),
    );
    let circuit = builder.finish().unwrap();
    for (bit, expected) in [(1, 7), (0, 9)] {
        let assignment = Assignment { public: vec![], private: vec![F::from_u64(bit)] };
        let wires = circuit.evaluate(&assignment).unwrap();
        assert_eq!(wires.get(chosen), F::from_u64(expected));
    }
}

fn merkle_matches_native<G: ZkField>() {
    let mut builder = CircuitBuilder::<G>::new().unwrap();
    let leaf = builder.private_input("leaf");
    let bits: Vec<Wire> = (0..3).map(|i| builder.private_input(format!("bit{i}"))).collect();
    let siblings: Vec<Wire> = (0..3).map(|i| builder.private_input(format!("sib{i}"))).collect();
    let root = merkle_root(&mut builder, leaf, &bits, &siblings, "leaf").unwrap();
    let circuit = builder.finish().unwrap();
    let sibling_values = [11, 22, 33].map(G::from_u64);
    let mut roots = Vec::new();
    for path in [[true, false, true], [false, false, true]] {
        let bit_values = path.map(|bit| if bit { G::one() } else { G::zero() });
        let private = [&[G::from_u64(5)][..], &bit_values, &sibling_values].concat();
        let wires = circuit.evaluate(&Assignment { public: vec![], private }).unwrap();
        let native = merkle_root_native(G::from_u64(5), &path, &sibling_values).unwrap();
        assert_eq!(wires.get(root), native);
        roots.push(native);
    }
    assert_ne!(roots[0], roots[1], "the path bits must change the root");
}

#[test]
fn merkle_gadget_agrees_with_native_root_on_both_fields() {
    merkle_matches_native::<Bn254>();
    merkle_matches_native::<BabyBear>();
}

#[test]
fn merkle_path_with_mismatched_lengths_is_rejected() {
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let leaf = builder.private_input("leaf");
    assert_eq!(
        merkle_root(&mut builder, leaf, &[leaf], &[], "leaf").err(),
        Some(CircuitError::PathLengthMismatch { bits: 1, siblings: 0 })
    );
}
