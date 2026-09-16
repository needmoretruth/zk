//! Part A: whoever keeps a Groth16 setup's secret numbers can prove a false statement.
//!
//! bellman's verifier accepts `(A, B, C)` for public inputs `x` when
//! `e(A, B) = e([α]G1, [β]G2) · e(Σ xᵢ·ICᵢ, [γ]G2) · e(C, [δ]G2)`, with `x₀ = 1`. Write every point
//! as a multiple of the generators and the equation is one line of field arithmetic:
//! `ab = αβ + γ·I + δ·c`, where `I` is the discrete logarithm of `Σ xᵢ·ICᵢ`. Pick any `a` and `b`
//! and solve for `c`. Nobody knows `I`, but nobody needs it: `[γ·I]G1 = γ·Σ xᵢ·ICᵢ`, so
//! `C = δ⁻¹·(ab·G1 − β·[α]G1 − γ·Σ xᵢ·ICᵢ)` needs only β, γ and δ. The circuit, the witness and the
//! truth of the statement never enter.

use bellman::SynthesisError;
use bellman::groth16::{Parameters, Proof, VerifyingKey, generate_parameters};
use bls12_381::{Bls12, G1Projective, G2Projective, Scalar};
use ff::Field;
use group::Curve;
use rand_core::OsRng;
use zk_circuit::CircuitError;
use zk_circuit::lower::r1cs::R1cs;
use zk_examples::{ExampleId, InstanceKind};

use crate::field::CircuitField;
use crate::synthesis::Synthesis;

/// The five secret numbers of one Groth16 setup.
///
/// bellman's `generate_random_parameters` samples these, builds the keys and drops them. Here they
/// are kept on purpose and in the open, to show what a copy of them is worth. The standard
/// generators of G1 and G2 are used with them; bellman's random setup also randomises the
/// generators, which changes nothing about the forgery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToxicWaste {
    /// α: with β it fixes the target `e([α]G1, [β]G2)` every proof must reach.
    pub alpha: Scalar,
    /// β: the forger subtracts `β·[α]G1` to cancel that target.
    pub beta: Scalar,
    /// γ: divides the public-input points `ICᵢ`; the forger multiplies it back.
    pub gamma: Scalar,
    /// δ: divides the prover's private part; the forger divides by it to build `C`.
    pub delta: Scalar,
    /// τ: the secret point where the circuit's polynomials are evaluated; key generation needs
    /// it, the forger does not.
    pub tau: Scalar,
}

impl ToxicWaste {
    /// Five fresh numbers from the operating system's random source, as a real setup draws them.
    pub fn sample() -> Self {
        let draw = || Scalar::random(OsRng);
        Self { alpha: draw(), beta: draw(), gamma: draw(), delta: draw(), tau: draw() }
    }
}

/// Why keys could not be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyError {
    /// The `one-plus-one` circuit could not be built over BLS12-381's scalar field.
    Circuit(CircuitError),
    /// bellman's key generator refused, for example because γ or δ is zero and has no inverse;
    /// the text is bellman's own.
    Bellman(String),
}

/// Why a proof could not be forged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeError {
    /// The verifying key has a point for a different number of public inputs.
    InputCount {
        /// Public inputs the verifying key expects.
        expected: usize,
        /// Public inputs given.
        given: usize,
    },
    /// δ is zero, so it has no inverse; no real setup has such a key.
    ZeroDelta,
}

/// Groth16 keys for `one-plus-one`, made by bellman's `generate_parameters` from exactly `waste`.
///
/// These are ordinary keys: the proving key could prove the true claim, and the verifying key is
/// the one every verifier would be given.
pub fn keys(waste: &ToxicWaste) -> Result<Parameters<Bls12>, KeyError> {
    let circuit = ExampleId::OnePlusOne.circuit::<CircuitField>().map_err(KeyError::Circuit)?;
    let r1cs = R1cs::from_circuit(&circuit);
    generate_parameters::<Bls12, _>(
        Synthesis::new(&r1cs),
        G1Projective::generator(),
        G2Projective::generator(),
        waste.alpha,
        waste.beta,
        waste.gamma,
        waste.delta,
        waste.tau,
    )
    .map_err(|refusal: SynthesisError| KeyError::Bellman(refusal.to_string()))
}

/// The public input of the false claim: the envelope of `one-plus-one`'s dishonest instance, the
/// answer 3 sealed with a salt derived from `seed`, exactly as a run of the exhibit builds it.
pub fn false_claim(seed: &[u8; 32]) -> Vec<Scalar> {
    let claim = ExampleId::OnePlusOne.instance::<CircuitField>(InstanceKind::Dishonest, seed);
    claim.public.into_iter().map(|value| value.0).collect()
}

/// A proof for `public_inputs` under `vk`, made from the toxic waste alone.
///
/// `A = a·G1`, `B = b·G2` for fresh random `a`, `b`, and
/// `C = δ⁻¹·(ab·G1 − β·[α]G1 − γ·Σ xᵢ·ICᵢ)` with `x₀ = 1`. The waste is an argument because there
/// is no other way to compute `C`: with keys whose waste was discarded, this function cannot be
/// called. α and τ are not read.
pub fn forge(
    vk: &VerifyingKey<Bls12>,
    waste: &ToxicWaste,
    public_inputs: &[Scalar],
) -> Result<Proof<Bls12>, ForgeError> {
    if public_inputs.len() + 1 != vk.ic.len() {
        let expected = vk.ic.len().saturating_sub(1);
        return Err(ForgeError::InputCount { expected, given: public_inputs.len() });
    }
    let delta_inverse =
        Option::<Scalar>::from(waste.delta.invert()).ok_or(ForgeError::ZeroDelta)?;
    let (a, b) = (Scalar::random(OsRng), Scalar::random(OsRng));
    let inputs = core::iter::once(&Scalar::ONE)
        .chain(public_inputs)
        .zip(&vk.ic)
        .fold(G1Projective::identity(), |sum, (x, point)| sum + point * x);
    let c = (G1Projective::generator() * (a * b) - vk.alpha_g1 * waste.beta - inputs * waste.gamma)
        * delta_inverse;
    Ok(Proof {
        a: (G1Projective::generator() * a).to_affine(),
        b: (G2Projective::generator() * b).to_affine(),
        c: c.to_affine(),
    })
}
