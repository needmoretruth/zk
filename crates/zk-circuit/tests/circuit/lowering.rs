use zk_circuit::gadgets::{assert_nonzero, range_check};
use zk_circuit::lower::Violation;
use zk_circuit::lower::plonkish::{Column, Plonkish};
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::lower::wide_air::WideAir;
use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, Gate, LinearCombination, WireValues, ZkField,
};

use crate::fields::{BabyBear, Bn254};

type Lc<F> = LinearCombination<F>;

/// Exercises every lowering path: two public inputs used asymmetrically, constant wires,
/// multiplication factors with several wires and constants, a sum long enough to chain, and hints.
///
/// Honest witness: `a = 4, b = 7` public, `x = 3, y = 5` private.
fn mixed<F: ZkField>() -> Circuit<F> {
    let n = F::from_u64;
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let a = builder.public_input("a");
    let b = builder.public_input("b");
    let x = builder.private_input("x");
    let y = builder.private_input("y");
    let one = builder.constant(F::one());
    let seven = builder.constant(n(7));
    let xy = builder.mul(x, y); // 15
    let factor = Lc::<F>::from(x).add_term(y, n(2)) + one; // 14
    let skew = builder.mul(factor, Lc::<F>::from(y) - seven); // -28
    let s = builder.linear(Lc::<F>::from(xy) + skew + x + y + Lc::from(a).scale(n(3))); // 7
    builder.assert_zero(Lc::<F>::from(s) - b, "s equals b");
    range_check(&mut builder, x, 4, "x fits in 4 bits").unwrap();
    assert_nonzero(&mut builder, Lc::<F>::from(y) - one, "y is not one");
    let long = Lc::<F>::from(a) + Lc::from(b).scale(n(2)) + x + y + xy + skew;
    builder.assert_zero(long.add_constant(n(13).neg()), "the long sum is 13");
    builder.finish().unwrap()
}

fn assignment<F: ZkField>(public: [u64; 2], private: [u64; 2]) -> Assignment<F> {
    Assignment {
        public: public.map(F::from_u64).to_vec(),
        private: private.map(F::from_u64).to_vec(),
    }
}

