//! PLONKish rows: `q_L·a + q_R·b + q_O·c + q_M·a·b + q_C = 0` with copy constraints.
//!
//! The shape of PLONK, Halo 2 and the Honk family: a table of three wire columns and five
//! selectors, with equalities between cells enforced by a permutation argument or `copy`.

use crate::circuit::Circuit;
use crate::eval::WireValues;
use crate::field::ZkField;
use crate::lc::Wire;
use crate::lower::plonkish_build;
use crate::lower::{Violation, check_length, check_public_count};

/// A wire column of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Column {
    /// Left wire `a`.
    A,
    /// Right wire `b`.
    B,
    /// Output wire `c`.
    C,
}

impl Column {
    /// The three columns in table order.
    pub const ALL: [Column; 3] = [Column::A, Column::B, Column::C];

    /// Position of the column in a row of cell values.
    pub fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::C => 2,
        }
    }
}

/// One cell of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    /// Row index.
    pub row: usize,
    /// Wire column.
    pub column: Column,
}

/// Selector values of one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selectors<F> {
    /// Coefficient of `a`.
    pub q_l: F,
    /// Coefficient of `b`.
    pub q_r: F,
    /// Coefficient of `c`.
    pub q_o: F,
    /// Coefficient of `a·b`.
    pub q_m: F,
    /// Constant.
    pub q_c: F,
}

impl<F: ZkField> Selectors<F> {
    pub(crate) fn zero() -> Self {
        Self { q_l: F::zero(), q_r: F::zero(), q_o: F::zero(), q_m: F::zero(), q_c: F::zero() }
    }

    fn evaluate(&self, [a, b, c]: [F; 3]) -> F {
        let linear = self.q_l.mul(a).add(self.q_r.mul(b)).add(self.q_o.mul(c));
        linear.add(self.q_m.mul(a).mul(b)).add(self.q_c)
    }
}

/// Where a cell's value comes from when the table is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellSource {
    /// Unused; holds zero and every selector touching it is zero.
    Empty,
    /// A circuit wire.
    Wire(Wire),
    /// A chaining temporary in column C: `c = q_L·a + q_R·b + q_C` (the row has `q_O = −1`).
    Chain,
    /// The same value as an earlier cell (a temporary read by a later row).
    CopyOf(Cell),
}

/// A circuit lowered to PLONKish rows.
///
/// Conventions a backend must follow:
/// - **Public inputs.** Rows `0..n` are public-input rows, one per public input in declaration
///   order. Row `i` has the input in column `A`, `q_L = 1` and every other selector zero. The
///   verifier adds `−x_i` to row `i`'s equation, so the row reads `a − x_i = 0` (the PLONK paper's
///   `PI(X) = −Σ x_i·L_i(X)`). A backend with an instance column (Halo 2) instead copies cell
///   `(i, A)` to instance row `i`.
/// - **Gates.** A multiplication `(α·x + β)(γ·y + δ) = z` is one row with `a = x`, `b = y`,
///   `c = z`, `q_M = αγ`, `q_L = αδ`, `q_R = βγ`, `q_O = −1`, `q_C = βδ`. A linear gate or
///   assert-zero with at most three wire terms is one row; longer sums are chained: each extra row
///   computes `q_L·a + q_R·b = c` into a temporary in column `C` that the next row reads.
///   Multiplication factors with several wires are chained into a temporary first.
/// - **Copy constraints.** [`Self::copy_classes`] lists equivalence classes of cells that must
///   hold equal values: each class has at least two cells, sorted by `(row, column)`; classes are
///   sorted by their first cell; a cell is in at most one class. Cells in no class are free.
/// - Unused cells hold zero, and constant wires never occupy a cell (they are folded into `q_C`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plonkish<F> {
    pub(crate) num_public: usize,
    pub(crate) rows: Vec<Selectors<F>>,
    pub(crate) sources: Vec<[CellSource; 3]>,
    pub(crate) copy_classes: Vec<Vec<Cell>>,
}

impl<F: ZkField> Plonkish<F> {
    /// Lowers `circuit`.
    pub fn from_circuit(circuit: &Circuit<F>) -> Self {
        plonkish_build::lower(circuit)
    }

    /// Selector values, one entry per row.
    pub fn rows(&self) -> &[Selectors<F>] {
        &self.rows
    }

    /// Number of rows.
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Number of public-input rows, which are rows `0..n`.
    pub fn num_public_rows(&self) -> usize {
        self.num_public
    }

    /// Equivalence classes of cells that must be equal.
    pub fn copy_classes(&self) -> &[Vec<Cell>] {
        &self.copy_classes
    }

    /// Number of copy classes.
    pub fn num_copy_classes(&self) -> usize {
        self.copy_classes.len()
    }

    /// The `[a, b, c]` values of every row for evaluated wires.
    pub fn cell_values(&self, wires: &WireValues<F>) -> Vec<[F; 3]> {
        let mut cells: Vec<[F; 3]> = Vec::with_capacity(self.rows.len());
        for (selectors, sources) in self.rows.iter().zip(&self.sources) {
            let mut row = [F::zero(); 3];
            for column in Column::ALL {
                row[column.index()] = match sources[column.index()] {
                    CellSource::Empty => F::zero(),
                    CellSource::Wire(wire) => wires.get(wire),
                    CellSource::CopyOf(cell) => cells[cell.row][cell.column.index()],
                    CellSource::Chain => {
                        selectors.q_l.mul(row[0]).add(selectors.q_r.mul(row[1])).add(selectors.q_c)
                    }
                };
            }
            cells.push(row);
        }
        cells
    }

    /// Checks every row equation (with `−x_i` added to public row `i`) and every copy class.
    pub fn check(&self, public: &[F], cells: &[[F; 3]]) -> Result<(), Violation> {
        check_public_count(self.num_public, public.len())?;
        check_length(self.rows.len(), cells.len())?;
        for (index, (selectors, row)) in self.rows.iter().zip(cells).enumerate() {
            let mut value = selectors.evaluate(*row);
            if let Some(x) = public.get(index) {
                value = value.sub(*x);
            }
            if value != F::zero() {
                return Err(Violation::Constraint { index });
            }
        }
        let value_of = |cell: &Cell| cells[cell.row][cell.column.index()];
        for (index, class) in self.copy_classes.iter().enumerate() {
            if let Some(first) = class.first()
                && class.iter().any(|cell| value_of(cell) != value_of(first))
            {
                return Err(Violation::CopyClass { index });
            }
        }
        Ok(())
    }
}
