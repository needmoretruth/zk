//! The verifier: the three knowledge-commitment checks, the same-coefficient check and the QAP
//! divisibility check, each a named function whose doc comment gives its pairing equation.
//!
//! Write `e(P, Q)` for the pairing of `P` in G1 and `Q` in G2, and `g2` for the G2 generator. The
//! verifier first rebuilds the input-dependent part of A from the public inputs (`accumulate_ic`),
//! then runs all five checks; a proof is accepted only if every one holds.

use ark_bn254::{Bn254, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::pairing::Pairing;
use ark_ec::{CurveGroup, PrimeGroup};

use crate::keys::VerifyingKey;
use crate::proof::Proof;

/// Rebuilds `PI(x) = rho_A (A_0(t) + sum_i x_i A_i(t)) G1`, the part of A that depends on the public
/// inputs, from the IC vector. `None` if the number of public inputs does not match the key.
pub fn accumulate_ic(vk: &VerifyingKey, public: &[Fr]) -> Option<G1Affine> {
    if public.len() != vk.ic_values.len() {
        return None;
    }
    let mut acc = G1Projective::from(vk.ic_base);
    for (value, base) in public.iter().zip(&vk.ic_values) {
        acc += *base * *value;
    }
    Some(acc.into_affine())
}

/// Knowledge-commitment check for A: `e(pi_A, alpha_A g2) = e(pi'_A, g2)`.
///
/// Forces `pi'_A = alpha_A * pi_A`, i.e. the prover built `pi_A` from the A-query and not from an
/// arbitrary group element.
pub fn check_kc_a(vk: &VerifyingKey, proof: &Proof) -> bool {
    Bn254::pairing(proof.a, vk.alpha_a_g2) == Bn254::pairing(proof.a_prime, g2_generator())
}

/// Knowledge-commitment check for B: `e(alpha_B g1, pi_B) = e(pi'_B, g2)`.
///
/// The B value lives in G2, so its `alpha` element `alpha_B g1` is paired on the left.
pub fn check_kc_b(vk: &VerifyingKey, proof: &Proof) -> bool {
    Bn254::pairing(vk.alpha_b_g1, proof.b) == Bn254::pairing(proof.b_prime, g2_generator())
}

/// Knowledge-commitment check for C: `e(pi_C, alpha_C g2) = e(pi'_C, g2)`.
pub fn check_kc_c(vk: &VerifyingKey, proof: &Proof) -> bool {
    Bn254::pairing(proof.c, vk.alpha_c_g2) == Bn254::pairing(proof.c_prime, g2_generator())
}

/// QAP divisibility check: `e(pi_A + PI(x), pi_B) = e(pi_H, rho_C Z(t) g2) * e(pi_C, g2)`.
///
/// This is the heart of the proof: it holds iff `A(t) B(t) - C(t) = H(t) Z(t)`, i.e. the assignment
/// satisfies every constraint. `acc` is `PI(x)` from [`accumulate_ic`].
pub fn check_qap_divisibility(vk: &VerifyingKey, proof: &Proof, acc: G1Affine) -> bool {
    let a_plus_input = (G1Projective::from(proof.a) + acc).into_affine();
    let lhs = Bn254::pairing(a_plus_input, proof.b);
    let rhs = Bn254::pairing(proof.h, vk.rc_z_g2) + Bn254::pairing(proof.c, g2_generator());
    lhs == rhs
}

/// Same-coefficient check:
/// `e(pi_K, gamma g2) = e(pi_A + PI(x) + pi_C, beta gamma g2) * e(beta gamma g1, pi_B)`.
///
/// Forces the A, B and C parts to use one and the same coefficient vector `w`, closing the gap the
/// three knowledge checks leave open.
pub fn check_same_coefficients(vk: &VerifyingKey, proof: &Proof, acc: G1Affine) -> bool {
    let a_input_c = (G1Projective::from(proof.a) + acc + proof.c).into_affine();
    let lhs = Bn254::pairing(proof.k, vk.gamma_g2);
    let rhs =
        Bn254::pairing(a_input_c, vk.gamma_beta_g2) + Bn254::pairing(vk.gamma_beta_g1, proof.b);
    lhs == rhs
}

/// Runs every check; accepts only if all five hold and the public inputs match the key.
pub fn verify(vk: &VerifyingKey, public: &[Fr], proof: &Proof) -> bool {
    let Some(acc) = accumulate_ic(vk, public) else {
        return false;
    };
    check_kc_a(vk, proof)
        && check_kc_b(vk, proof)
        && check_kc_c(vk, proof)
        && check_qap_divisibility(vk, proof, acc)
        && check_same_coefficients(vk, proof, acc)
}

/// The G2 generator, the right-hand argument of the "unshifted" pairings.
fn g2_generator() -> G2Affine {
    G2Projective::generator().into_affine()
}
