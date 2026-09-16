//! The museum's lowered PLONKish table, laid out as a Halo 2 circuit.
//!
//! Halo 2 lets a circuit declare its own columns and gates, so the table maps onto it one to one:
//! three advice columns `a`, `b`, `c`, five fixed columns holding the selectors, one instance
//! column, a single custom gate, and a copy constraint for every link in every copy class.
//!
//! Public inputs follow the lowering's convention literally. Row `i < n` keeps `q_L = 1`, and the
//! gate subtracts the instance column, so that row reads `a − x_i = 0` — the verifier-added `−x_i`
//! of the PLONK paper. The instance column is zero on every other row (Halo 2 pads it with zeros,
//! and never blinds it), so the subtraction changes nothing elsewhere. Cell `(i, a)` is also
//! bound to instance row `i` with `constrain_instance`, the way Orchard exposes its public inputs;
//! the two constraints state the same equation, and either alone would suffice.

use std::sync::Arc;

use halo2_proofs::circuit::floor_planner::V1;
use halo2_proofs::circuit::{Cell, Layouter, Region, Value};
use halo2_proofs::pasta::Fp;
use halo2_proofs::plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance};
use halo2_proofs::poly::Rotation;
use zk_circuit::lower::plonkish::{self, Plonkish};

use crate::field::PastaFp;

/// Wire columns `a`, `b`, `c`.
pub(crate) const ADVICE_COLUMNS: usize = 3;
/// Selector columns `q_L`, `q_R`, `q_O`, `q_M`, `q_C`.
pub(crate) const FIXED_COLUMNS: usize = 5;
/// The public inputs.
pub(crate) const INSTANCE_COLUMNS: usize = 1;

/// The columns every table uses; the same for all seven statements, so keys differ only in values.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TableConfig {
    advice: [Column<Advice>; ADVICE_COLUMNS],
    selectors: [Column<Fixed>; FIXED_COLUMNS],
    instance: Column<Instance>,
}

/// One statement's table, with the prover's cell values when there are any.
///
/// Key generation sees only the table; proving adds the values the circuit layer computed.
#[derive(Clone, Debug)]
pub(crate) struct TableCircuit {
    table: Arc<Plonkish<PastaFp>>,
    cells: Option<Vec<[PastaFp; 3]>>,
}

impl TableCircuit {
    /// The circuit key generation needs: layout and selectors, no witness.
    pub(crate) fn shape(table: Arc<Plonkish<PastaFp>>) -> Self {
        Self { table, cells: None }
    }

    /// The circuit the prover needs: the same layout filled with `cells`, one `[a, b, c]` per row.
    pub(crate) fn with_cells(table: Arc<Plonkish<PastaFp>>, cells: Vec<[PastaFp; 3]>) -> Self {
        Self { table, cells: Some(cells) }
    }

    /// A cell's value, unknown during key generation; a missing row is unknown too, which Halo 2
    /// turns into a synthesis error rather than a panic.
    fn value(&self, row: usize, column: plonkish::Column) -> Value<Fp> {
        match self.cells.as_ref().and_then(|cells| cells.get(row)) {
            Some(values) => Value::known(values[column.index()].0),
            None => Value::unknown(),
        }
    }

    /// Assigns one row's selectors and wires and returns its three wire cells.
    fn assign_row(
        &self,
        region: &mut Region<'_, Fp>,
        config: &TableConfig,
        row: usize,
        selectors: &plonkish::Selectors<PastaFp>,
    ) -> Result<[Cell; 3], Error> {
        let values = [selectors.q_l, selectors.q_r, selectors.q_o, selectors.q_m, selectors.q_c];
        for (column, value) in config.selectors.into_iter().zip(values) {
            region.assign_fixed(|| "selector", column, row, || Value::known(value.0))?;
        }
        let mut cells = Vec::with_capacity(ADVICE_COLUMNS);
        for wire in plonkish::Column::ALL {
            let column = config.advice[wire.index()];
            let value = self.value(row, wire);
            cells.push(region.assign_advice(|| "wire", column, row, || value)?.cell());
        }
        match cells[..] {
            [a, b, c] => Ok([a, b, c]),
            _ => Err(Error::Synthesis),
        }
    }
}

impl Circuit<Fp> for TableCircuit {
    type Config = TableConfig;
    /// The floor planner Orchard uses. A lone region is placed at row 0, which the gate's instance
    /// term relies on: table row `i` must be absolute row `i`.
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::shape(Arc::clone(&self.table))
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> TableConfig {
        let advice = [(); ADVICE_COLUMNS].map(|()| meta.advice_column());
        for column in advice {
            meta.enable_equality(column);
        }
        let selectors = [(); FIXED_COLUMNS].map(|()| meta.fixed_column());
        let instance = meta.instance_column();
        meta.enable_equality(instance);
        meta.create_gate("q_L·a + q_R·b + q_O·c + q_M·a·b + q_C − x = 0", |meta| {
            let [a, b, c] = advice.map(|column| meta.query_advice(column, Rotation::cur()));
            let [q_l, q_r, q_o, q_m, q_c] = selectors.map(|column| meta.query_fixed(column));
            let x = meta.query_instance(instance, Rotation::cur());
            vec![q_l * a.clone() + q_r * b.clone() + q_o * c + q_m * a * b + q_c - x]
        });
        TableConfig { advice, selectors, instance }
    }

    fn synthesize(
        &self,
        config: TableConfig,
        mut layouter: impl Layouter<Fp>,
    ) -> Result<(), Error> {
        let public_cells = layouter.assign_region(
            || "PLONKish table",
            |mut region| {
                let mut cells = Vec::with_capacity(self.table.num_rows());
                for (row, selectors) in self.table.rows().iter().enumerate() {
                    cells.push(self.assign_row(&mut region, &config, row, selectors)?);
                }
                for class in self.table.copy_classes() {
                    for link in class.windows(2) {
                        let [left, right] =
                            [link[0], link[1]].map(|cell| cells[cell.row][cell.column.index()]);
                        region.constrain_equal(left, right)?;
                    }
                }
                let public = cells.iter().take(self.table.num_public_rows());
                Ok(public.map(|row| row[plonkish::Column::A.index()]).collect::<Vec<_>>())
            },
        )?;
        for (row, cell) in public_cells.into_iter().enumerate() {
            layouter.constrain_instance(cell, config.instance, row)?;
        }
        Ok(())
    }
}
