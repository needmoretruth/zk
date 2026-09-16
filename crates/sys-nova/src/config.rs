//! Which of nova-snark's engines and compressing SNARKs the exhibit runs, named once.

use nova_snark::nova::{CompressedSNARK, ProverKey, PublicParams, RecursiveSNARK, VerifierKey};
use nova_snark::provider::{PallasEngine, VestaEngine, ipa_pc};
use nova_snark::spartan::snark::RelaxedR1CSSNARK;

use crate::step::StepFunction;

/// Steps in every chain: `RecursiveSNARK::new` and then `prove_step` this many times.
///
/// The first `prove_step` only records the base case the constructor already synthesized; the second
/// folds a second run of `F` into it. Two is the fewest steps in which the verifier checks a real fold
/// of the statement, and the verifier is told this number.
pub const NUM_STEPS: usize = 2;

/// The primary engine: the Pallas curve, whose scalar field the statements are written in.
pub(crate) type Primary = PallasEngine;

/// The secondary engine: Vesta, the other half of the cycle, running Nova's trivial circuit.
pub(crate) type Secondary = VestaEngine;

/// Spartan without preprocessing, opened with the inner-product argument: no trusted setup.
pub(crate) type Snark<E> = RelaxedR1CSSNARK<E, ipa_pc::EvaluationEngine<E>>;

/// Nova's public parameters for one statement's step function.
pub(crate) type Params = PublicParams<Primary, Secondary, StepFunction>;

/// The folded, uncompressed proof of the chain.
pub(crate) type Recursive = RecursiveSNARK<Primary, Secondary, StepFunction>;

/// The zero-knowledge SNARK that proves knowledge of a valid [`Recursive`].
pub(crate) type Compressed =
    CompressedSNARK<Primary, Secondary, StepFunction, Snark<Primary>, Snark<Secondary>>;

/// What `CompressedSNARK::prove` needs besides the parameters.
pub(crate) type ProvingKey =
    ProverKey<Primary, Secondary, StepFunction, Snark<Primary>, Snark<Secondary>>;

/// What `CompressedSNARK::verify` needs.
pub(crate) type VerifyingKey =
    VerifierKey<Primary, Secondary, StepFunction, Snark<Primary>, Snark<Secondary>>;
