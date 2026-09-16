//! The museum's circuit written out as a Miden Assembly program.
//!
//! Every wire lives in one VM memory cell, at the address equal to its wire index. Gates run in
//! circuit order, so a cell is always written before it is read:
//! - a public input is popped off the operand stack, where the verifier's stack inputs put it;
//! - a private input or a hint output is pushed from the advice stack with `adv_push`, the VM's
//!   channel for values only the prover knows;
//! - a constant is pushed as an immediate;
//! - a linear gate loads its terms, scales and adds them; a multiplication gate evaluates both
//!   sides and multiplies;
//! - an assert-zero gate evaluates its combination and runs `assertz`, whose error message is the
//!   gate's label.
//!
//! The text depends only on the circuit, never on a witness, so one statement always compiles to
//! one program with one hash. After the last gate the operand stack holds sixteen zeros, the
//! stack outputs every proof claims.

use std::collections::BTreeMap;

use miden_core::Felt;
use miden_core::mast::error_code_from_msg;
use zk_circuit::{Circuit, Gate, LinearCombination, Visibility, WireValues, ZkField};

use crate::field::{self, Goldilocks};

/// A program's source text and what its assertion codes mean.
pub(crate) struct Source {
    /// Miden Assembly, one line per gate.
    pub(crate) text: String,
    /// The circuit's label for each error code an `assertz` can fail with.
    pub(crate) labels: BTreeMap<u64, String>,
}

/// Writes `circuit` as a program.
pub(crate) fn write(circuit: &Circuit<Goldilocks>) -> Source {
    let mut lines = vec!["begin".to_string()];
    let mut labels = BTreeMap::new();
    for gate in circuit.gates() {
        let mut line = Line::default();
        match gate {
            Gate::Constant { output, value } => {
                line.push(format!("push.{}", field::literal(*value)));
                line.store(output.index());
            }
            Gate::Input { output, name, visibility: Visibility::Public } => {
                line.store(output.index());
                line.comment = Some(format!("public input {name}"));
            }
            Gate::Input { output, name, visibility: Visibility::Private } => {
                line.push("adv_push".to_string());
                line.store(output.index());
                line.comment = Some(format!("private input {name}"));
            }
            Gate::Linear { output, lc } => {
                line.combination(lc);
                line.store(output.index());
            }
            Gate::Mul { output, left, right } => {
                line.combination(left);
                line.combination(right);
                line.push("mul".to_string());
                line.store(output.index());
            }
            Gate::AssertZero { lc, label } => {
                let message = message(label);
                labels.insert(error_code_from_msg(&message).as_canonical_u64(), label.clone());
                line.combination(lc);
                line.push(format!("assertz.err=\"{message}\""));
            }
            Gate::Hint { outputs, .. } => {
                for output in outputs {
                    line.push("adv_push".to_string());
                    line.store(output.index());
                }
            }
        }
        lines.push(line.render());
    }
    lines.push("end".to_string());
    Source { text: lines.join("\n"), labels }
}

/// The advice stack for one evaluation: private inputs and hint outputs, in the order the program
/// pushes them.
pub(crate) fn advice(circuit: &Circuit<Goldilocks>, values: &WireValues<Goldilocks>) -> Vec<Felt> {
    let mut advice = Vec::new();
    for gate in circuit.gates() {
        match gate {
            Gate::Input { output, visibility: Visibility::Private, .. } => {
                advice.push(values.get(*output).0);
            }
            Gate::Hint { outputs, .. } => {
                advice.extend(outputs.iter().map(|output| values.get(*output).0));
            }
            _ => {}
        }
    }
    advice
}

/// One gate's instructions, with an optional comment naming the input it reads.
#[derive(Default)]
struct Line {
    instructions: Vec<String>,
    comment: Option<String>,
}

impl Line {
    fn push(&mut self, instruction: String) {
        self.instructions.push(instruction);
    }

    /// Pops the top of the stack into the wire's memory cell.
    fn store(&mut self, wire: usize) {
        self.push(format!("mem_store.{wire}"));
    }

    /// Leaves `Σ c·wire + constant` on top of the stack.
    fn combination(&mut self, lc: &LinearCombination<Goldilocks>) {
        let lc = lc.normalized();
        let constant = lc.constant_term();
        let mut first = true;
        for (wire, coefficient) in lc.terms() {
            self.push(format!("mem_load.{}", wire.index()));
            if *coefficient == Goldilocks::one().neg() {
                self.push("neg".to_string());
            } else if *coefficient != Goldilocks::one() {
                self.push(format!("mul.{}", field::literal(*coefficient)));
            }
            if !first {
                self.push("add".to_string());
            }
            first = false;
        }
        if first {
            self.push(format!("push.{}", field::literal(constant)));
        } else if constant != Goldilocks::zero() {
            self.push(format!("add.{}", field::literal(constant)));
        }
    }

    fn render(&self) -> String {
        let body = self.instructions.join(" ");
        match &self.comment {
            Some(comment) => format!("    {body} # {comment}"),
            None => format!("    {body}"),
        }
    }
}

/// A label as an assertion message: printable ASCII only, without the quote and backslash that
/// would end or escape the string literal.
fn message(label: &str) -> String {
    label
        .chars()
        .map(|c| if (c.is_ascii_graphic() || c == ' ') && c != '"' && c != '\\' { c } else { '?' })
        .collect()
}
