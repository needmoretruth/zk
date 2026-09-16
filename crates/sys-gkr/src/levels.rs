//! The graph laid out level by level: which slot every value occupies and the gates between levels.
//!
//! Level `ℓ + 1` is computed from level `ℓ` alone. A value needed later than the level after its own
//! is carried up one level at a time by a copy gate, `copy = value · one`, the only way to copy with
//! a gate layer that multiplies. Level 1 is the public table plus the committed table lifted into
//! the empty slots of the same positions; the output reads the last level, one slot per check.

use std::collections::HashMap;

use shared_types::Fr;
use shared_types::halo2curves::ff::Field as _;

use crate::graph::{Graph, NodeId, ONE, Origin};

/// A gate `(output slot, left slot, right slot)` in Remainder's wiring format.
pub(crate) type Gate = (u32, u32, u32);

/// Slots, wiring and table sizes for one graph.
pub(crate) struct Layout {
    /// The value in each slot of each level; `levels[0]` is level 1.
    levels: Vec<Vec<NodeId>>,
    /// Gates lifting committed entry `i` (times the public one) into its level-1 slot.
    pub(crate) lift: Vec<Gate>,
    /// `steps[i]` computes level `i + 2` from level `i + 1`.
    pub(crate) steps: Vec<Vec<Gate>>,
    /// Gates copying every check from the last level into the output, which must be all zero.
    pub(crate) output: Vec<Gate>,
    /// `log2` of the public table's length, which is level 1's length.
    pub(crate) public_bits: usize,
    /// `log2` of the committed table's length.
    pub(crate) committed_bits: usize,
}

impl Layout {
    /// Lays `graph` out; fails for a graph with no checks or nothing to commit to.
    pub(crate) fn of(graph: &Graph) -> Result<Self, String> {
        let committed = graph.committed_variables.len();
        if graph.checks.is_empty() || committed == 0 {
            return Err("a layered circuit needs at least one check and one private value".into());
        }
        let (last, top) = lifetimes(graph);
        let levels: Vec<Vec<NodeId>> =
            (1..=top).map(|level| present(graph, &last, level)).collect();
        let slots: Vec<HashMap<NodeId, u32>> = levels.iter().map(|nodes| slot_map(nodes)).collect();
        let public_bits = bits(levels[0].len());
        let lift = lift(graph, &slots[0], public_bits);
        let steps = (1..top).map(|level| step(graph, &levels, &slots, level)).collect();
        let at_top = &slots[top - 1];
        let mut output: Vec<Gate> = (0u32..)
            .zip(&graph.checks)
            .map(|(index, check)| (index, at_top[check], at_top[&ONE]))
            .collect();
        if let [only] = output.as_slice() {
            // Remainder sizes a gate layer by its highest output slot; two slots keep one variable.
            output.push((1, only.1, only.2));
        }
        Ok(Self { levels, lift, steps, output, public_bits, committed_bits: bits(committed) })
    }

    /// Remainder layers between the inputs and the output: the lift, level 1, each step, the output.
    pub(crate) fn num_layers(&self) -> usize {
        self.steps.len() + 3
    }

    /// Every gate, copies included.
    pub(crate) fn num_gates(&self) -> usize {
        self.lift.len() + self.steps.iter().map(Vec::len).sum::<usize>() + self.output.len()
    }

    /// The public table: one, the constants and the public inputs in their level-1 slots, zero in
    /// the slots the committed values are lifted into and in the padding.
    pub(crate) fn public_table(&self, graph: &Graph, public: &[Fr]) -> Vec<Fr> {
        let mut table = vec![Fr::ZERO; 1 << self.public_bits];
        for (slot, node) in table.iter_mut().zip(&self.levels[0]) {
            *slot = match &graph.nodes[*node] {
                Origin::One => Fr::ONE,
                Origin::Constant(value) => *value,
                Origin::Public(index) => public.get(*index).copied().unwrap_or(Fr::ZERO),
                Origin::Committed(_) | Origin::Gates(_) => Fr::ZERO,
            };
        }
        table
    }

