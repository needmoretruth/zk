//! The R1CS → QAP reduction over a multiplicative evaluation domain, exactly as libsnark does it.
//!
//! Each R1CS constraint `k` is bound to domain point `k`. The polynomials `A_i, B_i, C_i` are
//! defined in the Lagrange basis of the domain: `A_i(domain_k)` is the coefficient of variable `i`
//! in the left factor of constraint `k`. To keep the input-consistency argument sound, libsnark
//! appends `num_inputs + 1` extra constraints `input_i * 0 = 0` at domain points
//! `num_constraints .. num_constraints + num_inputs`, so the domain has size at least
//! `num_constraints + num_inputs + 1` (rounded up to a power of two). The QAP relation is: the R1CS
//! is satisfied iff `A(z) * B(z) - C(z)` is divisible by the vanishing polynomial `Z(z)` of the
//! domain, with quotient `H(z)`.
//!
//! Variable indices here are the assignment-vector indices of [`zk_circuit::lower::r1cs::R1cs`]:
//! index `0` is the constant one, `1..=num_inputs` the public inputs, the rest private.

use ark_bn254::Fr;
use ark_ff::{FftField, Field, Zero};
use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
use zk_circuit::lower::r1cs::R1cs;

use crate::field::Bn254Fr;

/// The QAP evaluated at the secret point `t`, the material the generator turns into a proving key.
pub struct QapAtPoint {
    /// `A_i(t)` for every variable `i` in `0..num_variables`.
    pub at: Vec<Fr>,
    /// `B_i(t)`.
    pub bt: Vec<Fr>,
    /// `C_i(t)`.
    pub ct: Vec<Fr>,
    /// `Z(t)`, the vanishing polynomial of the domain at `t`.
    pub zt: Fr,
    /// `1, t, t^2, ..., t^degree`, the powers the H-query encodes.
    pub powers_of_t: Vec<Fr>,
    /// The QAP degree, equal to the domain size.
    pub degree: usize,
    /// Number of variables including the constant one (`= at.len()`).
    pub num_variables: usize,
    /// Number of public inputs.
    pub num_inputs: usize,
}

/// The evaluation domain of size `>= num_constraints + num_inputs + 1`.
fn domain(r1cs: &R1cs<Bn254Fr>) -> Option<GeneralEvaluationDomain<Fr>> {
    let needed = r1cs.num_constraints() + r1cs.num_public_inputs() + 1;
    GeneralEvaluationDomain::<Fr>::new(needed)
}

/// Evaluates the QAP polynomials at `t`.
///
/// Mirrors libsnark's `r1cs_to_qap_instance_map_with_evaluation`: the Lagrange coefficients
/// `u_k = L_k(t)` are read off the domain, the input-consistency rows set `A_i(t) = u_{nc + i}`,
/// and every constraint adds `u_k * coeff` to the touched polynomial.
pub fn instance_at(r1cs: &R1cs<Bn254Fr>, t: Fr) -> Option<QapAtPoint> {
    let domain = domain(r1cs)?;
    let degree = domain.size();
    let nv = r1cs.num_variables();
    let l = r1cs.num_public_inputs();
    let nc = r1cs.num_constraints();

    let u = domain.evaluate_all_lagrange_coefficients(t);
    let mut at = vec![Fr::zero(); nv];
    let mut bt = vec![Fr::zero(); nv];
    let mut ct = vec![Fr::zero(); nv];

    // Input-consistency constraints `input_i * 0 = 0` at rows `nc + i`, i in 0..=l.
    for (i, slot) in at.iter_mut().enumerate().take(l + 1) {
        *slot += u[nc + i];
    }
    // All other constraints at rows 0..nc.
    for (k, row) in r1cs.constraints().iter().enumerate() {
        for &(index, coeff) in &row.a {
            at[index] += u[k] * coeff.inner();
        }
        for &(index, coeff) in &row.b {
            bt[index] += u[k] * coeff.inner();
        }
        for &(index, coeff) in &row.c {
            ct[index] += u[k] * coeff.inner();
        }
    }

    let zt = domain.evaluate_vanishing_polynomial(t);
    let mut powers_of_t = Vec::with_capacity(degree + 1);
    let mut power = Fr::ONE;
    for _ in 0..=degree {
        powers_of_t.push(power);
        power *= t;
    }

    Some(QapAtPoint { at, bt, ct, zt, powers_of_t, degree, num_variables: nv, num_inputs: l })
}

