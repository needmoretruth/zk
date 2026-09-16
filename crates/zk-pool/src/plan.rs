//! A spend's values, turned into the `pool-spend` assignment in the circuit's declaration order.

use zk_circuit::{Assignment, ZkField};
use zk_examples::merkle_tree::MerkleTree;
use zk_examples::pool::{TREE_DEPTH, address, note_commitment, nullifier};

use crate::error::PoolError;

/// The note a spend consumes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Input<F> {
    /// A zero-value note under a throwaway key: the circuit skips the tree check for it.
    Dummy {
        /// The throwaway spending key.
        sk: F,
        /// The throwaway note randomness.
        rcm: F,
    },
    /// A note at `position`, spent with `sk` (which is the owner's key unless this is a theft).
    Note {
        /// The spending key the prover uses.
        sk: F,
        /// The note's value.
        value: F,
        /// The note's randomness.
        rcm: F,
        /// Its tree position.
        position: usize,
    },
}

/// One output note's opening.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Output<F> {
    /// The recipient's address.
    pub(crate) pk: F,
    /// Its value, which a counterfeit makes "negative".
    pub(crate) value: F,
    /// Fresh randomness.
    pub(crate) rcm: F,
}

/// Every value a spend proof needs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan<F> {
    /// The consumed note.
    pub(crate) input: Input<F>,
    /// The two created notes.
    pub(crate) outputs: [Output<F>; 2],
    /// Public value entering the pool.
    pub(crate) v_pub_in: u16,
    /// Public value leaving the pool.
    pub(crate) v_pub_out: u16,
    /// A nullifier to publish instead of the true one; only the unbound-nullifier attack sets it.
    pub(crate) forged_nullifier: Option<F>,
}

impl<F: ZkField> Plan<F> {
    /// The assignment against the tree's current root, in `pool-spend`'s input order:
    /// public `root, nullifier, cm_out_1, cm_out_2, v_pub_in, v_pub_out`; private `sk, v_in, rcm_in,
    /// path_bit_0..7, sibling_0..7, pk_out_1, v_out_1, rcm_out_1, pk_out_2, v_out_2, rcm_out_2`.
    pub(crate) fn assignment(&self, tree: &MerkleTree<F>) -> Result<Assignment<F>, PoolError> {
        let (sk, v_in, rcm_in, bits, siblings) = match self.input {
            Input::Dummy { sk, rcm } => {
                (sk, F::zero(), rcm, vec![F::zero(); TREE_DEPTH], vec![F::zero(); TREE_DEPTH])
            }
            Input::Note { sk, value, rcm, position } => {
                let path = tree.path(position).ok_or_else(|| {
                    PoolError::Corrupt(format!("note position {position} is outside the tree"))
                })?;
                let bits = path.bits.iter().map(|bit| if *bit { F::one() } else { F::zero() });
                (sk, value, rcm, bits.collect(), path.siblings)
            }
        };
        let cm_in = note_commitment(address(sk), v_in, rcm_in);
        let nf = self.forged_nullifier.unwrap_or_else(|| nullifier(sk, cm_in));
        let [first, second] = self.outputs.map(|o| note_commitment(o.pk, o.value, o.rcm));
        let public = vec![
            tree.root(),
            nf,
            first,
            second,
            F::from_u64(u64::from(self.v_pub_in)),
            F::from_u64(u64::from(self.v_pub_out)),
        ];
        let outputs = self.outputs.iter().flat_map(|o| [o.pk, o.value, o.rcm]);
        let private =
            [sk, v_in, rcm_in].into_iter().chain(bits).chain(siblings).chain(outputs).collect();
        Ok(Assignment { public, private })
    }
}
