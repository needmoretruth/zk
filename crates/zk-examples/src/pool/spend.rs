//! `pool-spend`: the Toy Shielded Pool's 1-input, 2-output spend.
//!
//! Combines the other six lessons: hashing, Merkle membership, a nullifier, range checks and a
//! balance equation.

use zk_circuit::gadgets::{merkle_root, range_check};
use zk_circuit::toyhash::toyhash_gadget;
use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, Wire, ZkField,
};

use crate::pool::{TREE_DEPTH, VALUE_BITS, address, new_note_tree, note_commitment, nullifier};

/// Permanent ID.
pub const ID: &str = "pool-spend";

/// Whether the circuit proves that every amount fits in 16 bits.
///
/// [`RangeChecks::Omitted`] exists only to demonstrate the counterfeiting attack: without range
/// checks, an output of "−5" (the field element `p − 5`) balances an extra 5 created from nothing.
/// It is not one of the seven examples and must never back a real ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RangeChecks {
    /// All five amounts are range-checked; the `pool-spend` example.
    Enforced,
    /// No amount is range-checked; the broken circuit for the attack demonstration.
    Omitted,
}

const PUBLIC: [&str; 6] = ["root", "nullifier", "cm_out_1", "cm_out_2", "v_pub_in", "v_pub_out"];

/// `root`, `nullifier`, `cm_out_1`, `cm_out_2`, `v_pub_in`, `v_pub_out`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&PUBLIC)
}

/// `sk`, `v_in`, `rcm_in`, `path_bit_0..7`, `sibling_0..7`, then `pk_out_j`, `v_out_j`,
/// `rcm_out_j` for `j` = 1, 2.
pub fn private_input_names() -> Vec<String> {
    let head = ["sk", "v_in", "rcm_in"].map(String::from);
    let bits = (0..TREE_DEPTH).map(|i| format!("path_bit_{i}"));
    let siblings = (0..TREE_DEPTH).map(|i| format!("sibling_{i}"));
    let outputs =
        (1..=2).flat_map(|j| [format!("pk_out_{j}"), format!("v_out_{j}"), format!("rcm_out_{j}")]);
    head.into_iter().chain(bits).chain(siblings).chain(outputs).collect()
}

/// The `pool-spend` circuit, with range checks.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    circuit_with(RangeChecks::Enforced)
}

/// Named wires of the spend's inputs.
struct Inputs {
    root: Wire,
    nullifier: Wire,
    cm_out: [Wire; 2],
    v_pub_in: Wire,
    v_pub_out: Wire,
    sk: Wire,
    v_in: Wire,
    rcm_in: Wire,
    path_bits: Vec<Wire>,
    siblings: Vec<Wire>,
    /// `(pk_out_j, v_out_j, rcm_out_j)`.
    outputs: [(Wire, Wire, Wire); 2],
}

impl Inputs {
    fn declare<F: ZkField>(builder: &mut CircuitBuilder<F>) -> Self {
        let public: Vec<Wire> =
            public_input_names().into_iter().map(|n| builder.public_input(n)).collect();
        let private: Vec<Wire> =
            private_input_names().into_iter().map(|n| builder.private_input(n)).collect();
        let path_end = 3 + 2 * TREE_DEPTH;
        let output = |j: usize| {
            (
                private[path_end + 3 * j],
                private[path_end + 3 * j + 1],
                private[path_end + 3 * j + 2],
            )
        };
        Self {
            root: public[0],
            nullifier: public[1],
            cm_out: [public[2], public[3]],
            v_pub_in: public[4],
            v_pub_out: public[5],
            sk: private[0],
            v_in: private[1],
            rcm_in: private[2],
            path_bits: private[3..3 + TREE_DEPTH].to_vec(),
            siblings: private[3 + TREE_DEPTH..path_end].to_vec(),
            outputs: [output(0), output(1)],
        }
    }
}

