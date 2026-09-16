//! Building the PLONKish table from a circuit; the conventions are documented on
//! [`crate::lower::plonkish::Plonkish`].

use std::collections::VecDeque;

use crate::circuit::{Circuit, Gate};
use crate::field::ZkField;
use crate::lc::{LinearCombination, Wire};
use crate::lower::constants::ConstantWires;
use crate::lower::plonkish::{Cell, CellSource, Column, Plonkish, Selectors};

/// Something a cell can hold: a circuit wire, or a chaining temporary by index.
#[derive(Clone, Copy)]
enum Operand {
    Wire(Wire),
    Temp(usize),
}

pub(crate) fn lower<F: ZkField>(circuit: &Circuit<F>) -> Plonkish<F> {
    let constants = ConstantWires::of(circuit);
    let mut table = Table::new(circuit.num_wires());
    for input in circuit.public_inputs() {
        let selectors = Selectors { q_l: F::one(), ..Selectors::zero() };
        table.push_row(selectors, [Some(Operand::Wire(input.wire)), None, None]);
    }
    for gate in circuit.gates() {
        match gate {
            Gate::Constant { .. } | Gate::Input { .. } | Gate::Hint { .. } => {}
            Gate::Linear { output, lc } => {
                let folded = constants.fold(lc);
                let mut terms = wire_terms(&folded);
                terms.push_back((Operand::Wire(*output), F::one().neg()));
                table.linear(terms, folded.constant_term());
            }
            Gate::Mul { output, left, right } => {
                table.mul(&constants.fold(left), &constants.fold(right), *output);
            }
            Gate::AssertZero { lc, .. } => {
                let folded = constants.fold(lc);
                table.linear(wire_terms(&folded), folded.constant_term());
            }
        }
    }
    table.finish(circuit.public_inputs().len())
}

fn wire_terms<F: ZkField>(lc: &LinearCombination<F>) -> VecDeque<(Operand, F)> {
    lc.terms().iter().map(|(wire, coefficient)| (Operand::Wire(*wire), *coefficient)).collect()
}

struct Table<F> {
    rows: Vec<Selectors<F>>,
    sources: Vec<[CellSource; 3]>,
    wire_cells: Vec<Vec<Cell>>,
    temp_cells: Vec<Vec<Cell>>,
}

impl<F: ZkField> Table<F> {
    fn new(num_wires: usize) -> Self {
        Self {
            rows: Vec::new(),
            sources: Vec::new(),
            wire_cells: vec![Vec::new(); num_wires],
            temp_cells: Vec::new(),
        }
    }

    fn push_row(&mut self, selectors: Selectors<F>, operands: [Option<Operand>; 3]) -> usize {
        let row = self.rows.len();
        self.rows.push(selectors);
        let mut sources = [CellSource::Empty; 3];
        for (column, operand) in Column::ALL.into_iter().zip(operands) {
            let cell = Cell { row, column };
            sources[column.index()] = match operand {
                None => CellSource::Empty,
                Some(Operand::Wire(wire)) => {
                    self.wire_cells[wire.index()].push(cell);
                    CellSource::Wire(wire)
                }
                Some(Operand::Temp(temp)) => {
                    let holders = &mut self.temp_cells[temp];
                    let source =
                        holders.first().map_or(CellSource::Empty, |d| CellSource::CopyOf(*d));
                    holders.push(cell);
                    source
                }
            };
        }
        self.sources.push(sources);
        row
    }

    /// Emits `c1·x1 + c2·x2 − temp = 0` and returns the new temporary.
    fn chain(&mut self, (x1, c1): (Operand, F), (x2, c2): (Operand, F)) -> Operand {
        let selectors = Selectors { q_l: c1, q_r: c2, q_o: F::one().neg(), ..Selectors::zero() };
        let row = self.push_row(selectors, [Some(x1), Some(x2), None]);
        self.sources[row][Column::C.index()] = CellSource::Chain;
        self.temp_cells.push(vec![Cell { row, column: Column::C }]);
        Operand::Temp(self.temp_cells.len() - 1)
    }

    /// Emits `Σ terms + constant = 0`, chaining until the last row holds at most three terms.
    fn linear(&mut self, mut terms: VecDeque<(Operand, F)>, constant: F) {
        while terms.len() > 3 {
            if let (Some(first), Some(second)) = (terms.pop_front(), terms.pop_front()) {
                let temp = self.chain(first, second);
                terms.push_front((temp, F::one()));
            }
        }
        let mut selectors = Selectors { q_c: constant, ..Selectors::zero() };
        let mut operands = [None; 3];
        for (column, (operand, coefficient)) in Column::ALL.into_iter().zip(terms) {
            operands[column.index()] = Some(operand);
            match column {
                Column::A => selectors.q_l = coefficient,
                Column::B => selectors.q_r = coefficient,
                Column::C => selectors.q_o = coefficient,
            }
        }
        self.push_row(selectors, operands);
    }

    /// Reduces a multi-wire factor to one operand, returning it with its coefficient.
    fn single(&mut self, mut terms: VecDeque<(Operand, F)>) -> Option<(Operand, F)> {
        while terms.len() > 1 {
            if let (Some(first), Some(second)) = (terms.pop_front(), terms.pop_front()) {
                let temp = self.chain(first, second);
                terms.push_front((temp, F::one()));
            }
        }
        terms.pop_front()
    }

    /// Emits `(α·x + β)(γ·y + δ) − z = 0` in one row after reducing both factors.
    fn mul(&mut self, left: &LinearCombination<F>, right: &LinearCombination<F>, output: Wire) {
        let x = self.single(wire_terms(left));
        let y = self.single(wire_terms(right));
        let alpha = x.map_or(F::zero(), |(_, coefficient)| coefficient);
        let gamma = y.map_or(F::zero(), |(_, coefficient)| coefficient);
        let (beta, delta) = (left.constant_term(), right.constant_term());
        let selectors = Selectors {
            q_l: alpha.mul(delta),
            q_r: beta.mul(gamma),
            q_o: F::one().neg(),
            q_m: alpha.mul(gamma),
            q_c: beta.mul(delta),
        };
        let operands = [x.map(|(o, _)| o), y.map(|(o, _)| o), Some(Operand::Wire(output))];
        self.push_row(selectors, operands);
    }

    fn finish(self, num_public: usize) -> Plonkish<F> {
        let mut copy_classes: Vec<Vec<Cell>> = self
            .wire_cells
            .into_iter()
            .chain(self.temp_cells)
            .filter(|cells| cells.len() >= 2)
            .map(|mut cells| {
                cells.sort();
                cells
            })
            .collect();
        copy_classes.sort_by_key(|cells| cells.first().copied());
        Plonkish { num_public, rows: self.rows, sources: self.sources, copy_classes }
    }
}