    /// The committed table: the R1CS assignment's private inputs and hint outputs, zero-padded.
    pub(crate) fn committed_table(&self, graph: &Graph, assignment: &[Fr]) -> Vec<Fr> {
        let mut table = vec![Fr::ZERO; 1 << self.committed_bits];
        for (slot, variable) in table.iter_mut().zip(&graph.committed_variables) {
            *slot = assignment.get(*variable).copied().unwrap_or(Fr::ZERO);
        }
        table
    }
}

/// `log2` of the smallest power of two holding `length` values, at least one variable.
fn bits(length: usize) -> usize {
    length.max(2).next_power_of_two().trailing_zeros() as usize
}

/// The last level each value is read at, and the level the output reads.
///
/// Only values some check depends on get a lifetime; the rest are never laid out, because Remainder
/// requires every layer to feed the output.
fn lifetimes(graph: &Graph) -> (Vec<Option<usize>>, usize) {
    let top = graph.checks.iter().map(|check| graph.depth[*check]).max().unwrap_or(1);
    let mut last: Vec<Option<usize>> = vec![None; graph.nodes.len()];
    let mut pending: Vec<NodeId> = graph.checks.clone();
    pending.push(ONE);
    for node in &pending {
        last[*node] = Some(top);
    }
    while let Some(node) = pending.pop() {
        if let Origin::Gates(pairs) = &graph.nodes[node] {
            let read_at = graph.depth[node] - 1;
            for operand in pairs.iter().flat_map(|(x, y)| [*x, *y]) {
                let known = last[operand];
                last[operand] = Some(known.map_or(read_at, |level| level.max(read_at)));
                if known.is_none() {
                    pending.push(operand);
                }
            }
        }
    }
    (last, top)
}

/// The values in the slots of `level`, in slot order.
///
/// Level 1 holds every input whether or not a check reads it, so the public and committed tables
/// always have the same shape: one, the constants, the public inputs, then the committed entries
/// last, which keeps the highest slot free of public values. Later levels put one first and the
/// rest in creation order.
fn present(graph: &Graph, last: &[Option<usize>], level: usize) -> Vec<NodeId> {
    let alive = |node: &NodeId| {
        last[*node].is_some_and(|until| graph.depth[*node] <= level && level <= until)
    };
    if level > 1 {
        return (0..graph.nodes.len()).filter(alive).collect();
    }
    let rank = |node: &NodeId| match graph.nodes[*node] {
        Origin::One => Some((0, 0)),
        Origin::Constant(_) => Some((1, *node)),
        Origin::Public(index) => Some((2, index)),
        Origin::Committed(index) => Some((3, index)),
        Origin::Gates(_) => None,
    };
    let mut inputs: Vec<(usize, usize, NodeId)> = (0..graph.nodes.len())
        .filter(|node| !matches!(graph.nodes[*node], Origin::Constant(_)) || alive(node))
        .filter_map(|node| rank(&node).map(|(group, order)| (group, order, node)))
        .collect();
    inputs.sort_unstable();
    inputs.into_iter().map(|(_, _, node)| node).collect()
}

fn slot_map(nodes: &[NodeId]) -> HashMap<NodeId, u32> {
    (0u32..).zip(nodes).map(|(slot, node)| (*node, slot)).collect()
}

/// Level 1's committed half: `committed[i] · public[one]` into each committed value's slot.
///
/// Level 1 is the public table plus these gates, so both must have the same length; Remainder sizes
/// a gate layer by its highest output slot, so when the committed values do not reach the top slot,
/// one more gate writes `committed[0] · public[top]` there, and the public table holds zero at the top.
fn lift(graph: &Graph, level_one: &HashMap<NodeId, u32>, public_bits: usize) -> Vec<Gate> {
    let committed =
        (0..graph.nodes.len()).filter(|node| matches!(graph.nodes[*node], Origin::Committed(_)));
    // Committed nodes are created in entry order, so the i-th one is entry i.
    let mut gates: Vec<Gate> = (0u32..)
        .zip(committed)
        .map(|(entry, node)| (level_one[&node], entry, level_one[&ONE]))
        .collect();
    let top = (1u32 << public_bits) - 1;
    if gates.iter().all(|gate| gate.0 != top) {
        gates.push((top, 0, top));
    }
    gates
}

