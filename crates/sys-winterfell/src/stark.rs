//! Calls into `winter-prover` and `winter-verifier`: the real prover and verifier, in bytes.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use winter_air::proof::Proof;
use winter_air::{
    AuxRandElements, BatchingMethod, ConstraintCompositionCoefficients, FieldExtension,
    PartitionOptions, ProofOptions, TraceInfo,
};
use winter_crypto::hashers::Blake3_256;
use winter_crypto::{DefaultRandomCoin, MerkleTree};
use winter_math::FieldElement;
use winter_prover::matrix::ColMatrix;
use winter_prover::{
    CompositionPoly, CompositionPolyTrace, DefaultConstraintCommitment, DefaultConstraintEvaluator,
    DefaultTraceLde, Prover, StarkDomain, TracePolyTable, TraceTable,
};
use winter_utils::{Deserializable, Serializable, SliceReader};
use winter_verifier::{AcceptableOptions, VerifierError};
use zk_core::{SystemError, Verdict};

use crate::air::{CircuitAir, Statement, TRACE_LENGTH};
use crate::field::Val;

/// Winterfell's standard hash: Blake3 with a 256-bit digest (128-bit collision resistance).
type Hash = Blake3_256<Val>;
/// The Merkle tree Winterfell commits to trace and constraint evaluations with.
type Commitment = MerkleTree<Hash>;
/// Fiat–Shamir over the same hash.
type Coin = DefaultRandomCoin<Hash>;

/// The protocol parameters of Winterfell's own documented example, which it describes as "enough
/// for ~96-bit security level": 32 queries, blowup 8, no grinding, no field extension, FRI folding
/// factor 8, FRI remainder degree 31, linear batching for both compositions.
///
/// Winterfell's own estimate (`Proof::conjectured_security`) puts these at 95 bits, because it
/// subtracts one from the 96 bits of the queries; the same documentation verifies with
/// [`MIN_SECURITY_BITS`] of 95.
pub(crate) const OPTIONS: ProofOptions = ProofOptions::new(
    32,
    8,
    0,
    FieldExtension::None,
    8,
    31,
    BatchingMethod::Linear,
    BatchingMethod::Linear,
);

/// Conjectured security the verifier demands, as in Winterfell's documented example.
const MIN_SECURITY_BITS: u32 = 95;

/// The prover for one claim: Winterfell's default trace extension, constraint evaluator and
/// constraint commitment, with the claim's public inputs fixed.
struct CircuitProver {
    statement: Statement,
}

impl Prover for CircuitProver {
    type BaseField = Val;
    type Air = CircuitAir;
    type Trace = TraceTable<Val>;
    type HashFn = Hash;
    type VC = Commitment;
    type RandomCoin = Coin;
    type TraceLde<E: FieldElement<BaseField = Val>> = DefaultTraceLde<E, Hash, Commitment>;
    type ConstraintCommitment<E: FieldElement<BaseField = Val>> =
        DefaultConstraintCommitment<E, Hash, Commitment>;
    type ConstraintEvaluator<'a, E: FieldElement<BaseField = Val>> =
        DefaultConstraintEvaluator<'a, CircuitAir, E>;

    /// The claimed public inputs, not values read back from the trace: a false claim must reach
    /// the verifier as claimed.
    fn get_pub_inputs(&self, _trace: &Self::Trace) -> Statement {
        self.statement.clone()
    }

    fn options(&self) -> &ProofOptions {
        &OPTIONS
    }

    fn new_trace_lde<E: FieldElement<BaseField = Val>>(
        &self,
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<Val>,
        domain: &StarkDomain<Val>,
        partition_options: PartitionOptions,
    ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
        DefaultTraceLde::new(trace_info, main_trace, domain, partition_options)
    }

    fn new_evaluator<'a, E: FieldElement<BaseField = Val>>(
        &self,
        air: &'a CircuitAir,
        aux_rand_elements: Option<AuxRandElements<E>>,
        composition_coefficients: ConstraintCompositionCoefficients<E>,
    ) -> Self::ConstraintEvaluator<'a, E> {
        DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
    }

    fn build_constraint_commitment<E: FieldElement<BaseField = Val>>(
        &self,
        composition_poly_trace: CompositionPolyTrace<E>,
        num_constraint_composition_columns: usize,
        domain: &StarkDomain<Val>,
        partition_options: PartitionOptions,
    ) -> (Self::ConstraintCommitment<E>, CompositionPoly<E>) {
        DefaultConstraintCommitment::new(
            composition_poly_trace,
            num_constraint_composition_columns,
            domain,
            partition_options,
        )
    }
}

