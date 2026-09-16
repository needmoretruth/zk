//! One example laid out as a layered circuit, with Remainder's Hyrax prover and verifier ready:
//! prove, verify and the attack hooks.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use frontend::layouter::builder::Circuit as LayeredCircuit;
use hyrax::gkr::verify_hyrax_proof;
use hyrax::provable_circuit::HyraxProvableCircuit;
use hyrax::utils::vandermonde::VandermondeInverse;
use remainder::mle::evals::MultilinearExtension;
use shared_types::config::{GKRCircuitProverConfig, GKRCircuitVerifierConfig};
use shared_types::halo2curves::ff::Field as _;
use shared_types::pedersen::PedersenCommitter;
use shared_types::transcript::ec_transcript::ECTranscript;
use shared_types::transcript::poseidon_sponge::PoseidonSponge;
use shared_types::{Bn256Point, Fq, Fr, perform_function_under_expected_configs};
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{self, Bn254Scalar};
use crate::graph::Graph;
use crate::layered::{self, COMMITTED_TABLE, PUBLIC_TABLE};
use crate::levels::Layout;
use crate::proof;

/// The transcript both sides run: Poseidon over BN254's base field, absorbing curve points and
/// scalars, as in Remainder's Hyrax examples.
type Transcript = ECTranscript<Bn256Point, PoseidonSponge<Fq>>;

/// The label both transcripts start from; prover and verifier must agree on it.
const TRANSCRIPT_LABEL: &str = "zk museum / gkr / hyrax transcript";

/// The public string the Pedersen generators are hashed from (Remainder requires 32 bytes or more).
const GENERATOR_LABEL: &str = "zk museum / gkr / hyrax pedersen generators";

/// Generators for vector commitments; the upstream Hyrax tutorial's number.
///
/// The longest vector a proof here commits to is one gate layer's padded sum-check messages: three
/// coefficients (degree two) for every variable of the two levels its gates read. The widest level of
/// these statements holds 245 values (pool-spend), eight variables, so that is at most 3 · 16 = 48;
/// a row of the committed table, at most 2^7 entries laid out square, is 8. A statement that needed
/// more would make the prover panic on its honest claim, which the example tests would report.
const GENERATORS: usize = 512;

/// A statement lowered to R1CS, rewritten as a layered circuit, and built by Remainder.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<Bn254Scalar>,
    r1cs: R1cs<Bn254Scalar>,
    graph: Graph,
    layout: Layout,
    layered: LayeredCircuit<Fr>,
    committer: PedersenCommitter<Bn256Point>,
    /// Remainder reads its configuration from one process-wide lock, and rewrites it whenever a
    /// call asks for a different one while waiting for every other call to finish. Proving and
    /// verifying both ask for the same pair, so the lock is written once and never waited on again.
    prover_config: GKRCircuitProverConfig,
    verifier_config: GKRCircuitVerifierConfig,
    setup_bytes: u64,
}

impl Ready {
    /// Builds the circuit, lays it out, has Remainder's frontend build the layered circuit, and hashes
    /// the Pedersen generators from a public string. Nothing here is secret or random.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<Bn254Scalar>().map_err(|e| SystemError::Failed(e.to_string()))?;
        let r1cs = R1cs::from_circuit(&circuit);
        let graph = Graph::from_r1cs(&r1cs).map_err(SystemError::Failed)?;
        let layout = Layout::of(&graph).map_err(SystemError::Failed)?;
        let layered = layered::build(&layout).map_err(SystemError::Failed)?;
        control.checkpoint()?;
        let committer = PedersenCommitter::new(GENERATORS, GENERATOR_LABEL, None);
        let setup_bytes = proof::generators_size(&committer)?;
        let prover_config = GKRCircuitProverConfig::hyrax_compatible_runtime_optimized_default();
        let verifier_config =
            GKRCircuitVerifierConfig::new_from_prover_config(&prover_config, false);
        Ok(Self {
            example,
            circuit,
            r1cs,
            graph,
            layout,
            layered,
            committer,
            prover_config,
            verifier_config,
            setup_bytes,
        })
    }

    /// Evaluates `assignment` without checking it, hands the public and committed tables to
    /// Remainder, which computes every layer itself, and runs the Hyrax prover.
    fn prove_values(
        &self,
        assignment: &Assignment<Bn254Scalar>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(assignment)
            .map_err(|e| SystemError::Failed(e.to_string()))?;
        let values: Vec<Fr> =
            self.r1cs.assignment(&evaluation.values).iter().map(|value| value.0).collect();
        let public: Vec<Fr> = assignment.public.iter().map(|value| value.0).collect();
        let mut circuit = self.layered.clone();
        let committed = self.layout.committed_table(&self.graph, &values);
        circuit.set_input(PUBLIC_TABLE, self.public_table(&public));
        circuit.set_input(COMMITTED_TABLE, MultilinearExtension::new(committed));
        let mut provable: HyraxProvableCircuit<Bn256Point> =
            circuit
                .gen_hyrax_provable_circuit()
                .map_err(|e| SystemError::Failed(format!("Remainder: {e:#}")))?;
        control.checkpoint()?;
        let proved = catch_unwind(AssertUnwindSafe(|| {
            // Blinding factors come from the thread's ChaCha generator, seeded by the operating system.
            let mut rng = rand::thread_rng();
            let mut converter = VandermondeInverse::new();
            let mut transcript = Transcript::new(TRANSCRIPT_LABEL);
            perform_function_under_expected_configs!(
                |committer, rng, converter, transcript| provable
                    .prove(committer, rng, converter, transcript),
                &self.prover_config,
                &self.verifier_config,
                &self.committer,
                &mut rng,
                &mut converter,
                &mut transcript
            )
        }));
        let sent = proved.map_err(|panic| SystemError::Unsatisfied(panic_message(&*panic)))?;
        let encode = |values: &[Bn254Scalar]| values.iter().map(|v| field::encode(&v.0)).collect();
        Ok(Proven {
            proof: proof::encode(&sent)?,
            public: encode(&assignment.public),
            secrets: encode(&assignment.private),
        })
    }

    fn public_table(&self, public: &[Fr]) -> MultilinearExtension<Fr> {
        MultilinearExtension::new(self.layout.public_table(&self.graph, public))
    }
}

impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        let count = |value: usize| value as u64;
        CircuitShape {
            form: ShapeForm::Native,
            counts: vec![
                ("layers".into(), count(self.layout.num_layers())),
                ("gates".into(), count(self.layout.num_gates())),
            ],
        }
    }

    /// Length of the Pedersen generators in bincode: public points hashed from a fixed string.
    fn setup_bytes(&self) -> Option<u64> {
        Some(self.setup_bytes)
    }

    fn prove(&mut self, claim: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        if claim.example != self.example {
            let (asked, built) = (claim.example.id(), self.example.id());
            return Err(SystemError::Failed(format!(
                "asked to prove {asked} with a circuit built for {built}"
            )));
        }
        let assignment = claim.example.instance::<Bn254Scalar>(claim.kind, &claim.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: evaluated without checks, so a false assignment still reaches Remainder's prover.
    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let decode_all = |values: &[FieldBytes]| -> Result<Vec<Bn254Scalar>, SystemError> {
            values
                .iter()
                .map(|bytes| field::decode(bytes).map(Bn254Scalar).map_err(SystemError::Failed))
                .collect()
        };
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_values(&assignment, control)
    }

    /// Runs `verify_hyrax_proof` against the verifier's own public table. Remainder's verifier
    /// reports every failed check by panicking (`assert_eq!` on a mismatched public table, a failed
    /// proof of dot product, a claim with no counterpart), so a panic is its rejection.
    fn verify(
        &mut self,
        public: &[FieldBytes],
        bytes: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let public = match field::decode_all(public, self.graph.num_public) {
            Ok(public) => public,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let (proof, config) = match proof::decode(bytes) {
            Ok(sent) => sent,
            Err(why) => return Ok(Verdict::Malformed(why)),
        };
        let mut circuit = self.layered.clone();
        circuit.set_input(PUBLIC_TABLE, self.public_table(&public));
        let verifiable = circuit
            .gen_hyrax_verifiable_circuit::<Bn256Point>()
            .map_err(|e| SystemError::Failed(format!("Remainder: {e:#}")))?;
        let checked = catch_unwind(AssertUnwindSafe(|| {
            let mut transcript = Transcript::new(TRANSCRIPT_LABEL);
            perform_function_under_expected_configs!(
                verify_hyrax_proof,
                &self.prover_config,
                &self.verifier_config,
                &proof,
                &verifiable,
                &self.committer,
                &mut transcript,
                &config
            )
        }));
        Ok(if checked.is_ok() { Verdict::Accepted } else { Verdict::Rejected })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let scalar = field::decode(value).map_err(SystemError::Failed)?;
        Ok(field::encode(&(scalar + Fr::ONE)))
    }

    fn tamper_offset(&self, bytes: &[u8]) -> usize {
        proof::tamper_offset(bytes)
    }
}

/// The text a panic carried, when it carried text.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => format!("Remainder panicked: {text}"),
        (_, Some(text)) => format!("Remainder panicked: {text}"),
        _ => "Remainder panicked".to_string(),
    }
}
