//! `membership`: I am one of the 16 members, and this is my one vote, without saying who I am.
//!
//! Anonymous voting. Each member's leaf is `ToyHash(secret, 0)` in a depth-4 tree; the voter proves
//! knowledge of a secret whose leaf is under the public root, and publishes the nullifier
//! `ToyHash(secret, 1)`. A second vote by the same member repeats the nullifier and is refused.

use zk_circuit::gadgets::merkle_root;
use zk_circuit::toyhash::{toyhash, toyhash_gadget};
use zk_circuit::{
    Assignment, Circuit, CircuitBuilder, CircuitError, LinearCombination, Wire, ZkField,
};

use crate::merkle_tree::MerkleTree;

/// Permanent ID.
pub const ID: &str = "membership";

/// Tree depth: 16 members.
pub const DEPTH: usize = 4;

const VOTER_INDEX: usize = 9;
const OUTSIDER_SECRET: u64 = 123_456;

/// `root`, `nullifier`.
pub fn public_input_names() -> Vec<String> {
    crate::names(&["root", "nullifier"])
}

/// `secret`, `path_bit_0..3`, `sibling_0..3`.
pub fn private_input_names() -> Vec<String> {
    let bits = (0..DEPTH).map(|i| format!("path_bit_{i}"));
    let siblings = (0..DEPTH).map(|i| format!("sibling_{i}"));
    core::iter::once("secret".to_string()).chain(bits).chain(siblings).collect()
}

/// Asserts the leaf `ToyHash(secret, 0)` is under `root` and `nullifier = ToyHash(secret, 1)`.
pub fn circuit<F: ZkField>() -> Result<Circuit<F>, CircuitError> {
    let mut builder = CircuitBuilder::<F>::new()?;
    let root = builder.public_input("root");
    let nullifier = builder.public_input("nullifier");
    let names = private_input_names();
    let private: Vec<Wire> = names.into_iter().map(|name| builder.private_input(name)).collect();
    let (secret, path) = (private[0], &private[1..]);
    let (bits, siblings) = path.split_at(DEPTH);
    let zero = builder.constant(F::zero());
    let one = builder.constant(F::one());
    let leaf = toyhash_gadget(&mut builder, secret, zero);
    let computed = merkle_root(&mut builder, leaf, bits, siblings, "member leaf")?;
    builder.assert_zero(
        LinearCombination::<F>::from(computed) - root,
        "member leaf is under the root",
    );
    let expected = toyhash_gadget(&mut builder, secret, one);
    builder.assert_zero(
        LinearCombination::<F>::from(expected) - nullifier,
        "nullifier belongs to the secret",
    );
    builder.finish()
}

/// The secret of member `index`.
fn member_secret<F: ZkField>(index: usize) -> F {
    F::from_u64(900_001 + 7_919 * index as u64)
}

/// Leaf of a member: `ToyHash(secret, 0)`.
pub fn leaf<F: ZkField>(secret: F) -> F {
    toyhash(secret, F::zero())
}

/// Nullifier of a member: `ToyHash(secret, 1)`.
pub fn nullifier<F: ZkField>(secret: F) -> F {
    toyhash(secret, F::one())
}

fn members<F: ZkField>() -> MerkleTree<F> {
    let mut tree = MerkleTree::new(DEPTH);
    for index in 0..tree.capacity() {
        tree.push(leaf(member_secret::<F>(index)));
    }
    tree
}

/// A witness for `secret` claiming the position of member [`VOTER_INDEX`] in the member tree.
fn vote<F: ZkField>(secret: F) -> Assignment<F> {
    let tree = members::<F>();
    #[allow(clippy::expect_used, reason = "VOTER_INDEX is a constant below the tree's capacity")]
    let path = tree.path(VOTER_INDEX).expect("voter index fits the tree");
    let bits = path.bits.iter().map(|bit| if *bit { F::one() } else { F::zero() });
    let private = core::iter::once(secret).chain(bits).chain(path.siblings).collect();
    Assignment { public: vec![tree.root(), nullifier(secret)], private }
}

/// Member 9 votes.
pub fn honest<F: ZkField>() -> Assignment<F> {
    vote(member_secret(VOTER_INDEX))
}

/// A secret that is in no leaf, presented with member 9's path and the real root.
pub fn dishonest<F: ZkField>() -> Assignment<F> {
    vote(F::from_u64(OUTSIDER_SECRET))
}
