//! The thread that owns ark-marlin's prover key, because that key cannot cross threads.
//!
//! `IndexProverKey` holds the index polynomials as `LabeledPolynomial`s, which keep their
//! coefficients behind an `Rc`, so the key is not `Send`, while [`zk_core::Prepared`] must be, so
//! that a prepared system can be handed to another thread. Rather than re-serializing the
//! key for every proof, which would add decoding time to the prover's measured time, one thread runs
//! both setup steps, keeps the key, and proves whatever assignment it is sent. Only `Send` values
//! cross: the verifier key, sizes, assignment vectors and proof bytes.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ark_bls12_381::Fr as Scalar;
use ark_marlin::{IndexProverKey, IndexVerifierKey};
use ark_serialize::CanonicalSerialize;
use rand_core::OsRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_core::{Control, SystemError};

use crate::field::Fr;
use crate::setup::{self, Kzg, Upstream};
use crate::synthesis::Synthesis;

/// One proof to make: the full assignment vector `z`, and where to send the bytes.
struct Request {
    z: Vec<Fr>,
    reply: Sender<Result<Vec<u8>, SystemError>>,
}

/// A handle on the proving thread; dropping it closes the channel and the thread ends.
pub(crate) struct ProverThread {
    requests: Sender<Request>,
}

/// What setup produced that may leave the proving thread.
pub(crate) struct Indexed {
    /// The handle that proves with the key kept on its thread.
    pub(crate) prover: ProverThread,
    /// The verifier's index key.
    pub(crate) verifier_key: IndexVerifierKey<Scalar, Kzg>,
    /// Compressed size of the universal SRS.
    pub(crate) srs_bytes: u64,
    /// Compressed size of the prover's index key, verifier key included.
    pub(crate) index_bytes: u64,
}

type SetupReply = Result<(IndexVerifierKey<Scalar, Kzg>, u64, u64), SystemError>;

/// Starts the proving thread, which runs `Marlin::universal_setup` and `Marlin::index` for `r1cs`,
/// and waits for both to finish.
pub(crate) fn start(r1cs: Arc<R1cs<Fr>>, control: &Control) -> Result<Indexed, SystemError> {
    let (setup_tx, setup_rx) = mpsc::channel::<SetupReply>();
    let (requests, request_rx) = mpsc::channel::<Request>();
    let control = control.clone();
    // Named after the thread that prepared it, so a host that decides by thread name what a panic
    // means (the terminal UI shows panics on its job threads as a result instead of printing them)
    // treats a refusal inside ark-marlin's prover as if proving ran on that thread.
    let name = thread::current().name().unwrap_or("ark-marlin prover").to_string();
    let spawned = thread::Builder::new()
        .name(name)
        .spawn(move || serve(&r1cs, &control, &setup_tx, request_rx))
        .map_err(|e| SystemError::Failed(format!("could not start the proving thread: {e}")))?;
    match setup_rx.recv() {
        Ok(Ok((verifier_key, srs_bytes, index_bytes))) => {
            Ok(Indexed { prover: ProverThread { requests }, verifier_key, srs_bytes, index_bytes })
        }
        Ok(Err(error)) => Err(error),
        // The thread dropped its sender without replying, which only a panic in setup does.
        Err(_) => Err(SystemError::Failed(match spawned.join() {
            Err(panic) => panic_message(panic.as_ref()),
            Ok(()) => "the proving thread stopped during setup".to_string(),
        })),
    }
}

/// The thread's whole life: set up, report, then prove until the handle is dropped.
fn serve(
    r1cs: &R1cs<Fr>,
    control: &Control,
    setup_tx: &Sender<SetupReply>,
    requests: Receiver<Request>,
) {
    let keys = match setup::run(r1cs, control) {
        Ok(keys) => keys,
        Err(error) => {
            // Nobody is left to tell if the caller has gone; the thread simply ends.
            let _ = setup_tx.send(Err(error));
            return;
        }
    };
    if setup_tx.send(Ok((keys.verifier, keys.srs_bytes, keys.index_bytes))).is_err() {
        return;
    }
    for request in requests {
        // A caller that stopped waiting needs no answer.
        let _ = request.reply.send(prove(&keys.prover, r1cs, &request.z));
    }
}

impl ProverThread {
    /// Sends `z` to the proving thread and waits for the proof in ark-marlin's own serialization.
    pub(crate) fn prove(&self, z: Vec<Fr>) -> Result<Vec<u8>, SystemError> {
        let (reply, answer) = mpsc::channel();
        self.requests
            .send(Request { z, reply })
            .map_err(|_| SystemError::Failed("the proving thread has stopped".to_string()))?;
        answer
            .recv()
            .map_err(|_| SystemError::Failed("the proving thread has stopped".to_string()))?
    }
}

/// `Marlin::prove` with fresh OS randomness, serialized compressed.
fn prove(
    prover_key: &IndexProverKey<Scalar, Kzg>,
    r1cs: &R1cs<Fr>,
    z: &[Fr],
) -> Result<Vec<u8>, SystemError> {
    let synthesis = Synthesis::with_witness(r1cs, z);
    // ark-marlin's prover never calls `is_satisfied`. Its only reaction to a false witness is a
    // `debug_assert!` that the outer sum-check vanishes, so a build with debug assertions panics
    // here and a release build returns a proof for the verifier to refuse; either refusal is
    // arkworks' own and is reported as one.
    let proved =
        catch_unwind(AssertUnwindSafe(|| Upstream::prove(prover_key, synthesis, &mut OsRng)));
    let proof = match proved {
        Ok(Ok(proof)) => proof,
        Ok(Err(refusal)) => return Err(SystemError::Unsatisfied(format!("{refusal:?}"))),
        Err(panic) => return Err(SystemError::Unsatisfied(panic_message(panic.as_ref()))),
    };
    let mut bytes = Vec::new();
    proof
        .serialize(&mut bytes)
        .map_err(|e| SystemError::Failed(format!("ark-marlin proof serialization: {e}")))?;
    Ok(bytes)
}

/// The text a panic carried, when it carried text.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("ark-marlin panicked: {text}"),
        (_, Some(text)) => format!("ark-marlin panicked: {text}"),
        _ => "ark-marlin panicked".to_string(),
    }
}
