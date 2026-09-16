//! The prover: turn a satisfying assignment into a proof, with the zero-knowledge blinding.
//!
//! Given the proving key and the full assignment vector `z` (index 0 is the constant one), the
//! prover draws fresh `d1, d2, d3`, computes the quotient polynomial `H`, and forms each proof
//! element as a multi-scalar multiplication over the corresponding query. It never checks that the
//! assignment satisfies the circuit: a false witness produces a proof the verifier will reject,
//! which is what the museum's dishonest-witness attack needs to show.

use ark_bn254::{Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{CurveGroup, VariableBaseMSM};
use ark_ff::UniformRand;
use ark_std::rand::rngs::StdRng;
use zk_circuit::lower::r1cs::R1cs;

use crate::field::Bn254Fr;
use crate::keys::{Kc, ProvingKey};
use crate::proof::Proof;
use crate::qap::witness_h;

/// Produces a proof for assignment `z` (length `num_variables`, `z[0] = 1`).
pub fn prove(pk: &ProvingKey, r1cs: &R1cs<Bn254Fr>, z: &[Fr], rng: &mut StdRng) -> Option<Proof> {
    let d1 = Fr::rand(rng);
    let d2 = Fr::rand(rng);
    let d3 = Fr::rand(rng);
    let h_coeffs = witness_h(r1cs, z, d1, d2, d3)?;

    let nv = pk.num_variables;
    // The scalars for the variable range are z[1..nv]; index 0 (the one) and the blinding element
    // at index nv are added separately, matching libsnark's prover.
    let vars = &z[1..nv];

    // A knowledge commitment: value part g and alpha part h, both in G1.
    let a = combine_g1(&pk.a_query, vars, d1, nv);
    let c = combine_g1(&pk.c_query, vars, d3, nv);
    let b = combine_g2(&pk.b_query, vars, d2, nv);

    // H = sum_j h_coeffs[j] * (t^j G1).
    let h = G1Projective::msm(&pk.h_query, &h_coeffs).ok()?.into_affine();

    // K = k[0] + d1 k[nv] + d2 k[nv+1] + d3 k[nv+2] + sum_i z[i] k[i].
    let mut k = G1Projective::msm(&pk.k_query[1..nv], vars).ok()?;
    k += pk.k_query[0];
    k += pk.k_query[nv] * d1;
    k += pk.k_query[nv + 1] * d2;
    k += pk.k_query[nv + 2] * d3;
    let k = k.into_affine();

    Some(Proof { a: a.g, a_prime: a.h, b: b.g, b_prime: b.h, c: c.g, c_prime: c.h, k, h })
}

/// `query[0] + blind * query[nv] + sum_i vars[i] * query[1 + i]`, done for both `g` and `h`.
fn combine_g1(query: &[Kc<G1Affine>], vars: &[Fr], blind: Fr, nv: usize) -> Kc<G1Affine> {
    let g_bases: Vec<G1Affine> = query[1..nv].iter().map(|kc| kc.g).collect();
    let h_bases: Vec<G1Affine> = query[1..nv].iter().map(|kc| kc.h).collect();
    let mut g = msm_g1(&g_bases, vars);
    let mut h = msm_g1(&h_bases, vars);
    g += query[0].g;
    h += query[0].h;
    g += query[nv].g * blind;
    h += query[nv].h * blind;
    Kc { g: g.into_affine(), h: h.into_affine() }
}

/// The same combination, but the value part lives in G2 and the alpha part in G1.
fn combine_g2(query: &[Kc<G2Affine>], vars: &[Fr], blind: Fr, nv: usize) -> Kc<G2Affine> {
    let g_bases: Vec<G2Affine> = query[1..nv].iter().map(|kc| kc.g).collect();
    let h_bases: Vec<G1Affine> = query[1..nv].iter().map(|kc| kc.h).collect();
    let mut g = G2Projective::msm(&g_bases, vars).unwrap_or_default();
    let mut h = msm_g1(&h_bases, vars);
    g += query[0].g;
    h += query[0].h;
    g += query[nv].g * blind;
    h += query[nv].h * blind;
    Kc { g: g.into_affine(), h: h.into_affine() }
}

/// Multi-scalar multiplication in G1; an empty query is the identity. The length always matches by
/// construction, so the `Result` cannot be `Err` and defaulting to the identity is safe.
fn msm_g1(bases: &[G1Affine], scalars: &[Fr]) -> G1Projective {
    G1Projective::msm(bases, scalars).unwrap_or_default()
}
