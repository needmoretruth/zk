//! The bridge from the museum's [`ProofSystem`]/[`Prepared`] contract to the BCTV14 code.
//!
//! `prepare` builds the example circuit, lowers it to R1CS and runs the trusted setup. `prove`
//! evaluates the circuit *without* checking the witness, hands the assignment to the real prover and
//! serializes the proof. `verify` decodes and runs the five pairing checks. The adapter never
//! pre-screens a witness, so the museum can show the real verifier rejecting a false claim.

use ark_bn254::Fr;
use ark_serialize::CanonicalSerialize;
use ark_std::rand::SeedableRng;
use ark_std::rand::rngs::StdRng;
use zk_circuit::lower::r1cs::R1cs;
use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{Bn254Fr, fr_from_canonical_le, fr_to_canonical_le};
use crate::keys::{ProvingKey, VerifyingKey, generate};
use crate::proof::Proof;
use crate::{prover, verifier};

/// A BCTV14 instance ready to prove one example: circuit lowered, keys built.
pub struct PreparedBctv14 {
    circuit: Circuit<Bn254Fr>,
    r1cs: R1cs<Bn254Fr>,
    pk: ProvingKey,
    vk: VerifyingKey,
}

/// The field-element inputs for one proof: the full assignment and the public and private values.
struct ProverInputs {
    z: Vec<Fr>,
    public: Vec<Fr>,
    secrets: Vec<Fr>,
}

/// Draws a fresh CSPRNG seeded from the operating system.
pub(crate) fn os_rng() -> Result<StdRng, SystemError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)
        .map_err(|e| SystemError::Failed(format!("no OS randomness: {e}")))?;
    Ok(StdRng::from_seed(seed))
}

/// Builds the circuit, lowers it and runs the setup.
pub fn prepare(example: ExampleId, control: &Control) -> Result<PreparedBctv14, SystemError> {
    control.checkpoint()?;
    let circuit =
        example.circuit::<Bn254Fr>().map_err(|e| SystemError::Failed(format!("{e:?}")))?;
    let r1cs = R1cs::from_circuit(&circuit);
    control.checkpoint()?;
    let mut rng = os_rng()?;
    let (pk, vk) =
        generate(&r1cs, &mut rng).ok_or_else(|| SystemError::Failed("setup failed".into()))?;
    Ok(PreparedBctv14 { circuit, r1cs, pk, vk })
}

impl PreparedBctv14 {
    /// The full assignment vector `z` (index 0 the constant one) as arkworks elements, computed
    /// without stopping at a violated assertion so a false witness reaches the real prover.
    fn assignment(&self, claim: &Assignment<Bn254Fr>) -> Result<ProverInputs, SystemError> {
        let evaluation = self
            .circuit
            .evaluate_unchecked(claim)
            .map_err(|e| SystemError::Failed(format!("{e:?}")))?;
        let z: Vec<Fr> =
            self.r1cs.assignment(&evaluation.values).iter().map(|v| v.inner()).collect();
        let public: Vec<Fr> = claim.public.iter().map(|v| v.inner()).collect();
        let secrets: Vec<Fr> = claim.private.iter().map(|v| v.inner()).collect();
        Ok(ProverInputs { z, public, secrets })
    }

    /// Proves one assignment, true or not, with fresh prover randomness.
    fn prove_assigned(
        &self,
        claim: &Assignment<Bn254Fr>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let inputs = self.assignment(claim)?;
        let mut rng = os_rng()?;
        control.checkpoint()?;
        let proof = prover::prove(&self.pk, &self.r1cs, &inputs.z, &mut rng)
            .ok_or_else(|| SystemError::Failed("prover failed".into()))?;
        Ok(Proven {
            proof: proof.to_bytes(),
            public: inputs.public.into_iter().map(fr_to_canonical_le).collect(),
            secrets: inputs.secrets.into_iter().map(fr_to_canonical_le).collect(),
        })
    }
}

/// Canonical little-endian scalars as circuit values; anything else is refused, not reduced.
fn decode_all(values: &[FieldBytes]) -> Result<Vec<Bn254Fr>, SystemError> {
    values
        .iter()
        .map(|bytes| {
            fr_from_canonical_le(bytes).map(Bn254Fr).ok_or_else(|| {
                SystemError::Failed("an input is not a canonical BN254 scalar".into())
            })
        })
        .collect()
}

impl Prepared for PreparedBctv14 {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![
                ("constraints".into(), self.r1cs.num_constraints() as u64),
                ("variables".into(), self.r1cs.num_variables() as u64),
            ],
        }
    }

    fn setup_bytes(&self) -> Option<u64> {
        Some(setup_size(&self.pk, &self.vk) as u64)
    }

    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let claim = instance.example.instance::<Bn254Fr>(instance.kind, &instance.seed);
        self.prove_assigned(&claim, control)
    }

    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let claim = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_assigned(&claim, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        let mut public_fr = Vec::with_capacity(public.len());
        for bytes in public {
            match fr_from_canonical_le(bytes) {
                Some(value) => public_fr.push(value),
                None => {
                    return Ok(Verdict::Malformed(
                        "public input is not a canonical field element".into(),
                    ));
                }
            }
        }
        let Some(proof) = Proof::from_bytes(proof) else {
            return Ok(Verdict::Malformed(
                "proof bytes do not decode to eight group elements".into(),
            ));
        };
        Ok(if verifier::verify(&self.vk, &public_fr, &proof) {
            Verdict::Accepted
        } else {
            Verdict::Rejected
        })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        // The harness always passes a canonical element here, but reduce defensively so a bump
        // never panics on unexpected input.
        let element = fr_from_canonical_le(value)
            .unwrap_or_else(|| ark_ff::PrimeField::from_le_bytes_mod_order(value));
        Ok(fr_to_canonical_le(element + Fr::from(1u64)))
    }

    fn tamper_offset(&self, _proof: &[u8]) -> usize {
        // Byte 1 sits inside the x-coordinate of pi_A, the first proof element; the verifier reads
        // pi_A in the A knowledge-commitment, divisibility and same-coefficient checks, so flipping
        // it always changes a value a check depends on (or makes the point fail to decode).
        1
    }
}

/// Total compressed size of the proving and verification material.
fn setup_size(pk: &ProvingKey, vk: &VerifyingKey) -> usize {
    let mut total = 0;
    for kc in &pk.a_query {
        total += kc.g.compressed_size() + kc.h.compressed_size();
    }
    for kc in &pk.b_query {
        total += kc.g.compressed_size() + kc.h.compressed_size();
    }
    for kc in &pk.c_query {
        total += kc.g.compressed_size() + kc.h.compressed_size();
    }
    for point in &pk.h_query {
        total += point.compressed_size();
    }
    for point in &pk.k_query {
        total += point.compressed_size();
    }
    total += vk.alpha_a_g2.compressed_size()
        + vk.alpha_b_g1.compressed_size()
        + vk.alpha_c_g2.compressed_size()
        + vk.gamma_g2.compressed_size()
        + vk.gamma_beta_g1.compressed_size()
        + vk.gamma_beta_g2.compressed_size()
        + vk.rc_z_g2.compressed_size()
        + vk.ic_base.compressed_size();
    for point in &vk.ic_values {
        total += point.compressed_size();
    }
    total
}