/// The spend circuit, optionally without range checks (see [`RangeChecks`]).
pub fn circuit_with<F: ZkField>(range_checks: RangeChecks) -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let inputs = Inputs::declare(&mut builder);
    let zero = builder.constant(F::zero());
    let pk_in = toyhash_gadget(&mut builder, inputs.sk, zero);
    let cm_in = commitment_gadget(&mut builder, pk_in, inputs.v_in, inputs.rcm_in);
    let computed_root =
        merkle_root(&mut builder, cm_in, &inputs.path_bits, &inputs.siblings, "input note")?;
    // The anchor is enforced only for a non-zero input, as Zcash Sapling does for dummy spends:
    // (computed_root − root)·v_in = 0 lets a zero-value input use any path, so a transaction can
    // shield public value without owning a note.
    let gap = builder.mul(LinearCombination::<F>::from(computed_root) - inputs.root, inputs.v_in);
    builder.assert_zero(gap, "input note is in the tree unless its value is zero");
    let nf = toyhash_gadget(&mut builder, inputs.sk, cm_in);
    builder.assert_zero(
        LinearCombination::<F>::from(nf) - inputs.nullifier,
        "nullifier belongs to the input note and key",
    );
    for (index, ((pk, value, rcm), cm_out)) in inputs.outputs.iter().zip(inputs.cm_out).enumerate()
    {
        let cm = commitment_gadget(&mut builder, *pk, *value, *rcm);
        let label = format!("output note {} commitment is correct", index + 1);
        builder.assert_zero(LinearCombination::<F>::from(cm) - cm_out, label);
    }
    let [(_, v_out_1, _), (_, v_out_2, _)] = inputs.outputs;
    let balance = LinearCombination::<F>::from(inputs.v_in) + inputs.v_pub_in
        - v_out_1
        - v_out_2
        - inputs.v_pub_out;
    builder.assert_zero(balance, "value in equals value out");
    if range_checks == RangeChecks::Enforced {
        let amounts = [
            (inputs.v_in, "v_in"),
            (v_out_1, "v_out_1"),
            (v_out_2, "v_out_2"),
            (inputs.v_pub_in, "v_pub_in"),
            (inputs.v_pub_out, "v_pub_out"),
        ];
        for (amount, name) in amounts {
            range_check(&mut builder, amount, VALUE_BITS, &format!("{name} fits in 16 bits"))?;
        }
    }
    builder.finish()
}

/// `ToyHash(ToyHash(pk, value), rcm)` as gates.
fn commitment_gadget<F: ZkField>(
    builder: &mut CircuitBuilder<F>,
    pk: Wire,
    value: Wire,
    rcm: Wire,
) -> Wire {
    let inner = toyhash_gadget(builder, pk, value);
    toyhash_gadget(builder, inner, rcm)
}

const SPENDER_SK: u64 = 7_654_321;
const INPUT_VALUE: u64 = 100;
const INPUT_RCM: u64 = 111_111;
const INPUT_INDEX: usize = 6;
const PAYEE_SK: u64 = 2_222_222;

/// A spend of the sample input note into two outputs worth `v_out`, with no public value moving.
///
/// The input is a 100-unit note owned by spending key 7654321, at index 6 of a tree holding six
/// other notes. Output 1 pays the address of key 2222222; output 2 returns change to the spender.
/// `v_out` takes field elements so the attack demonstration can pass a "negative" amount.
pub fn sample_spend<F: ZkField>(v_out: [F; 2]) -> Assignment<F> {
    let mut tree = new_note_tree::<F>();
    for index in 0..INPUT_INDEX as u64 {
        let pk = address(F::from_u64(1_000 + index));
        tree.push(note_commitment(pk, F::from_u64(10 + index), F::from_u64(5_000 + index)));
    }
    let [sk, v_in, rcm_in] = [SPENDER_SK, INPUT_VALUE, INPUT_RCM].map(F::from_u64);
    let pk_in = address(sk);
    let cm_in = note_commitment(pk_in, v_in, rcm_in);
    let index = tree.push(cm_in).unwrap_or(INPUT_INDEX);
    #[allow(clippy::expect_used, reason = "the input's index is below the tree's capacity")]
    let path = tree.path(index).expect("input index fits the tree");
    let pk_out = [address(F::from_u64(PAYEE_SK)), pk_in];
    let rcm_out = [F::from_u64(333), F::from_u64(444)];
    let cm_out = [0, 1].map(|j| note_commitment(pk_out[j], v_out[j], rcm_out[j]));
    let public =
        vec![tree.root(), nullifier(sk, cm_in), cm_out[0], cm_out[1], F::zero(), F::zero()];
    let bits = path.bits.iter().map(|bit| if *bit { F::one() } else { F::zero() });
    let outputs = (0..2).flat_map(|j| [pk_out[j], v_out[j], rcm_out[j]]);
    let private =
        [sk, v_in, rcm_in].into_iter().chain(bits).chain(path.siblings).chain(outputs).collect();
    Assignment { public, private }
}

/// Spends a 100 note into 70 to the payee and 30 change.
pub fn honest<F: ZkField>() -> Assignment<F> {
    sample_spend([F::from_u64(70), F::from_u64(30)])
}

/// Spends a 100 note into 80 and 30: outputs worth more than the input.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    sample_spend([F::from_u64(80), F::from_u64(30)])
}
