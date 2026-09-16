//! Part A: bellman's ordinary verifier accepts a proof that the envelope holds 3, forged from the
//! toxic waste alone, and only from the right waste.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use bellman::groth16::{prepare_verifying_key, verify_proof};
use bls12_381::Scalar;
use ff::Field;
use rand_core::OsRng;
use zk_ceremony::toxic::{ForgeError, ToxicWaste, false_claim, forge, keys};

const SEED: [u8; 32] = [7; 32];

#[test]
fn the_toxic_waste_forges_a_proof_that_bellman_accepts_for_the_answer_three() {
    let waste = ToxicWaste::sample();
    let started = Instant::now();
    let parameters = keys(&waste).unwrap();
    let keygen = started.elapsed();
    let claim = false_claim(&SEED);
    let started = Instant::now();
    let proof = forge(&parameters.vk, &waste, &claim).unwrap();
    let forging = started.elapsed();
    let verifying_key = prepare_verifying_key(&parameters.vk);
    let started = Instant::now();
    let verdict = verify_proof(&verifying_key, &proof, &claim);
    let verifying = started.elapsed();
    println!("keys {keygen:?} · forge {forging:?} · bellman verify {verifying:?}");
    assert!(verdict.is_ok(), "{verdict:?}");
}

#[test]
fn a_proof_forged_with_a_wrong_delta_is_rejected() {
    let waste = ToxicWaste::sample();
    let parameters = keys(&waste).unwrap();
    let claim = false_claim(&SEED);
    let guessed = ToxicWaste { delta: Scalar::random(OsRng), ..waste };
    let proof = forge(&parameters.vk, &guessed, &claim).unwrap();
    let verdict = verify_proof(&prepare_verifying_key(&parameters.vk), &proof, &claim);
    assert!(verdict.is_err());
}

#[test]
fn a_claim_with_the_wrong_number_of_inputs_is_refused() {
    let waste = ToxicWaste::sample();
    let parameters = keys(&waste).unwrap();
    let error = forge(&parameters.vk, &waste, &[]).unwrap_err();
    assert_eq!(error, ForgeError::InputCount { expected: 1, given: 0 });
}
