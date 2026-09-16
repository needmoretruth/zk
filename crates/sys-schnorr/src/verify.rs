//! Verifying: rebuild the relation from the public inputs and the published commitments, then let
//! `sigma-proofs` check its proof.

use ff::Field as _;
use sigma_proofs::traits::SigmaProtocol;
use zk_circuit::Circuit;
use zk_core::{ExampleId, FieldBytes, Verdict};

use crate::field::{PallasScalar, decode};
use crate::layout::{Check, Layout};
use crate::pedersen::Generators;
use crate::proof;
use crate::relation;

/// Undecodable inputs, commitments or upstream bytes are [`Verdict::Malformed`]. Everything else is
/// the verifier's decision: a statement `sigma-proofs` refuses to form (a commitment that makes an
/// equation degenerate), a public check that fails, or a proof `verify_batchable` rejects.
pub(crate) fn verify(
    circuit: &Circuit<PallasScalar>,
    example: ExampleId,
    generators: &Generators,
    public: &[FieldBytes],
    bytes: &[u8],
) -> Verdict {
    let Some(values) = public.iter().map(|bytes| decode(bytes)).collect::<Option<Vec<_>>>() else {
        return Verdict::Malformed("a public input is not a canonical Pallas scalar".into());
    };
    let expected = circuit.public_inputs().len();
    if values.len() != expected {
        return Verdict::Malformed(format!(
            "expected {expected} public inputs, got {}",
            values.len()
        ));
    }
    let layout = Layout::new(circuit, &values);
    let (commitments, upstream) = match proof::split(bytes, layout.committed.len()) {
        Ok(parts) => parts,
        Err(why) => return Verdict::Malformed(why),
    };
    let statement = relation::build(&layout, &commitments, generators);
    let session = proof::session(example, &values, &commitments);
    let Ok(nizk) = statement.into_nizk(&session) else { return Verdict::Rejected };
    let shape = nizk.interactive_proof.commitment_len();
    let responses = nizk.interactive_proof.response_len();
    if let Err(why) = proof::check_upstream_shape(upstream, shape, responses) {
        return Verdict::Malformed(why);
    }
    let public_checks_hold = layout.checks.iter().all(|check| match check {
        Check::KnownZero(value) => value.is_zero_vartime(),
        _ => true,
    });
    if public_checks_hold && nizk.verify_batchable(upstream).is_ok() {
        Verdict::Accepted
    } else {
        Verdict::Rejected
    }
}
