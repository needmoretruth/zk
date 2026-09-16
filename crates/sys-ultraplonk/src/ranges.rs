//! Finding the museum's bit-decomposition range checks, so UltraPlonk can prove them with its range table.
//!
//! `zk_circuit::gadgets::range_check` proves `0 ≤ v < 2^n` with a hint that splits `v` into `n` bits,
//! a booleanity product and assertion per bit, and one assertion that the bits recompose to `v`.
//! UltraPlonk's point is that a table lookup does this job without the bits. The pattern is recognised
//! on the gates themselves, not on labels, and only where replacing it keeps the statement the same:
//! every bit and every booleanity product must be read by exactly the gates of the pattern and
//! nothing else. Then the bits are existentially quantified helpers seen by no other constraint, and
//! "some bits in {0, 1} recompose to `v`" says exactly `v ∈ [0, 2^n)` (no wrap-around, since `n` is
//! below the 254-bit modulus), which is what jellyfish's `enforce_in_range(v, n)` enforces. Anything
//! else is left as bits and translated gate by gate.

use zk_circuit::{Circuit, Gate, Hint, LinearCombination, Wire, ZkField};

use crate::field::Fr;

/// Bits of jellyfish's range table: every UltraPlonk circuit here carries the table `0..256`.
///
/// jellyfish's `range_gate_with_lookup` looks up each full chunk of this many bits in the table and
/// bit-decomposes a shorter top chunk. The table's rows are part of the evaluation domain, so a
/// wider table (Aztec's barretenberg used 14 bits) would make even `one-plus-one` pay for 2^14 rows.
pub const RANGE_BIT_LEN: usize = 8;

/// The range checks found in one circuit and the gates their lookups make redundant.
#[derive(Clone, Debug)]
pub(crate) struct RangePlan {
    /// Bit width of the check whose hint sits at each gate index, `None` elsewhere.
    widths: Vec<Option<u32>>,
    /// Whether each gate is a booleanity product, its assertion or a recomposition being replaced.
    replaced: Vec<bool>,
}

impl RangePlan {
    /// Scans `circuit` for range checks that can be replaced without changing the statement.
    pub(crate) fn find(circuit: &Circuit<Fr>) -> Self {
        let gates = circuit.gates();
        let readers = readers(circuit);
        let mut plan = Self { widths: vec![None; gates.len()], replaced: vec![false; gates.len()] };
        for (index, gate) in gates.iter().enumerate() {
            let Gate::Hint { outputs, kind: Hint::Bits(width), input } = gate else { continue };
            let usable = *width > 0 && *width < Fr::MODULUS_BITS;
            if let Some(covered) =
                usable.then(|| matched(gates, &readers, outputs, input)).flatten()
            {
                plan.widths[index] = Some(*width);
                for gate in covered {
                    plan.replaced[gate] = true;
                }
            }
        }
        plan
    }

    /// The bit width to range-check at gate `index`, if a replaced check's hint sits there.
    pub(crate) fn width_at(&self, index: usize) -> Option<u32> {
        self.widths[index]
    }

    /// Whether gate `index` belongs to a replaced check and emits nothing.
    pub(crate) fn is_replaced(&self, index: usize) -> bool {
        self.replaced[index]
    }

    /// How many range checks became table lookups.
    pub(crate) fn checks(&self) -> u64 {
        self.widths.iter().flatten().count() as u64
    }

    /// How many table lookups those checks take: one per full [`RANGE_BIT_LEN`]-bit chunk, the count
    /// jellyfish's `range_gate_with_lookup` adds for a width.
    pub(crate) fn lookups(&self) -> u64 {
        self.widths.iter().flatten().map(|width| u64::from(*width) / RANGE_BIT_LEN as u64).sum()
    }
}

/// For every wire, the gates that read it with a non-zero coefficient, each gate once.
fn readers(circuit: &Circuit<Fr>) -> Vec<Vec<usize>> {
    let mut readers = vec![Vec::new(); circuit.num_wires()];
    for (index, gate) in circuit.gates().iter().enumerate() {
        for lc in operands(gate) {
            for (wire, _) in lc.normalized().terms() {
                let list: &mut Vec<usize> = &mut readers[wire.index()];
                if list.last() != Some(&index) {
                    list.push(index);
                }
            }
        }
    }
    readers
}

/// The linear combinations a gate reads.
fn operands(gate: &Gate<Fr>) -> Vec<&LinearCombination<Fr>> {
    match gate {
        Gate::Constant { .. } | Gate::Input { .. } => Vec::new(),
        Gate::Linear { lc, .. } | Gate::AssertZero { lc, .. } => vec![lc],
        Gate::Mul { left, right, .. } => vec![left, right],
        Gate::Hint { input, .. } => vec![input],
    }
}

/// The gates of a range-check pattern over `bits`, or `None` if anything else reads its wires.
fn matched(
    gates: &[Gate<Fr>],
    readers: &[Vec<usize>],
    bits: &[Wire],
    input: &LinearCombination<Fr>,
) -> Option<Vec<usize>> {
    let mut covered = Vec::with_capacity(2 * bits.len() + 1);
    let mut recomposition = None;
    for bit in bits {
        let [first, second] = readers[bit.index()].as_slice() else { return None };
        let (product_gate, sum_gate) =
            if is_assert(&gates[*first]) { (*second, *first) } else { (*first, *second) };
        if recomposition.is_some_and(|gate| gate != sum_gate) {
            return None;
        }
        recomposition = Some(sum_gate);
        let product = booleanity_output(&gates[product_gate], *bit)?;
        let [assertion] = readers[product.index()].as_slice() else { return None };
        if !asserts_only(&gates[*assertion], product) {
            return None;
        }
        covered.extend([product_gate, *assertion]);
    }
    let recomposition = recomposition?;
    let Gate::AssertZero { lc, .. } = &gates[recomposition] else { return None };
    let mut expected = input.clone();
    let mut weight = Fr::one();
    for bit in bits {
        expected = expected.add_term(*bit, weight.neg());
        weight = weight.add(weight);
    }
    (lc.normalized() == expected.normalized()).then(|| {
        covered.push(recomposition);
        covered
    })
}

fn is_assert(gate: &Gate<Fr>) -> bool {
    matches!(gate, Gate::AssertZero { .. })
}

/// The output of `bit · (bit − 1)` (in either factor order), the booleanity product.
fn booleanity_output(gate: &Gate<Fr>, bit: Wire) -> Option<Wire> {
    let Gate::Mul { output, left, right } = gate else { return None };
    let plain = LinearCombination::<Fr>::from(bit);
    let shifted = plain.clone().add_constant(Fr::one().neg());
    let (left, right) = (left.normalized(), right.normalized());
    ((left == plain && right == shifted) || (left == shifted && right == plain)).then_some(*output)
}

/// Whether `gate` asserts `k · product = 0` and nothing more.
fn asserts_only(gate: &Gate<Fr>, product: Wire) -> bool {
    let Gate::AssertZero { lc, .. } = gate else { return false };
    let lc = lc.normalized();
    lc.constant_term() == Fr::zero() && matches!(lc.terms(), [(wire, _)] if *wire == product)
}
