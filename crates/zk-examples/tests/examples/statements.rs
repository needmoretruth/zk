use zk_circuit::lower::Violation;
use zk_circuit::lower::plonkish::Plonkish;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{Circuit, EvalError, WireValues, ZkField};
use zk_examples::ExampleId;

use crate::fields::{BabyBear, Bn254};

fn lowered_checks<F: ZkField>(
    circuit: &Circuit<F>,
    public: &[F],
    wires: &WireValues<F>,
) -> [Result<(), Violation>; 3] {
    let r1cs = R1cs::from_circuit(circuit);
    let plonkish = Plonkish::from_circuit(circuit);
    let air = WideAir::from_circuit(circuit);
    [
        r1cs.check(public, &r1cs.assignment(wires)),
        plonkish.check(public, &plonkish.cell_values(wires)),
        air.check(public, &air.row(wires)),
    ]
}

fn honest_is_accepted_everywhere<F: ZkField>() {
    for example in ExampleId::ALL {
        let circuit = example.circuit::<F>().unwrap();
        let honest = example.honest::<F>();
        let wires = circuit.evaluate(&honest).unwrap_or_else(|e| panic!("{}: {e}", example.id()));
        let checks = lowered_checks(&circuit, &honest.public, &wires);
        assert_eq!(checks, [Ok(()), Ok(()), Ok(())], "{}", example.id());
    }
}

/// The assertion each dishonest claim is built to break first.
fn expected_first_violation(example: ExampleId) -> &'static str {
    match example {
        ExampleId::OnePlusOne => "answer equals 1 + 1",
        ExampleId::Password => "digest is the hash of the pin",
        ExampleId::Sudoku => "box 0 sums to 10",
        ExampleId::Age => "year - birth_year - threshold fits in 8 bits",
        ExampleId::Membership => "member leaf is under the root",
        ExampleId::Factoring => "p - 2 fits in 15 bits",
        ExampleId::PoolSpend => "value in equals value out",
    }
}

fn dishonest_is_rejected_everywhere<F: ZkField>() {
    for example in ExampleId::ALL {
        let id = example.id();
        let circuit = example.circuit::<F>().unwrap();
        let dishonest = example.dishonest::<F>();
        let label = expected_first_violation(example).to_string();
        assert_eq!(circuit.evaluate(&dishonest), Err(EvalError::AssertionFailed { label }), "{id}");
        let unchecked = circuit.evaluate_unchecked(&dishonest).unwrap();
        for result in lowered_checks(&circuit, &dishonest.public, &unchecked.values) {
            assert!(matches!(result, Err(Violation::Constraint { .. })), "{id}: {result:?}");
        }
    }
}

#[test]
fn honest_claims_pass_evaluation_and_every_lowering_on_bn254() {
    honest_is_accepted_everywhere::<Bn254>();
}

#[test]
fn honest_claims_pass_evaluation_and_every_lowering_on_babybear() {
    honest_is_accepted_everywhere::<BabyBear>();
}

#[test]
fn dishonest_claims_fail_evaluation_and_every_lowering_on_bn254() {
    dishonest_is_rejected_everywhere::<Bn254>();
}

#[test]
fn dishonest_claims_fail_evaluation_and_every_lowering_on_babybear() {
    dishonest_is_rejected_everywhere::<BabyBear>();
}

#[test]
fn example_ids_are_the_permanent_seven() {
    let ids = ExampleId::ALL.map(ExampleId::id);
    let expected =
        ["one-plus-one", "password", "sudoku", "age", "membership", "factoring", "pool-spend"];
    assert_eq!(ids, expected);
    for example in ExampleId::ALL {
        assert_eq!(ExampleId::from_id(example.id()), Some(example));
    }
}

#[test]
fn published_input_names_match_the_circuits_and_assignments() {
    for example in ExampleId::ALL {
        let circuit = example.circuit::<BabyBear>().unwrap();
        let names = |inputs: &[zk_circuit::Input]| -> Vec<String> {
            inputs.iter().map(|input| input.name.clone()).collect()
        };
        assert_eq!(names(circuit.public_inputs()), example.public_input_names());
        assert_eq!(names(circuit.private_inputs()), example.private_input_names());
        let honest = example.honest::<BabyBear>();
        assert_eq!(honest.public.len(), circuit.public_inputs().len());
        assert_eq!(honest.private.len(), circuit.private_inputs().len());
    }
}
