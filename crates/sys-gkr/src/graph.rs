//! The museum's R1CS rewritten as values a layered circuit of multiplication gates computes.
//!
//! A GKR layer is a set of gates, each reading two values of the layer below. Remainder's gate layer
//! sums `lhs[x] · rhs[y]` over every gate wired to an output slot and has no coefficients, so every
//! value here is such a sum of products. A coefficient `c` becomes a product with a slot holding `c`,
//! and a constant term a product with the slot holding one; the verifier supplies both with the
//! public inputs. The R1CS lowering has already inlined additions into its rows, so a row
//! `⟨a, z⟩ · ⟨b, z⟩ = ⟨c, z⟩` costs at most two layers: one gathering each factor into a single value,
//! one multiplying them.
//!
//! A value used with a factor `s` is not recomputed with `s` folded in: it is carried as the pair
//! (value, `s`) until a sum needs it, where `s` becomes that gate's coefficient. Checks drop a
//! non-zero factor, since `s · v = 0` exactly when `v = 0`.

use std::collections::{BTreeMap, HashMap, HashSet};

use shared_types::Fr;
use shared_types::halo2curves::ff::Field as _;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};

use crate::field::Bn254Scalar;

/// Position of a value in [`Graph::nodes`].
pub(crate) type NodeId = usize;

/// The value one: the first node of every graph.
pub(crate) const ONE: NodeId = 0;

/// Where a value comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The constant one, supplied by the verifier.
    One,
    /// A coefficient or constant term other than one, supplied by the verifier.
    Constant(Fr),
    /// Public input `i` in declaration order.
    Public(usize),
    /// Entry `i` of the committed table: a private input or a hint output.
    Committed(usize),
    /// `Σ x · y` over the pairs, computed by gates reading the layer below.
    Gates(Vec<(NodeId, NodeId)>),
}

/// A node times a factor the layered circuit has not applied yet.
#[derive(Clone, Copy, Debug)]
struct Scaled {
    node: NodeId,
    scale: Fr,
}

/// A linear combination of nodes, merged by node, without zero coefficients.
struct Linear {
    terms: BTreeMap<NodeId, Fr>,
    constant: Fr,
}

/// Every value the layered circuit computes, with the level at which each first exists.
pub(crate) struct Graph {
    /// Values in creation order; inputs first.
    pub(crate) nodes: Vec<Origin>,
    /// Level of each node: 1 for inputs and constants, one more than its deepest operand otherwise.
    pub(crate) depth: Vec<usize>,
    /// Nodes that must be zero, one per non-trivial R1CS check, in row order.
    pub(crate) checks: Vec<NodeId>,
    /// The R1CS variable behind each committed entry: private inputs, then hint outputs.
    pub(crate) committed_variables: Vec<usize>,
    /// Number of public inputs.
    pub(crate) num_public: usize,
    constants: HashMap<Fr, NodeId>,
    computed: HashMap<Vec<(NodeId, NodeId)>, NodeId>,
}

impl Graph {
    /// Rewrites every row of `r1cs`. A row whose product side is one fresh variable defines that
    /// variable; every other row, assert-zero rows included, becomes a check that `a · b − c` is zero.
    pub(crate) fn from_r1cs(r1cs: &R1cs<Bn254Scalar>) -> Result<Self, String> {
        let num_public = r1cs.num_public_inputs();
        let mut graph = Self {
            nodes: vec![Origin::One],
            depth: vec![1],
            checks: Vec::new(),
            committed_variables: Vec::new(),
            num_public,
            constants: HashMap::new(),
            computed: HashMap::new(),
        };
        let mut values: Vec<Option<Scaled>> = vec![None; r1cs.num_variables()];
        values[0] = Some(Scaled { node: ONE, scale: Fr::ONE });
        for (index, slot) in values.iter_mut().enumerate().skip(1).take(num_public) {
            *slot = Some(unit(graph.input(Origin::Public(index - 1))));
        }
        let products = defined_by_rows(r1cs);
        for (variable, slot) in values.iter_mut().enumerate().skip(1 + num_public) {
            if !products.contains(&variable) {
                let entry = graph.committed_variables.len();
                graph.committed_variables.push(variable);
                *slot = Some(unit(graph.input(Origin::Committed(entry))));
            }
        }
        for row in r1cs.constraints() {
            let left = graph.reduce(&row.a, &values)?;
            let right = graph.reduce(&row.b, &values)?;
            let left = graph.materialize(left);
            let right = graph.materialize(right);
            let product = graph.multiply(left, right);
            match row.c.as_slice() {
                [(variable, one)] if one.0 == Fr::ONE && values[*variable].is_none() => {
                    values[*variable] = Some(product);
                }
                _ => {
                    let mut difference = graph.reduce(&row.c, &values)?;
                    difference.negate();
                    difference.add(product);
                    let difference = graph.materialize(difference);
                    graph.check(difference);
                }
            }
        }
        Ok(graph)
    }