/// The three checkers on the assignments each lowering derives from `wires`.
fn check_all<F: ZkField>(
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

fn honest_witness_satisfies_every_lowering<F: ZkField>() {
    let circuit = mixed::<F>();
    let honest = assignment::<F>([4, 7], [3, 5]);
    let wires = circuit.evaluate(&honest).unwrap();
    assert_eq!(check_all(&circuit, &honest.public, &wires), [Ok(()), Ok(()), Ok(())]);
}

#[test]
fn honest_witness_satisfies_every_lowering_on_both_fields() {
    honest_witness_satisfies_every_lowering::<Bn254>();
    honest_witness_satisfies_every_lowering::<BabyBear>();
}

#[test]
fn a_false_witness_fails_every_lowering() {
    let circuit = mixed::<BabyBear>();
    let wrong = assignment::<BabyBear>([4, 7], [3, 6]);
    let evaluation = circuit.evaluate_unchecked(&wrong).unwrap();
    assert!(!evaluation.violated.is_empty());
    for result in check_all(&circuit, &wrong.public, &evaluation.values) {
        assert!(matches!(result, Err(Violation::Constraint { .. })), "{result:?}");
    }
}

#[test]
fn swapped_public_inputs_fail_every_lowering() {
    let circuit = mixed::<BabyBear>();
    let honest = assignment::<BabyBear>([4, 7], [3, 5]);
    let wires = circuit.evaluate(&honest).unwrap();
    let swapped = [honest.public[1], honest.public[0]];
    for result in check_all(&circuit, &swapped, &wires) {
        assert!(result.is_err());
    }
}

#[test]
fn r1cs_layout_is_one_then_public_then_private_then_internal() {
    let circuit = mixed::<BabyBear>();
    let honest = assignment::<BabyBear>([4, 7], [3, 5]);
    let r1cs = R1cs::from_circuit(&circuit);
    let z = r1cs.assignment(&circuit.evaluate(&honest).unwrap());
    assert_eq!(z[..5], [1, 4, 7, 3, 5].map(BabyBear::from_u64));
    let count = |f: fn(&Gate<BabyBear>) -> usize| circuit.gates().iter().map(f).sum::<usize>();
    let muls = count(|g| usize::from(matches!(g, Gate::Mul { .. })));
    let asserts = count(|g| usize::from(matches!(g, Gate::AssertZero { .. })));
    let hinted = count(|g| if let Gate::Hint { outputs, .. } = g { outputs.len() } else { 0 });
    assert_eq!(r1cs.num_constraints(), muls + asserts);
    assert_eq!(r1cs.num_variables(), 1 + 4 + muls + hinted);
    assert_eq!(r1cs.num_public_inputs(), 2);
}

#[test]
fn plonkish_public_rows_hold_public_inputs_in_column_a_in_order() {
    let circuit = mixed::<BabyBear>();
    let honest = assignment::<BabyBear>([4, 7], [3, 5]);
    let plonkish = Plonkish::from_circuit(&circuit);
    let cells = plonkish.cell_values(&circuit.evaluate(&honest).unwrap());
    assert_eq!(plonkish.num_public_rows(), 2);
    for (row, value) in honest.public.iter().enumerate() {
        let selectors = plonkish.rows()[row];
        assert_eq!(selectors.q_l, BabyBear::one());
        let others = [selectors.q_r, selectors.q_o, selectors.q_m, selectors.q_c];
        assert_eq!(others, [BabyBear::zero(); 4]);
        assert_eq!(cells[row][Column::A.index()], *value);
    }
}

#[test]
fn plonkish_rejects_a_table_whose_rows_hold_but_whose_copies_differ() {
    type F = BabyBear;
    let mut builder = CircuitBuilder::<F>::new().unwrap();
    let x = builder.public_input("x");
    let y = builder.private_input("y");
    let product = builder.mul(x, y);
    builder.assert_zero(Lc::<F>::from(product).add_constant(F::from_u64(6).neg()), "x·y = 6");
    let circuit = builder.finish().unwrap();
    let honest = Assignment { public: vec![F::from_u64(2)], private: vec![F::from_u64(3)] };
    let plonkish = Plonkish::from_circuit(&circuit);
    let mut cells = plonkish.cell_values(&circuit.evaluate(&honest).unwrap());
    assert_eq!(plonkish.check(&honest.public, &cells), Ok(()));
    // Row 1 is the multiplication. Reading x as 1 and y as 6 keeps 1·6 = 6, so only the copy
    // constraint tying row 1's `a` to public row 0 can catch it.
    cells[1][Column::A.index()] = F::from_u64(1);
    cells[1][Column::B.index()] = F::from_u64(6);
    assert!(matches!(plonkish.check(&honest.public, &cells), Err(Violation::CopyClass { .. })));
}

#[test]
fn plonkish_copy_classes_are_sorted_disjoint_and_nontrivial() {
    let plonkish = Plonkish::from_circuit(&mixed::<BabyBear>());
    let mut seen = std::collections::HashSet::new();
    for class in plonkish.copy_classes() {
        assert!(class.len() >= 2);
        assert!(class.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(class.iter().all(|cell| seen.insert(*cell)));
    }
    let firsts: Vec<_> = plonkish.copy_classes().iter().map(|class| class[0]).collect();
    assert!(firsts.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn wide_air_has_a_column_per_non_constant_wire_and_degree_at_most_two() {
    let circuit = mixed::<BabyBear>();
    let air = WideAir::from_circuit(&circuit);
    let constants = circuit.gates().iter().filter(|g| matches!(g, Gate::Constant { .. })).count();
    let constraining = circuit
        .gates()
        .iter()
        .filter(|g| matches!(g, Gate::Linear { .. } | Gate::Mul { .. } | Gate::AssertZero { .. }))
        .count();
    assert_eq!(air.num_columns(), circuit.num_wires() - constants);
    assert_eq!(air.num_constraints(), constraining);
    assert_eq!(air.max_degree(), 2);
    let publics = [4, 7].map(BabyBear::from_u64);
    let boundary = air.boundary(&publics).unwrap();
    assert_eq!(boundary, [(0, publics[0]), (1, publics[1])]);
}
