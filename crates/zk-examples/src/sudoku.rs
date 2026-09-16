//! `sudoku`: I solved this 4×4 sudoku (2×2 boxes).
//!
//! Teaches how a puzzle becomes constraints. Every cell is in {1, 2, 3, 4}; every row, column and
//! box sums to 10 and multiplies to 24.
//!
//! Why sum and product suffice: with values in {1, 2, 3, 4}, a product of 24 = 2^3·3 needs exactly
//! one 3 (two would make 9 divide it), leaving three values from {1, 2, 4} multiplying to 8, which
//! are {1, 2, 4} or {2, 2, 2}. The sum rules out {3, 2, 2, 2} (9) and keeps {1, 2, 3, 4} (10). No
//! value exceeds 4 and no product exceeds 256, so nothing wraps in a field above 2^30.

use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, Wire, ZkField,
};

/// Permanent ID.
pub const ID: &str = "sudoku";

const SIZE: usize = 4;

/// The unique solution of [`GIVENS`].
const SOLUTION: [[u64; SIZE]; SIZE] = [[1, 2, 3, 4], [3, 4, 1, 2], [2, 1, 4, 3], [4, 3, 2, 1]];

/// The puzzle: 0 marks an empty cell. Rows 0 and 3 given, which forces [`SOLUTION`].
const GIVENS: [[u64; SIZE]; SIZE] = [[1, 2, 3, 4], [0, 0, 0, 0], [0, 0, 0, 0], [4, 3, 2, 1]];

/// A Latin square that agrees with every given: rows and columns are valid, boxes are not
/// (the top-left box holds 1, 2, 2, 1). With valid rows and columns, broken boxes come in pairs.
const BROKEN_BOXES: [[u64; SIZE]; SIZE] = [[1, 2, 3, 4], [2, 1, 4, 3], [3, 4, 1, 2], [4, 3, 2, 1]];

fn cell_names(prefix: &str) -> Vec<String> {
    (0..SIZE * SIZE).map(|i| format!("{prefix}_r{}c{}", i / SIZE, i % SIZE)).collect()
}

/// `given_r{row}c{column}` for the 16 cells in row order; 0 means empty.
pub fn public_input_names() -> Vec<String> {
    cell_names("given")
}

/// `cell_r{row}c{column}` for the 16 cells in row order.
pub fn private_input_names() -> Vec<String> {
    cell_names("cell")
}

/// Cell ranges, then rows, columns and boxes, then givens (`g·(c − g) = 0`).
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let givens: Vec<Wire> =
        public_input_names().into_iter().map(|n| builder.public_input(n)).collect();
    let cells: Vec<Wire> =
        private_input_names().into_iter().map(|n| builder.private_input(n)).collect();
    for (index, cell) in cells.iter().enumerate() {
        assert_digit(
            &mut builder,
            *cell,
            &format!("cell r{}c{} is 1, 2, 3 or 4", index / SIZE, index % SIZE),
        );
    }
    for (name, group) in groups() {
        let members: Vec<Wire> = group.iter().map(|index| cells[*index]).collect();
        assert_permutation(&mut builder, &members, &name);
    }
    for (index, (given, cell)) in givens.iter().zip(&cells).enumerate() {
        let mismatch = builder.mul(*given, LinearCombination::<F>::from(*cell) - *given);
        builder.assert_zero(
            mismatch,
            format!("cell r{}c{} matches its given", index / SIZE, index % SIZE),
        );
    }
    builder.finish()
}

/// `(c − 1)(c − 2)(c − 3)(c − 4) = 0`.
fn assert_digit<F: ZkField>(builder: &mut CircuitBuilder<F>, cell: Wire, label: &str) {
    let minus = |k: u64| LinearCombination::<F>::from(cell).add_constant(F::from_u64(k).neg());
    let first = builder.mul(minus(1), minus(2));
    let second = builder.mul(first, minus(3));
    let third = builder.mul(second, minus(4));
    builder.assert_zero(third, label);
}

/// Sum 10 and product 24.
fn assert_permutation<F: ZkField>(builder: &mut CircuitBuilder<F>, members: &[Wire], name: &str) {
    let sum = members
        .iter()
        .fold(LinearCombination::<F>::constant(F::from_u64(10).neg()), |acc, m| acc + *m);
    builder.assert_zero(sum, format!("{name} sums to 10"));
    if let Some((first, rest)) = members.split_first() {
        let mut product = LinearCombination::<F>::from(*first);
        for member in rest {
            product = builder.mul(product, *member).into();
        }
        let label = format!("{name} multiplies to 24");
        builder.assert_zero(product.add_constant(F::from_u64(24).neg()), label);
    }
}

/// The 12 groups as (name, cell indices): rows, then columns, then boxes in row order.
fn groups() -> Vec<(String, Vec<usize>)> {
    let rows = (0..SIZE).map(|r| (format!("row {r}"), (0..SIZE).map(|c| r * SIZE + c).collect()));
    let columns =
        (0..SIZE).map(|c| (format!("column {c}"), (0..SIZE).map(|r| r * SIZE + c).collect()));
    let boxes = (0..SIZE).map(|b| {
        let (top, left) = (b / 2 * 2, b % 2 * 2);
        let cells = (0..SIZE).map(|i| (top + i / 2) * SIZE + left + i % 2).collect();
        (format!("box {b}"), cells)
    });
    rows.chain(columns).chain(boxes).collect()
}

fn with_cells<F: ZkField>(grid: &[[u64; SIZE]; SIZE]) -> Assignment<F> {
    Assignment {
        public: crate::values(GIVENS.as_flattened()),
        private: crate::values(grid.as_flattened()),
    }
}

/// The solution of the puzzle.
pub fn honest<F: ZkField>() -> Assignment<F> {
    with_cells(&SOLUTION)
}

/// A grid whose rows, columns and givens all check out but whose boxes do not.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    with_cells(&BROKEN_BOXES)
}