/// Runs `Prover::prove` on the wire row repeated [`TRACE_LENGTH`] times beside a step counter
/// climbing from 0, and returns `Proof::to_bytes`.
///
/// Nothing is checked here. In builds with debug assertions Winterfell itself validates the trace
/// against the AIR before proving and panics on a false claim; that panic, like an error the
/// prover returns, is caught and reported as the prover's refusal.
pub(crate) fn prove(statement: Statement, row: &[Val]) -> Result<Vec<u8>, SystemError> {
    let mut columns: Vec<Vec<Val>> = row.iter().map(|value| vec![*value; TRACE_LENGTH]).collect();
    columns.push((0..TRACE_LENGTH as u64).map(Val::from).collect());
    let prover = CircuitProver { statement };
    let proved = catch_unwind(AssertUnwindSafe(|| prover.prove(TraceTable::init(columns))));
    match proved {
        Ok(Ok(proof)) => Ok(proof.to_bytes()),
        Ok(Err(error)) => Err(SystemError::Unsatisfied(format!("winter_prover refused: {error}"))),
        Err(panic) => Err(SystemError::Unsatisfied(panic_message("prove", panic.as_ref()))),
    }
}

/// Decodes `bytes` and runs `winter_verifier::verify`, classifying the outcome.
///
/// Bytes Winterfell cannot decode, bytes left over after the proof, a trace shape other than this
/// statement's, and a proof the verifier cannot parse into the statement's frames are malformed.
/// Every other verifier error — an out-of-domain evaluation that does not match the constraints,
/// a Merkle opening or FRI check that fails, too little security — is a rejection.
pub(crate) fn verify(statement: Statement, width: usize, bytes: &[u8]) -> Verdict {
    let proof = match decode(bytes) {
        Ok(proof) => proof,
        Err(why) => return Verdict::Malformed(why),
    };
    let shape = proof.trace_info();
    let (columns, length) = (shape.main_trace_width(), shape.length());
    if columns != width || length != TRACE_LENGTH || shape.is_multi_segment() {
        return Verdict::Malformed(format!(
            "the proof is for {columns} columns and {length} rows, not {width} and {TRACE_LENGTH}"
        ));
    }
    let acceptable = AcceptableOptions::MinConjecturedSecurity(MIN_SECURITY_BITS);
    let verified = catch_unwind(AssertUnwindSafe(|| {
        winter_verifier::verify::<CircuitAir, Hash, Coin, Commitment>(proof, statement, &acceptable)
    }));
    match verified {
        Ok(Ok(())) => Verdict::Accepted,
        Ok(Err(
            error @ (VerifierError::ProofDeserializationError(_)
            | VerifierError::InconsistentBaseField),
        )) => Verdict::Malformed(error.to_string()),
        Ok(Err(_)) => Verdict::Rejected,
        Err(panic) => Verdict::Malformed(panic_message("verify", panic.as_ref())),
    }
}

/// `Proof::read_from` over the whole input; `Proof::from_bytes` would ignore trailing bytes.
fn decode(bytes: &[u8]) -> Result<Proof, String> {
    let mut reader = SliceReader::new(bytes);
    let proof = Proof::read_from(&mut reader)
        .map_err(|e| format!("Winterfell could not decode the proof: {e}"))?;
    let used = proof.to_bytes().len();
    if used != bytes.len() {
        return Err(format!("{} bytes follow the proof", bytes.len().saturating_sub(used)));
    }
    Ok(proof)
}

/// Where the first out-of-domain trace value starts in a serialized proof, if the bytes decode.
///
/// `Proof::write_into` writes its parts in order with no framing: the context, one byte of unique
/// query count, the commitments, the trace queries, the constraint queries, then the out-of-domain
/// frame as a two-byte length followed by one frame-size byte and the values. Re-serializing the
/// decoded parts measures the offset with Winterfell's own encoder instead of a hand-written layout.
pub(crate) fn ood_trace_offset(bytes: &[u8]) -> Option<usize> {
    let proof = decode(bytes).ok()?;
    let queries: usize = proof.trace_queries.iter().map(|q| q.to_bytes().len()).sum();
    let offset = proof.context.to_bytes().len()
        + 1
        + proof.commitments.to_bytes().len()
        + queries
        + proof.constraint_queries.to_bytes().len()
        + 2
        + 1;
    (offset < bytes.len()).then_some(offset)
}

/// The text a panic carried, when it carried text.
fn panic_message(step: &str, payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("winterfell {step} panicked: {text}"),
        (_, Some(text)) => format!("winterfell {step} panicked: {text}"),
        _ => format!("winterfell {step} panicked"),
    }
}
