use zk_circuit::gadgets::merkle_root_native;
use zk_circuit::{EvalError, ZkField};
use zk_examples::merkle_tree::MerkleTree;
use zk_examples::pool::spend::{RangeChecks, circuit, circuit_with, sample_spend};

use crate::fields::BabyBear;

type F = BabyBear;

#[test]
fn a_negative_output_counterfeits_value_only_without_range_checks() {
    // 100 in, 105 + (−5) out: the balance equation holds in the field.
    let counterfeit = sample_spend([F::from_u64(105), F::from_u64(5).neg()]);
    let broken = circuit_with::<F>(RangeChecks::Omitted).unwrap();
    assert_eq!(broken.evaluate(&counterfeit).map(|_| ()), Ok(()));
    let label = "v_out_2 fits in 16 bits".to_string();
    assert_eq!(
        circuit::<F>().unwrap().evaluate(&counterfeit).map(|_| ()),
        Err(EvalError::AssertionFailed { label })
    );
}

#[test]
fn every_tree_path_leads_to_the_root_and_a_full_tree_refuses_more_leaves() {
    let mut tree = MerkleTree::<F>::new(3);
    for leaf in 0..5 {
        assert_eq!(tree.push(F::from_u64(leaf * 11 + 1)), Some(leaf as usize));
    }
    for index in 0..tree.capacity() {
        let path = tree.path(index).unwrap();
        let leaf = tree.leaf(index).unwrap();
        assert_eq!(merkle_root_native(leaf, &path.bits, &path.siblings).unwrap(), tree.root());
    }
    assert_eq!(tree.path(tree.capacity()), None);
    while tree.len() < tree.capacity() {
        tree.push(F::one());
    }
    assert_eq!(tree.push(F::one()), None);
}
