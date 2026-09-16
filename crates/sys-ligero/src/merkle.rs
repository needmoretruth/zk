//! The commitment: a SHA-256 Merkle tree over the `n` columns, each leaf salted.
//!
//! §5.1 commits to every entry with a statistically hiding commitment and compresses the
//! commitments with a Merkle tree. Here the hiding comes from a fresh 32-byte salt per column,
//! so an unopened column's leaf says nothing about its entries.
//!
//! - `leaf_j = SHA-256("zk/ligero/v1/leaf" ‖ salt_j ‖ entry…)`, the column's entries in row order;
//! - `node = SHA-256("zk/ligero/v1/node" ‖ left ‖ right)`; `n` is a power of two, so the tree is full.
//!
//! An opening of the columns `Q` (ascending) carries only the siblings the verifier cannot compute:
//! level by level from the leaves, left to right, every known node whose sibling is not also known
//! takes that sibling from the list. How many siblings there are depends on `Q` alone.

use p3_goldilocks::Goldilocks;
use sha2::{Digest, Sha256};

use crate::hash::{Bytes32, DOMAIN_LEAF, DOMAIN_NODE, update_elements};

/// The hash of one salted column.
pub(crate) fn leaf(salt: &Bytes32, entries: &[Goldilocks]) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_LEAF);
    hasher.update(salt);
    update_elements(&mut hasher, entries.iter().copied());
    hasher.finalize().into()
}

fn node(left: &Bytes32, right: &Bytes32) -> Bytes32 {
    Sha256::new_with_prefix(DOMAIN_NODE).chain_update(left).chain_update(right).finalize().into()
}

/// Hashes the known nodes `(index, hash)` (ascending) up `levels` levels, asking `sibling` for every
/// missing one in the documented order; `None` when `sibling` has none left or nothing is known.
fn fold<S>(mut known: Vec<(usize, Bytes32)>, levels: usize, mut sibling: S) -> Option<Bytes32>
where
    S: FnMut(usize, usize) -> Option<Bytes32>,
{
    for level in 0..levels {
        let mut parents = Vec::with_capacity(known.len());
        let mut i = 0;
        while i < known.len() {
            let (index, hash) = known[i];
            let (left, right) = if index % 2 == 1 {
                (sibling(level, index - 1)?, hash)
            } else if let Some(&(_, right)) = known.get(i + 1).filter(|(n, _)| *n == index + 1) {
                i += 1;
                (hash, right)
            } else {
                (hash, sibling(level, index + 1)?)
            };
            parents.push((index / 2, node(&left, &right)));
            i += 1;
        }
        known = parents;
    }
    match known.as_slice() {
        [(0, root)] => Some(*root),
        _ => None,
    }
}

/// Every level of the tree, leaves first.
pub(crate) struct Tree {
    levels: Vec<Vec<Bytes32>>,
}

impl Tree {
    /// Builds the tree over `leaves`, whose count is a power of two.
    pub(crate) fn new(leaves: Vec<Bytes32>) -> Self {
        let mut levels = vec![leaves];
        while let Some(top) = levels.last().filter(|level| level.len() > 1) {
            let parents =
                top.as_chunks::<2>().0.iter().map(|[left, right]| node(left, right)).collect();
            levels.push(parents);
        }
        Self { levels }
    }

    /// The commitment.
    pub(crate) fn root(&self) -> Bytes32 {
        self.levels.last().and_then(|top| top.first()).copied().unwrap_or_default()
    }

    /// The siblings an opening of `opened` (ascending, distinct) carries, in order.
    pub(crate) fn siblings(&self, opened: &[usize]) -> Vec<Bytes32> {
        let known = opened.iter().map(|j| (*j, self.levels[0][*j])).collect();
        let mut siblings = Vec::new();
        // The root this recomputes is the tree's own; only the siblings it asked for matter here.
        let _ = fold(known, self.levels.len() - 1, |level, index| {
            let hash = self.levels[level][index];
            siblings.push(hash);
            Some(hash)
        });
        siblings
    }
}

/// Whether the opened leaves `(column, leaf)` (ascending) and `siblings` hash to `root` in a tree of
/// `length` leaves, using every sibling exactly once.
pub(crate) fn opens_to(
    root: &Bytes32,
    length: usize,
    opened: Vec<(usize, Bytes32)>,
    siblings: &[Bytes32],
) -> bool {
    let mut supply = siblings.iter();
    let levels = length.trailing_zeros() as usize;
    let computed = fold(opened, levels, |_, _| supply.next().copied());
    computed.as_ref() == Some(root) && supply.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(length: usize) -> Tree {
        Tree::new((0..length).map(|j| leaf(&[j as u8; 32], &[])).collect())
    }

    fn leaves(tree: &Tree, opened: &[usize]) -> Vec<(usize, Bytes32)> {
        opened.iter().map(|j| (*j, tree.levels[0][*j])).collect()
    }

    #[test]
    fn an_opening_hashes_to_the_root_with_exactly_its_siblings() {
        let tree = tree(64);
        for opened in [vec![0], vec![63], vec![0, 1, 2, 3], vec![5, 6, 17, 40, 41, 63]] {
            let siblings = tree.siblings(&opened);
            assert!(opens_to(&tree.root(), 64, leaves(&tree, &opened), &siblings), "{opened:?}");
            let mut extra = siblings.clone();
            extra.push([0; 32]);
            assert!(!opens_to(&tree.root(), 64, leaves(&tree, &opened), &extra));
            if let Some((_, fewer)) = siblings.split_last() {
                assert!(!opens_to(&tree.root(), 64, leaves(&tree, &opened), fewer));
            }
        }
        // Opening everything needs no sibling; one leaf needs one per level.
        assert!(tree.siblings(&(0..64).collect::<Vec<_>>()).is_empty());
        assert_eq!(tree.siblings(&[9]).len(), 6);
    }
}
