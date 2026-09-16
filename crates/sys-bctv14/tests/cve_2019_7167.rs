//! CVE-2019-7167 (Gabizon, ePrint 2019/119): with the redundant `pi'_A` input elements published,
//! a valid proof for one public input is rewritten into a valid proof for any other. This shows the
//! forgery accepted against the flawed keys and impossible from the corrected keys.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ark_bn254::Fr;
use ark_ff::One;
use ark_std::rand::SeedableRng;
use ark_std::rand::rngs::StdRng;
use sys_bctv14::cve_2019_7167::{forge, forge_without_redundant_elements, generate_flawed};
use sys_bctv14::field::Bn254Fr;
use sys_bctv14::{prover, verifier};
use zk_circuit::lower::r1cs::R1cs;
use zk_core::{ExampleId, InstanceKind};

const SEED: [u8; 32] = [7; 32];

/// Sets up one-plus-one, makes an honest proof for the true envelope, and returns everything the
/// forgery needs.
fn honest_setup() -> (
    R1cs<Bn254Fr>,
    sys_bctv14::keys::VerifyingKey,
    sys_bctv14::cve_2019_7167::FlawExtras,
    sys_bctv14::Proof,
    Vec<Fr>,
) {
    let circuit = ExampleId::OnePlusOne.circuit::<Bn254Fr>().unwrap();
    let r1cs = R1cs::from_circuit(&circuit);
    let mut rng = StdRng::from_seed(SEED);
    let (pk, vk, flaw) = generate_flawed(&r1cs, &mut rng).unwrap();

    let claim = ExampleId::OnePlusOne.instance::<Bn254Fr>(InstanceKind::Honest, &SEED);
    let wires = circuit.evaluate(&claim).unwrap();
    let z: Vec<Fr> = r1cs.assignment(&wires).iter().map(|v| v.inner()).collect();
    let x_true: Vec<Fr> = claim.public.iter().map(|v| v.inner()).collect();

    let honest = prover::prove(&pk, &r1cs, &z, &mut rng).unwrap();
    assert!(verifier::verify(&vk, &x_true, &honest), "the honest proof must verify for its input");
    (r1cs, vk, flaw, honest, x_true)
}

#[test]
fn forgery_is_accepted_against_the_flawed_keys() {
    let (_r1cs, vk, flaw, honest, x_true) = honest_setup();
    // Any different public input is a false statement: no witness for it is known.
    let x_false: Vec<Fr> = x_true.iter().map(|v| *v + Fr::one()).collect();

    // The honest proof alone does not verify for the false statement.
    assert!(!verifier::verify(&vk, &x_false, &honest));

    // With the redundant elements, the proof is rewritten and the ordinary verifier accepts it.
    let forged = forge(&flaw, &vk, &honest, &x_true, &x_false).unwrap();
    assert!(
        verifier::verify(&vk, &x_false, &forged),
        "the forgery must be accepted for the false statement"
    );
}

#[test]
fn the_forgery_cannot_be_built_from_the_corrected_keys() {
    let (_r1cs, vk, _flaw, honest, x_true) = honest_setup();
    let x_false: Vec<Fr> = x_true.iter().map(|v| *v + Fr::one()).collect();

    // Adjusting only pi_A (all the corrected key exposes) leaves the A knowledge-check unsatisfiable,
    // so the ordinary verifier rejects it.
    let attempt = forge_without_redundant_elements(&vk, &honest, &x_true, &x_false).unwrap();
    assert!(
        !verifier::verify(&vk, &x_false, &attempt),
        "without the redundant pi'_A elements the forgery is rejected"
    );
}
