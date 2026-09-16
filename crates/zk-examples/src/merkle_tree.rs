//! A fixed-depth ToyHash Merkle tree with membership paths.
//!
//! The membership example and the Toy Shielded Pool ledger both need a tree built outside the
//! circuit whose paths feed [`zk_circuit::gadgets::merkle_root`]. The path convention is the
//! gadget's: level 0 is the leaves, and bit `i` of a leaf's index says whether its ancestor at level
//! `i` is a right child.

use zk_circuit::ZkField;
use zk_circuit::toyhash::toyhash;

/// The siblings and direction bits from a leaf up to the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerklePath<F> {
    /// Bit `i` of the leaf index, least significant first; `true` means a right child.
    pub bits: Vec<bool>,
    /// The sibling at each level, leaf level first.
    pub siblings: Vec<F>,
}

/// An append-only tree of `2^depth` leaves; unfilled leaves hold zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleTree<F> {
    depth: usize,
    leaves: Vec<F>,
}

impl<F: ZkField> MerkleTree<F> {
    /// An empty tree with room for `2^depth` leaves.
    pub fn new(depth: usize) -> Self {
        Self { depth, leaves: Vec::new() }
    }

    /// Number of levels between the leaves and the root.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Maximum number of leaves.
    pub fn capacity(&self) -> usize {
        1 << self.depth
    }

    /// Number of leaves appended so far.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether no leaf has been appended.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Appends a leaf and returns its index, or `None` when the tree is full.
    pub fn push(&mut self, leaf: F) -> Option<usize> {
        if self.leaves.len() == self.capacity() {
            return None;
        }
        self.leaves.push(leaf);
        Some(self.leaves.len() - 1)
    }

    /// The leaf at `index`, zero if unfilled, `None` beyond capacity.
    pub fn leaf(&self, index: usize) -> Option<F> {
        (index < self.capacity()).then(|| self.leaves.get(index).copied().unwrap_or_else(F::zero))
    }

    /// Every level from the leaves (padded with zeros) up to the single root.
    fn levels(&self) -> Vec<Vec<F>> {
        let mut level: Vec<F> = (0..self.capacity()).filter_map(|i| self.leaf(i)).collect();
        let mut levels = Vec::with_capacity(self.depth + 1);
        for _ in 0..self.depth {
            let parent = level.chunks(2).map(|pair| toyhash(pair[0], pair[1])).collect();
            levels.push(level);
            level = parent;
        }
        levels.push(level);
        levels
    }

    /// The root.
    pub fn root(&self) -> F {
        self.levels().last().and_then(|top| top.first().copied()).unwrap_or_else(F::zero)
    }

    /// The path proving the leaf at `index`, `None` beyond capacity.
    pub fn path(&self, index: usize) -> Option<MerklePath<F>> {
        if index >= self.capacity() {
            return None;
        }
        let levels = self.levels();
        let (bits, siblings) = levels[..self.depth]
            .iter()
            .enumerate()
            .map(|(level, nodes)| {
                let position = index >> level;
                (position & 1 == 1, nodes[position ^ 1])
            })
            .unzip();
        Some(MerklePath { bits, siblings })
    }
}
