//! Calls into Miden: the real assembler, processor, prover and verifier, with proofs as bytes.

use std::any::Any;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use miden_assembly::Assembler;
use miden_core::Felt;
use miden_core::mast::MastNode;
use miden_processor::advice::{AdviceInputs, AdviceStack};
use miden_processor::operation::OperationError;
use miden_processor::{
    DefaultHost, ExecutionError, ExecutionOptions, Program, ProgramInfo, StackInputs,
};
use miden_prover::{ExecutionProof, Prover, StackOutputs, prove_sync};
use miden_verifier::{ExecutionClaim, VerificationError, Verifier};
use zk_core::{SystemError, Verdict};

/// Compiles Miden Assembly into a program with Miden's own assembler.
pub(crate) fn assemble(text: &str) -> Result<Program, SystemError> {
    let package = Assembler::default().assemble_program("museum", text).map_err(|e| {
        SystemError::Failed(format!("the Miden assembler refused the program: {e}"))
    })?;
    package
        .try_into_program()
        .map_err(|e| SystemError::Failed(format!("the package holds no program: {e}")))
}

/// VM operations in the compiled program's basic blocks, the instructions the processor steps.
pub(crate) fn operations(program: &Program) -> u64 {
    program
        .mast_forest()
        .nodes()
        .iter()
        .map(|node| match node {
            MastNode::Block(block) => u64::from(block.num_operations()),
            _ => 0,
        })
        .sum()
}

/// Executes `program` on the public stack inputs and the advice, proves the execution with
/// `miden_prover::prove_sync`, and returns the proof in Miden's own byte encoding.
///
/// Nothing is checked beforehand. The processor runs every `assertz`, so a false claim stops
/// execution at its first violated assertion: that is Miden's prover refusing, reported with the
/// circuit label the failed error code stands for.
pub(crate) fn prove(
    program: &Program,
    public: &[Felt],
    advice: Vec<Felt>,
    labels: &BTreeMap<u64, String>,
) -> Result<Vec<u8>, SystemError> {
    let stack = StackInputs::new(public)
        .map_err(|e| SystemError::Failed(format!("the stack inputs do not fit: {e}")))?;
    let mut advice_stack = AdviceStack::new();
    advice_stack.append_elements(advice);
    let advice = AdviceInputs::default().with_stack(advice_stack);
    let proved = catch_unwind(AssertUnwindSafe(|| {
        let mut host = DefaultHost::default();
        let options = ExecutionOptions::default();
        prove_sync(&Prover::new(), program, stack, advice, &mut host, options)
    }));
    match proved {
        Ok(Ok((_, proof))) => Ok(proof.to_bytes()),
        Ok(Err(error)) => Err(refusal(error, labels)),
        Err(panic) => Err(SystemError::Failed(panic_message("prove_sync", panic.as_ref()))),
    }
}

/// A failed assertion is the prover refusing a false claim; any other execution error is a fault.
fn refusal(error: ExecutionError, labels: &BTreeMap<u64, String>) -> SystemError {
    if let ExecutionError::OperationError {
        err: OperationError::FailedAssertion { err_code, .. },
        ..
    } = &error
    {
        let code = err_code.as_canonical_u64();
        let label = labels.get(&code).map_or("an unlabelled assertion", String::as_str);
        return SystemError::Unsatisfied(format!("Miden execution failed an assertion: {label}"));
    }
    SystemError::Failed(format!("Miden could not execute or prove the program: {error}"))
}

/// Decodes `bytes` and runs `Verifier::verify` on the claim that `info`'s program, started on
/// `public`, halts with sixteen zeros on the stack.
///
/// Bytes Miden cannot decode, a STARK proof whose own encoding does not decode or is too large,
/// proof-format and compatibility mismatches, and verifier panics are malformed. A proof that the
/// STARK verifier checks and refuses, including one that proves a different program, other stack
/// inputs or other outputs, is a rejection. So is a proof that leaves precompile work unproved,
/// because it does not settle the whole claim.
pub(crate) fn verify(info: &ProgramInfo, public: &[Felt], bytes: &[u8]) -> Verdict {
    let proof = match ExecutionProof::read_from_bytes(bytes) {
        Ok(proof) => proof,
        Err(e) => return Verdict::Malformed(format!("Miden could not decode the proof: {e}")),
    };
    let stack = match StackInputs::new(public) {
        Ok(stack) => stack,
        Err(e) => return Verdict::Malformed(format!("the stack inputs do not fit: {e}")),
    };
    let claim = ExecutionClaim::from_program_info(info.clone(), stack, StackOutputs::default());
    let verified = catch_unwind(AssertUnwindSafe(|| Verifier::new().verify(&claim, &proof)));
    match verified {
        Ok(Ok(outcome)) if outcome.is_complete() => Verdict::Accepted,
        Ok(Ok(_)) => Verdict::Rejected,
        Ok(Err(error)) => classify(error),
        Err(panic) => Verdict::Malformed(panic_message("Verifier::verify", panic.as_ref())),
    }
}

fn classify(error: VerificationError) -> Verdict {
    use miden_verifier::StarkVerificationError as Stark;
    match &error {
        VerificationError::StarkVerificationError(_, inner) => match inner.as_ref() {
            Stark::Verifier(_) => Verdict::Rejected,
            Stark::Deserialization(_) | Stark::ProofTooLarge { .. } => {
                Verdict::Malformed(error.to_string())
            }
        },
        VerificationError::PrecompileStarkVerification(_) => Verdict::Rejected,
        _ => Verdict::Malformed(error.to_string()),
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(step: &str, payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("{step} panicked: {text}"),
        (_, Some(text)) => format!("{step} panicked: {text}"),
        _ => format!("{step} panicked"),
    }
}
