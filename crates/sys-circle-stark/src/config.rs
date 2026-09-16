//! The STARK configuration: hash, circle commitment and challenger.
//!
//! The types are the circle configuration Plonky3 0.7.0 itself runs, identical in `p3-circle`'s
//! `pcs.rs` tests and in `p3-uni-stark`'s `fib_air.rs`, `mul_air.rs` and `periodic_air.rs`: rows of
//! Mersenne31 elements serialized to bytes and hashed with Keccak-256 into a binary Merkle tree
//! (`MerkleTreeMmcs`), the circle FRI commitment (`CirclePcs`), a Keccak-256 challenger, and
//! challenges in the degree-3 binomial extension of Mersenne31. One thing differs from those
//! tests, because they are tests: the FRI parameters are `FriParameters::new_benchmark`, Plonky3's
//! production-like set without hiding (binary folding, rate 1/2, 100 queries, 16 bits of query
//! proof of work), instead of their 40 queries and 8 bits.
//!
//! Nothing here is random. `CirclePcs` has no hiding variant and `MerkleTreeMmcs` no salts, so
//! there is no masking randomness to seed, and the same claim always gives the same proof.

use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_circle::CirclePcs;
use p3_commit::ExtensionMmcs;
use p3_field::extension::BinomialExtensionField;
use p3_fri::FriParameters;
use p3_keccak::Keccak256Hash;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher};

use crate::field::Val;

/// Keccak-256 over bytes.
type ByteHash = Keccak256Hash;
/// Hashes field elements by serializing them into bytes.
type FieldHash = SerializingHasher<ByteHash>;
/// Two-to-one compression for the inner Merkle nodes.
type Compress = CompressionFunctionFromHasher<ByteHash, 2, 32>;
/// The Merkle tree over base-field matrices, with 32-byte digests.
type ValMmcs = MerkleTreeMmcs<Val, u8, FieldHash, Compress, 2, 32>;
/// Challenges, and every opened value, live in Mersenne31's degree-3 extension.
pub(crate) type Challenge = BinomialExtensionField<Val, 3>;
/// The same tree over extension-field matrices, for FRI's folded codewords.
type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
/// Fiat–Shamir over Keccak-256.
type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;
/// The circle FRI polynomial commitment.
type Pcs = CirclePcs<Val, ValMmcs, ChallengeMmcs>;

/// The complete configuration `p3_uni_stark::prove` and `verify` take.
pub(crate) type Config = p3_uni_stark::StarkConfig<Pcs, Challenge, Challenger>;

/// Builds the configuration; every part is public and reproducible.
pub(crate) fn build() -> Config {
    let byte_hash = ByteHash {};
    let val_mmcs = ValMmcs::new(FieldHash::new(byte_hash), Compress::new(byte_hash), 0);
    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());
    let pcs = Pcs::new(val_mmcs, FriParameters::new_benchmark(challenge_mmcs));
    let challenger = Challenger::from_hasher(vec![], byte_hash);
    Config::new(pcs, challenger)
}