    fn input(&mut self, origin: Origin) -> NodeId {
        self.nodes.push(origin);
        self.depth.push(1);
        self.nodes.len() - 1
    }

    /// The node holding `value`: one itself, or a verifier-supplied constant made once per value.
    fn constant(&mut self, value: Fr) -> NodeId {
        if value == Fr::ONE {
            return ONE;
        }
        if let Some(node) = self.constants.get(&value) {
            return *node;
        }
        let node = self.input(Origin::Constant(value));
        self.constants.insert(value, node);
        node
    }

    /// The node `Σ x · y`, made once per set of pairs.
    fn gates(&mut self, mut pairs: Vec<(NodeId, NodeId)>) -> NodeId {
        for pair in &mut pairs {
            *pair = (pair.0.min(pair.1), pair.0.max(pair.1));
        }
        pairs.sort_unstable();
        if let Some(node) = self.computed.get(&pairs) {
            return *node;
        }
        let deepest = pairs.iter().map(|(x, y)| self.depth[*x].max(self.depth[*y])).max();
        self.nodes.push(Origin::Gates(pairs.clone()));
        self.depth.push(deepest.unwrap_or(0) + 1);
        let node = self.nodes.len() - 1;
        self.computed.insert(pairs, node);
        node
    }

    fn reduce(
        &self,
        lc: &SparseLc<Bn254Scalar>,
        values: &[Option<Scaled>],
    ) -> Result<Linear, String> {
        let mut linear = Linear { terms: BTreeMap::new(), constant: Fr::ZERO };
        for (variable, coefficient) in lc {
            let value =
                values.get(*variable).copied().flatten().ok_or_else(|| {
                    format!("R1CS variable {variable} is read before it is defined")
                })?;
            linear.add(Scaled { node: value.node, scale: value.scale * coefficient.0 });
        }
        Ok(linear)
    }

    /// One node (with a factor) for a linear combination: no gate when it already is one node.
    fn materialize(&mut self, linear: Linear) -> Scaled {
        let mut terms = linear.terms.into_iter();
        match (terms.len(), linear.constant == Fr::ZERO) {
            (0, _) => Scaled { node: ONE, scale: linear.constant },
            (1, true) => match terms.next() {
                Some((node, scale)) => Scaled { node, scale },
                None => Scaled { node: ONE, scale: Fr::ZERO },
            },
            _ => {
                let mut pairs = Vec::with_capacity(terms.len() + 1);
                for (node, coefficient) in terms {
                    pairs.push((node, self.constant(coefficient)));
                }
                if linear.constant != Fr::ZERO {
                    pairs.push((ONE, self.constant(linear.constant)));
                }
                unit(self.gates(pairs))
            }
        }
    }

    /// A product; free when either side is a multiple of one.
    fn multiply(&mut self, left: Scaled, right: Scaled) -> Scaled {
        let scale = left.scale * right.scale;
        if scale == Fr::ZERO {
            return Scaled { node: ONE, scale };
        }
        let node = match (left.node, right.node) {
            (ONE, other) | (other, ONE) => other,
            (x, y) => self.gates(vec![(x, y)]),
        };
        Scaled { node, scale }
    }

    /// Requires `value` to be zero; `0 = 0` needs no check.
    fn check(&mut self, value: Scaled) {
        if value.scale != Fr::ZERO {
            self.checks.push(value.node);
        }
    }
}

impl Linear {
    fn add(&mut self, value: Scaled) {
        let slot = if value.node == ONE {
            &mut self.constant
        } else {
            self.terms.entry(value.node).or_insert(Fr::ZERO)
        };
        *slot += value.scale;
        if value.node != ONE && *slot == Fr::ZERO {
            self.terms.remove(&value.node);
        }
    }

    fn negate(&mut self) {
        self.constant = -self.constant;
        for coefficient in self.terms.values_mut() {
            *coefficient = -*coefficient;
        }
    }
}

/// A node with nothing left to apply.
fn unit(node: NodeId) -> Scaled {
    Scaled { node, scale: Fr::ONE }
}

/// Variables some row defines as its product: the product side of a multiplication row.
fn defined_by_rows(r1cs: &R1cs<Bn254Scalar>) -> HashSet<usize> {
    r1cs.constraints()
        .iter()
        .filter_map(|row| match row.c.as_slice() {
            [(variable, one)] if one.0 == Fr::ONE => Some(*variable),
            _ => None,
        })
        .collect()
}
