//! The museum's R1CS rebuilt, row for row, through Bulletproofs' `ConstraintSystem` API.
//!
//! Bulletproofs has no circuit description of its own: the prover and the verifier each replay the
//! same gadget code against their own constraint system, and a proof verifies only if both replays
//! built the same constraints. One function serves both sides, so they cannot drift apart.

use bulletproofs::r1cs::{ConstraintSystem, LinearCombination, R1CSError, Variable};
use curve25519_dalek::scalar::Scalar;
use merlin::Transcript;
use zk_circuit::lower::r1cs::{R1cs, SparseLc};
use zk_core::ExampleId;

use crate::field::RistrettoScalar;

/// Separates this exhibit's transcripts from any other use of the same example label.
const DOMAIN: &[u8] = b"needmoretruth/zk sys-bulletproofs r1cs v1";

/// The transcript both sides start from, bound to the statement before any proof element.
///
/// The label is the example ID, which fixes the circuit. Public inputs enter the constraints only as
/// constants, which the upstream protocol never hashes, so they are absorbed here as well: otherwise
/// a prover could see the challenges first and then pick public inputs that balance a forged proof.
/// Upstream's own gadget documentation commits statement parameters to the transcript the same way.
pub(crate) fn transcript(example: ExampleId, public: &[Scalar]) -> Transcript {
    let mut transcript = Transcript::new(example.id().as_bytes());
    transcript.append_message(b"dom-sep", DOMAIN);
    transcript.append_u64(b"public-inputs", public.len() as u64);
    for value in public {
        transcript.append_message(b"public-input", value.as_bytes());
    }
    transcript
}

/// Where one R1CS variable lives in the Bulletproofs constraint system.
#[derive(Clone, Copy)]
enum Slot {
    /// `z[0] = 1` or a public input: a value both sides know, entering as a multiple of `One`.
    Known(Scalar),
    /// A private input, multiplication output or hint output: a low-level variable of the proof.
    Hidden(Variable),
}

/// Builds every row of `r1cs` in `cs`, in constraint order.
///
/// `witness` is the full assignment `z = [1, public, private, internal]` on the prover's side and
/// `None` on the verifier's. Private variables are allocated with `allocate`, which packs them two
/// to a multiplier as its left and right inputs; nothing about them leaves the proof, so no Pedersen
/// commitments are published. A multiplication row `A·B = C` becomes `multiply(A, B)` plus
/// `constrain(out − C)`; an assert-zero row (`B = 1`, `C` empty) becomes `constrain(A)`.
pub(crate) fn synthesize<CS: ConstraintSystem>(
    cs: &mut CS,
    r1cs: &R1cs<RistrettoScalar>,
    public: &[Scalar],
    witness: Option<&[RistrettoScalar]>,
) -> Result<(), R1CSError> {
    let slots = allocate(cs, r1cs, public, witness)?;
    let constant_one = [(0, RistrettoScalar(Scalar::ONE))];
    for row in r1cs.constraints() {
        let a = combination(&row.a, &slots);
        if row.b == constant_one && row.c.is_empty() {
            cs.constrain(a);
        } else {
            let (_, _, out) = cs.multiply(a, combination(&row.b, &slots));
            cs.constrain(out - combination(&row.c, &slots));
        }
    }
    Ok(())
}

/// One slot per R1CS variable: the constant and public inputs as known values, the rest allocated.
fn allocate<CS: ConstraintSystem>(
    cs: &mut CS,
    r1cs: &R1cs<RistrettoScalar>,
    public: &[Scalar],
    witness: Option<&[RistrettoScalar]>,
) -> Result<Vec<Slot>, R1CSError> {
    let expected = r1cs.num_public_inputs();
    if public.len() != expected {
        let description = format!("expected {expected} public inputs, got {}", public.len());
        return Err(R1CSError::GadgetError { description });
    }
    let mut slots = Vec::with_capacity(r1cs.num_variables());
    slots.push(Slot::Known(Scalar::ONE));
    slots.extend(public.iter().map(|value| Slot::Known(*value)));
    for index in slots.len()..r1cs.num_variables() {
        let value = witness
            .map(|z| z.get(index).map(|value| value.0).ok_or(R1CSError::MissingAssignment))
            .transpose()?;
        slots.push(Slot::Hidden(cs.allocate(value)?));
    }
    Ok(slots)
}

/// `Σ coefficient · variable` over slots. Indices come from the lowering, which only emits indices
/// below `num_variables`, the length of `slots`.
fn combination(terms: &SparseLc<RistrettoScalar>, slots: &[Slot]) -> LinearCombination {
    terms
        .iter()
        .map(|(index, coefficient)| match slots[*index] {
            Slot::Known(value) => (Variable::One(), coefficient.0 * value),
            Slot::Hidden(variable) => (variable, coefficient.0),
        })
        .collect()
}