/// Gates computing level `level + 1` from level `level`: each new value's own gates, one copy for
/// every value carried up.
fn step(
    graph: &Graph,
    levels: &[Vec<NodeId>],
    slots: &[HashMap<NodeId, u32>],
    level: usize,
) -> Vec<Gate> {
    let (below, above) = (&slots[level - 1], &slots[level]);
    let mut gates = Vec::new();
    for node in &levels[level] {
        let output = above[node];
        match &graph.nodes[*node] {
            Origin::Gates(pairs) if graph.depth[*node] == level + 1 => {
                gates.extend(pairs.iter().map(|(x, y)| (output, below[x], below[y])));
            }
            _ => gates.push((output, below[node], below[&ONE])),
        }
    }
    gates
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use shared_types::Fr;
    use shared_types::halo2curves::ff::Field as _;
    use zk_circuit::lower::r1cs::R1cs;
    use zk_examples::{ExampleId, InstanceKind};

    use super::{Gate, Layout};
    use crate::field::Bn254Scalar;
    use crate::graph::Graph;

    /// What one gate layer computes: `out[z] = Σ left[x] · right[y]`, sized by its highest slot. An
    /// operand outside the layer below panics, so a gate reading any other layer cannot pass.
    fn apply(gates: &[Gate], left: &[Fr], right: &[Fr]) -> Vec<Fr> {
        let top = gates.iter().map(|gate| gate.0 as usize).max().unwrap();
        let mut out = vec![Fr::ZERO; (top + 1).next_power_of_two()];
        for (z, x, y) in gates {
            out[*z as usize] += left[*x as usize] * right[*y as usize];
        }
        out
    }

    /// The layered circuit evaluated in plain Rust from the two tables, as Remainder would.
    fn outputs(layout: &Layout, public: &[Fr], committed: &[Fr]) -> Vec<Fr> {
        let lifted = apply(&layout.lift, committed, public);
        assert_eq!(lifted.len(), public.len());
        let mut level: Vec<Fr> = public.iter().zip(&lifted).map(|(p, l)| *p + l).collect();
        for gates in &layout.steps {
            level = apply(gates, &level, &level);
        }
        apply(&layout.output, &level, &level)
    }

    /// Each check is zero exactly when its assertion holds: as many non-zero outputs as the circuit
    /// layer reports violated assertions, for true and false claims of every statement.
    #[test]
    fn the_layers_zero_one_output_per_assertion_exactly_when_it_holds() {
        for example in ExampleId::ALL {
            let circuit = example.circuit::<Bn254Scalar>().unwrap();
            let r1cs = R1cs::from_circuit(&circuit);
            let graph = Graph::from_r1cs(&r1cs).unwrap();
            let layout = Layout::of(&graph).unwrap();
            assert_eq!(graph.checks.len(), circuit.assertion_labels().len(), "{example:?}");
            for kind in [InstanceKind::Honest, InstanceKind::Dishonest] {
                let assignment = example.instance::<Bn254Scalar>(kind, &[7; 32]);
                let evaluation = circuit.evaluate_unchecked(&assignment).unwrap();
                let values: Vec<Fr> =
                    r1cs.assignment(&evaluation.values).iter().map(|v| v.0).collect();
                let public: Vec<Fr> = assignment.public.iter().map(|v| v.0).collect();
                let out = outputs(
                    &layout,
                    &layout.public_table(&graph, &public),
                    &layout.committed_table(&graph, &values),
                );
                let failing = out.iter().take(graph.checks.len()).filter(|v| **v != Fr::ZERO);
                assert_eq!(failing.count(), evaluation.violated.len(), "{example:?} {kind:?}");
                assert!(out.iter().skip(graph.checks.len().max(2)).all(|v| *v == Fr::ZERO));
            }
        }
    }
}
