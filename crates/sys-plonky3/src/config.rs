//! The STARK configuration: hash, commitment scheme, challenger and hiding randomness.
//!
//! The types follow `make_zk_config` in `p3-uni-stark` 0.7.0's own `tests/fib_air.rs`, the one
//! configuration Plonky3 ships that runs its STARK with zero knowledge: a Keccak-based Merkle tree
//! whose leaves are salted (`MerkleTreeHidingMmcs`), the hiding FRI commitment (`HidingFriPcs`), a
//! Keccak-256 challenger and challenges in the degree-4 extension of BabyBear. Two things differ
//! from that test, both because it is a test: the random generators are seeded from the operating
//! system instead of fixed seeds (Plonky3's documentation warns that predictable seeds let an
//! observer strip the masks), and the FRI parameters are `FriParameters::new_benchmark_zk`,
//! Plonky3's production-like zero-knowledge set, instead of `new_testing`.

use p3_challenger::{HashChallenger, SerializingChallenger32};
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::extension::BinomialExtensionField;
use p3_fri::{FriParameters, HidingFriPcs};
use p3_keccak::{Keccak256Hash, KeccakF, VECTOR_LEN};
use p3_merkle_tree::MerkleTreeHidingMmcs;
use p3_symmetric::{CompressionFunctionFromHasher, PaddingFreeSponge, SerializingHasher};
use rand::SeedableRng;
use rand::rngs::{StdRng, SysRng};
use zk_core::SystemError;

use crate::field::Val;

/// Random columns the hiding commitment appends to every committed matrix, as in Plonky3's test.
const RANDOM_CODEWORDS: usize = 4;

/// Salt elements per Merkle leaf: 4 × 31 bits, as in Plonky3's test.
const SALT_ELEMENTS: usize = 4;

/// Keccak-f as a sponge over 64-bit words.
type U64Hash = PaddingFreeSponge<KeccakF, 25, 17, 4>;
/// Hashes field elements by serializing them into 64-bit words.
type FieldHash = SerializingHasher<U64Hash>;
/// Two-to-one compression for the inner Merkle nodes.
type Compress = CompressionFunctionFromHasher<U64Hash, 2, 4>;
/// The salted Merkle tree over base-field matrices.
type ValMmcs = MerkleTreeHidingMmcs<
    [Val; VECTOR_LEN],
    [u64; VECTOR_LEN],
    FieldHash,
    Compress,
    StdRng,
    2,
    4,
    SALT_ELEMENTS,
>;
/// Challenges, and every opened value, live in BabyBear's degree-4 extension.
pub(crate) type Challenge = BinomialExtensionField<Val, 4>;
/// The same tree over extension-field matrices, for FRI's folded codewords.
type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
/// Fiat–Shamir over Keccak-256.
type Challenger = SerializingChallenger32<Val, HashChallenger<u8, Keccak256Hash, 32>>;
/// The hiding FRI polynomial commitment.
type Pcs = HidingFriPcs<Val, Radix2DitParallel<Val>, ValMmcs, ChallengeMmcs, StdRng>;

/// The complete configuration `p3_uni_stark::prove` and `verify` take.
pub(crate) type Config = p3_uni_stark::StarkConfig<Pcs, Challenge, Challenger>;

/// Builds the configuration with fresh operating-system randomness for the masks and salts.
pub(crate) fn build() -> Result<Config, SystemError> {
    let u64_hash = U64Hash::new(KeccakF {});
    let val_mmcs = ValMmcs::new(FieldHash::new(u64_hash), Compress::new(u64_hash), 0, fresh_rng()?);
    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());
    let fri_parameters = FriParameters::new_benchmark_zk(challenge_mmcs);
    let pcs = Pcs::new(
        Radix2DitParallel::default(),
        val_mmcs,
        fri_parameters,
        RANDOM_CODEWORDS,
        fresh_rng()?,
    );
    let challenger = Challenger::from_hasher(vec![], Keccak256Hash {});
    Ok(Config::new(pcs, challenger))
}

fn fresh_rng() -> Result<StdRng, SystemError> {
    StdRng::try_from_rng(&mut SysRng)
        .map_err(|e| SystemError::Failed(format!("no operating-system randomness: {e}")))
}