/// The coefficients of the quotient polynomial `H(z) = (A(z)B(z) - C(z)) / Z(z)`, including the
/// zero-knowledge blinding `d1, d2, d3`, exactly as libsnark's `r1cs_to_qap_witness_map`.
///
/// With `A = A_base + d1*Z`, `B = B_base + d2*Z`, `C = C_base + d3*Z`, the quotient splits as
/// `H = H_base + (d2*A_base + d1*B_base - d3) + d1*d2*Z`, where `H_base` comes from the unblinded
/// polynomials. `H_base` is found by evaluating `A_base, B_base, C_base` on a coset of the domain,
/// dividing pointwise by `Z` (constant on the coset), and interpolating back. The returned vector
/// has length `degree + 1` and never checks satisfiability, so a false witness yields a proof the
/// verifier rejects rather than a refusal here.
pub fn witness_h(r1cs: &R1cs<Bn254Fr>, z: &[Fr], d1: Fr, d2: Fr, d3: Fr) -> Option<Vec<Fr>> {
    let domain = domain(r1cs)?;
    let m = domain.size();
    let nc = r1cs.num_constraints();
    let l = r1cs.num_public_inputs();

    // Evaluations of A_base, B_base, C_base on the domain S.
    let mut a_evals = vec![Fr::zero(); m];
    let mut b_evals = vec![Fr::zero(); m];
    let mut c_evals = vec![Fr::zero(); m];
    // Input-consistency rows carry the variable values themselves (index 0 is the constant one).
    a_evals[nc..nc + l + 1].copy_from_slice(&z[..l + 1]);
    for (k, row) in r1cs.constraints().iter().enumerate() {
        a_evals[k] += dot(&row.a, z);
        b_evals[k] += dot(&row.b, z);
        c_evals[k] += dot(&row.c, z);
    }

    // Coefficients of the base polynomials.
    let a_coeffs = domain.ifft(&a_evals);
    let b_coeffs = domain.ifft(&b_evals);
    let c_coeffs = domain.ifft(&c_evals);

    // The zk-patch: (d2*A_base + d1*B_base - d3) + d1*d2*Z, in coefficient form, length m+1.
    let mut coefficients_for_h = vec![Fr::zero(); m + 1];
    for i in 0..m {
        coefficients_for_h[i] = d2 * a_coeffs[i] + d1 * b_coeffs[i];
    }
    coefficients_for_h[0] -= d3;
    // Z(z) = z^m - 1, so add d1*d2 at degree m and subtract it at degree 0.
    let d1d2 = d1 * d2;
    coefficients_for_h[m] += d1d2;
    coefficients_for_h[0] -= d1d2;

    // H_base on a coset: (A*B - C)/Z, with Z constant across the coset.
    let coset = domain.get_coset(Fr::GENERATOR)?;
    let a_coset = coset.fft(&a_coeffs);
    let b_coset = coset.fft(&b_coeffs);
    let c_coset = coset.fft(&c_coeffs);
    // Z at any coset point g*w^i equals g^m - 1; a single inverse divides them all.
    let z_on_coset = domain.evaluate_vanishing_polynomial(Fr::GENERATOR);
    let z_inv = z_on_coset.inverse()?;
    let mut h_coset = vec![Fr::zero(); m];
    for i in 0..m {
        h_coset[i] = (a_coset[i] * b_coset[i] - c_coset[i]) * z_inv;
    }
    let h_base = coset.ifft(&h_coset);
    for i in 0..m {
        coefficients_for_h[i] += h_base[i];
    }

    Some(coefficients_for_h)
}

/// `sum coeff * z[index]` over a sparse linear combination.
fn dot(lc: &[(usize, Bn254Fr)], z: &[Fr]) -> Fr {
    lc.iter().fold(Fr::zero(), |acc, &(index, coeff)| acc + coeff.inner() * z[index])
}
