//! Marlin's two setup steps: a universal SRS sized for the circuit, then the circuit's index.

use ark_bls12_381::{Bls12_381, Fr as Scalar};
use ark_marlin::{IndexProverKey, IndexVerifierKey};
use ark_poly::univariate::DensePolynomial;
use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_serialize::CanonicalSerialize;
use blake2::Blake2s;
use rand_core::OsRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_core::{Control, SystemError};

use crate::field::Fr;
use crate::synthesis::Synthesis;

/// The `CircuitShape` count that carries the universal SRS's compressed size in bytes.
///
/// The SRS is not per-circuit material, so it is kept out of `setup_bytes`, which counts only what
/// indexing derived for this circuit; the shape is where the comparison screen can still show it.
pub const SRS_BYTES_COUNT: &str = "universal-srs-bytes";

/// KZG commitments over BLS12-381 with Marlin's degree-bound enforcement, as in the paper.
pub(crate) type Kzg = MarlinKZG10<Bls12_381, DensePolynomial<Scalar>>;

/// ark-marlin instantiated exactly as its own tests do: `MarlinKZG10` and a BLAKE2s Fiat–Shamir RNG.
pub(crate) type Upstream = ark_marlin::Marlin<Scalar, Kzg, Blake2s>;

/// What indexing derived for one circuit, and what the SRS it was derived from weighed.
pub(crate) struct Keys {
    /// The prover's index key; it carries the verifier's key inside it.
    pub(crate) prover: IndexProverKey<Scalar, Kzg>,
    /// The verifier's index key.
    pub(crate) verifier: IndexVerifierKey<Scalar, Kzg>,
    /// Compressed size of the universal SRS.
    pub(crate) srs_bytes: u64,
    /// Compressed size of the prover's index key, verifier key included.
    pub(crate) index_bytes: u64,
}

/// The three sizes `Marlin::universal_setup` takes, counted the way ark-marlin's indexer counts them.
///
/// The indexer pads the formatted public input `(1, x)` to a power of two with zero inputs before
/// counting variables, and takes the densest of `A`, `B` and `C` as the non-zero count. An SRS sized
/// from the unpadded counts could be too small for the index, which `Marlin::index` refuses with
/// `IndexTooLarge`; one sized from larger counts would work but overstate the setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IndexSize {
    constraints: usize,
    variables: usize,
    non_zero: usize,
}

impl IndexSize {
    fn of(r1cs: &R1cs<Fr>) -> Self {
        let public = r1cs.num_public_inputs();
        let formatted_inputs = (public + 1).next_power_of_two();
        let witnesses = r1cs.num_variables() - (public + 1);
        let rows = r1cs.constraints();
        let a: usize = rows.iter().map(|row| row.a.len()).sum();
        let b: usize = rows.iter().map(|row| row.b.len()).sum();
        let c: usize = rows.iter().map(|row| row.c.len()).sum();
        Self {
            constraints: r1cs.num_constraints(),
            variables: formatted_inputs + witnesses,
            non_zero: a.max(b).max(c),
        }
    }
}

/// Runs `Marlin::universal_setup` with OS randomness, then `Marlin::index` for `r1cs` against it.
///
/// The SRS is dropped once the index is built: proving and verifying need only the committer and
/// verifier keys `Marlin::index` trimmed from it, which the index keys carry.
pub(crate) fn run(r1cs: &R1cs<Fr>, control: &Control) -> Result<Keys, SystemError> {
    let size = IndexSize::of(r1cs);
    let srs =
        Upstream::universal_setup(size.constraints, size.variables, size.non_zero, &mut OsRng)
            .map_err(|e| SystemError::Failed(format!("ark-marlin universal_setup: {e:?}")))?;
    let srs_bytes = srs.serialized_size() as u64;
    control.checkpoint()?;
    let (prover, verifier) = Upstream::index(&srs, Synthesis::without_witness(r1cs))
        .map_err(|e| SystemError::Failed(format!("ark-marlin index: {e:?}")))?;
    let index_bytes = prover.serialized_size() as u64;
    Ok(Keys { prover, verifier, srs_bytes, index_bytes })
}
