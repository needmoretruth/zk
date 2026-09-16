use zk_circuit::{CircuitBuilder, CircuitError, LinearCombination};

use crate::fields::BabyBear;

#[test]
fn an_input_name_used_twice_is_rejected() {
    let mut builder = CircuitBuilder::<BabyBear>::new().unwrap();
    builder.public_input("x");
    builder.private_input("x");
    assert_eq!(
        builder.finish().err(),
        Some(CircuitError::DuplicateInputName { name: "x".to_string() })
    );
}

#[test]
fn a_wire_from_another_builder_is_rejected() {
    let mut other = CircuitBuilder::<BabyBear>::new().unwrap();
    let foreign = (0..3).map(|i| other.private_input(format!("v{i}"))).last().unwrap();
    let mut builder = CircuitBuilder::<BabyBear>::new().unwrap();
    let own = builder.private_input("own");
    builder.assert_zero(LinearCombination::from(own) - foreign, "own equals foreign");
    assert_eq!(builder.finish().err(), Some(CircuitError::UndefinedWire { gate: 1, wire: 2 }));
}
