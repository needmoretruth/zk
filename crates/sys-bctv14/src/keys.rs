//! The proving and verification keys and the trusted-setup generator.
//!
//! Layout follows libsnark's `r1cs_ppzksnark`: the proving key holds the A/B/C knowledge-commitment
//! queries, the H-query (powers of `t`) and the same-coefficient K-query; the verification key holds
//! `alpha_A, alpha_B, alpha_C, gamma, beta*gamma, rC*Z(t)` and the input-consistency (IC) vector.
//! The toxic waste `t, alpha_A, alpha_B, alpha_C, rho_A, rho_B, beta, gamma` is sampled here and
//! dropped when this function returns; anyone who keeps it can forge proofs.

use ark_bn254::{Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::UniformRand;
use ark_std::rand::rngs::StdRng;
use zk_circuit::lower::r1cs::R1cs;

use crate::field::Bn254Fr;
use crate::qap::{QapAtPoint, instance_at};

/// A knowledge commitment: a base-group point `g` and its `alpha`-shifted twin `h`, so a pairing can
/// check the prover used the key's element and not an arbitrary group element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kc<G> {
    /// The value element.
    pub g: G,
    /// The `alpha`-shifted element (always in G1).
    pub h: G1Affine,
}

/// The proving key: everything the prover needs and nothing that reveals the witness.
pub struct ProvingKey {
    /// A-query `(rho_A A_i(t) G1, alpha_A rho_A A_i(t) G1)`, length `num_variables + 1`; the input
    /// prefix `0..=num_inputs` is zeroed (moved into the verification key's IC vector).
    pub a_query: Vec<Kc<G1Affine>>,
    /// B-query `(rho_B B_i(t) G2, alpha_B rho_B B_i(t) G1)`, length `num_variables + 1`.
    pub b_query: Vec<Kc<G2Affine>>,
    /// C-query `(rho_C C_i(t) G1, alpha_C rho_C C_i(t) G1)`, length `num_variables + 1`.
    pub c_query: Vec<Kc<G1Affine>>,
    /// H-query `t^j G1` for `j in 0..=degree`.
    pub h_query: Vec<G1Affine>,
    /// K-query `beta (rho_A A_i + rho_B B_i + rho_C C_i)(t) G1`, length `num_variables + 3`.
    pub k_query: Vec<G1Affine>,
    /// Variables including the constant one.
    pub num_variables: usize,
}

/// The verification key: a constant number of group elements plus one IC element per public input.
pub struct VerifyingKey {
    /// `alpha_A G2`.
    pub alpha_a_g2: G2Affine,
    /// `alpha_B G1`.
    pub alpha_b_g1: G1Affine,
    /// `alpha_C G2`.
    pub alpha_c_g2: G2Affine,
    /// `gamma G2`.
    pub gamma_g2: G2Affine,
    /// `beta gamma G1`.
    pub gamma_beta_g1: G1Affine,
    /// `beta gamma G2`.
    pub gamma_beta_g2: G2Affine,
    /// `rho_C Z(t) G2`, the target-polynomial element of the divisibility check.
    pub rc_z_g2: G2Affine,
    /// IC base `rho_A A_0(t) G1` (the constant-one term).
    pub ic_base: G1Affine,
    /// IC values `rho_A A_i(t) G1` for each public input `i`, in declaration order.
    pub ic_values: Vec<G1Affine>,
}

/// The eight secret field elements of the setup. Sampled, used, and dropped; kept in one struct only
/// so the flawed CVE generator can reuse the exact same derivation.
pub(crate) struct ToxicWaste {
    pub t: Fr,
    pub alpha_a: Fr,
    pub alpha_b: Fr,
    pub alpha_c: Fr,
    pub rho_a: Fr,
    pub rho_b: Fr,
    pub beta: Fr,
    pub gamma: Fr,
}

impl ToxicWaste {
    pub(crate) fn sample(rng: &mut StdRng) -> Self {
        Self {
            t: Fr::rand(rng),
            alpha_a: Fr::rand(rng),
            alpha_b: Fr::rand(rng),
            alpha_c: Fr::rand(rng),
            rho_a: Fr::rand(rng),
            rho_b: Fr::rand(rng),
            beta: Fr::rand(rng),
            gamma: Fr::rand(rng),
        }
    }
}

/// The IC coefficients `A_i(t)` for `i in 0..=num_inputs`, saved before the A-query prefix is
/// zeroed. The honest generator keeps only their `rho_A`-scaled forms in the verification key; the
/// CVE generator also exposes their `alpha_A rho_A`-scaled twins, which is the flaw.
pub(crate) struct SetupInternals {
    pub ic_coefficients: Vec<Fr>,
    pub rho_a: Fr,
    pub alpha_a: Fr,
}

/// Runs the trusted setup, returning both keys and (for the CVE module) the internals.
pub(crate) fn generate_with_internals(
    r1cs: &R1cs<Bn254Fr>,
    rng: &mut StdRng,
) -> Option<(ProvingKey, VerifyingKey, SetupInternals)> {
    let waste = ToxicWaste::sample(rng);
    let qap = instance_at(r1cs, waste.t)?;
    Some(build_keys(&qap, &waste))
}

/// The honest generator.
pub fn generate(r1cs: &R1cs<Bn254Fr>, rng: &mut StdRng) -> Option<(ProvingKey, VerifyingKey)> {
    generate_with_internals(r1cs, rng).map(|(pk, vk, _)| (pk, vk))
}

pub(crate) fn build_keys(
    qap: &QapAtPoint,
    waste: &ToxicWaste,
) -> (ProvingKey, VerifyingKey, SetupInternals) {
    let g1 = G1Projective::generator();
    let g2 = G2Projective::generator();
    let rho_c = waste.rho_a * waste.rho_b;
    let nv = qap.num_variables;
    let l = qap.num_inputs;

    // At/Bt/Ct with Z(t) appended at index nv (the zero-knowledge blinding element).
    let mut at: Vec<Fr> = qap.at.clone();
    let mut bt: Vec<Fr> = qap.bt.clone();
    let mut ct: Vec<Fr> = qap.ct.clone();
    at.push(qap.zt);
    bt.push(qap.zt);
    ct.push(qap.zt);

    // K-query coefficients: real variables first, then the three blinding terms.
    let mut kt: Vec<Fr> = Vec::with_capacity(nv + 3);
    for i in 0..nv {
        kt.push(waste.beta * (waste.rho_a * at[i] + waste.rho_b * bt[i] + rho_c * ct[i]));
    }
    kt.push(waste.beta * waste.rho_a * qap.zt);
    kt.push(waste.beta * waste.rho_b * qap.zt);
    kt.push(waste.beta * rho_c * qap.zt);

    // Move the A-query input prefix into the IC coefficients, then zero it.
    let mut ic_coefficients = Vec::with_capacity(l + 1);
    for slot in at.iter_mut().take(l + 1) {
        ic_coefficients.push(*slot);
        *slot = Fr::from(0u64);
    }

    let a_query = kc_g1(&at, waste.rho_a, waste.rho_a * waste.alpha_a, g1);
    let b_query = kc_g2(&bt, waste.rho_b, waste.rho_b * waste.alpha_b, g1, g2);
    let c_query = kc_g1(&ct, rho_c, rho_c * waste.alpha_c, g1);
    let h_query = scale_batch_g1(&qap.powers_of_t, Fr::from(1u64), g1);
    let k_query = scale_batch_g1(&kt, Fr::from(1u64), g1);

    let pk = ProvingKey { a_query, b_query, c_query, h_query, k_query, num_variables: nv };

    let ic_base = (g1 * (waste.rho_a * ic_coefficients[0])).into_affine();
    let ic_values: Vec<G1Affine> =
        (1..=l).map(|i| (g1 * (waste.rho_a * ic_coefficients[i])).into_affine()).collect();

    let vk = VerifyingKey {
        alpha_a_g2: (g2 * waste.alpha_a).into_affine(),
        alpha_b_g1: (g1 * waste.alpha_b).into_affine(),
        alpha_c_g2: (g2 * waste.alpha_c).into_affine(),
        gamma_g2: (g2 * waste.gamma).into_affine(),
        gamma_beta_g1: (g1 * (waste.gamma * waste.beta)).into_affine(),
        gamma_beta_g2: (g2 * (waste.gamma * waste.beta)).into_affine(),
        rc_z_g2: (g2 * (rho_c * qap.zt)).into_affine(),
        ic_base,
        ic_values,
    };

    let internals = SetupInternals { ic_coefficients, rho_a: waste.rho_a, alpha_a: waste.alpha_a };
    (pk, vk, internals)
}

/// `(scale * coeff G1, alpha_scale * coeff G1)` for each coefficient.
fn kc_g1(coeffs: &[Fr], scale: Fr, alpha_scale: Fr, g1: G1Projective) -> Vec<Kc<G1Affine>> {
    let g: Vec<G1Projective> = coeffs.iter().map(|c| g1 * (scale * c)).collect();
    let h: Vec<G1Projective> = coeffs.iter().map(|c| g1 * (alpha_scale * c)).collect();
    let g = G1Projective::normalize_batch(&g);
    let h = G1Projective::normalize_batch(&h);
    g.into_iter().zip(h).map(|(g, h)| Kc { g, h }).collect()
}

/// `(scale * coeff G2, alpha_scale * coeff G1)` for each coefficient.
fn kc_g2(
    coeffs: &[Fr],
    scale: Fr,
    alpha_scale: Fr,
    g1: G1Projective,
    g2: G2Projective,
) -> Vec<Kc<G2Affine>> {
    let g: Vec<G2Projective> = coeffs.iter().map(|c| g2 * (scale * c)).collect();
    let h: Vec<G1Projective> = coeffs.iter().map(|c| g1 * (alpha_scale * c)).collect();
    let g = G2Projective::normalize_batch(&g);
    let h = G1Projective::normalize_batch(&h);
    g.into_iter().zip(h).map(|(g, h)| Kc { g, h }).collect()
}

/// `scale * coeff G1` for each coefficient, batch-normalized.
fn scale_batch_g1(coeffs: &[Fr], scale: Fr, g1: G1Projective) -> Vec<G1Affine> {
    let points: Vec<G1Projective> = coeffs.iter().map(|c| g1 * (scale * c)).collect();
    G1Projective::normalize_batch(&points)
}
