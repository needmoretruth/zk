//! Part C: over the final string an honest opening verifies, and colluders who kept every secret
//! open a commitment to a value the polynomial does not have; one secret short, they cannot.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use bls12_381::{G1Affine, Scalar};
use ff::Field;
use rand_core::OsRng;
use zk_ceremony::kzg::{Collusion, KzgError, commit, open, verify};
use zk_ceremony::tau::{DEFAULT_PARTICIPANTS, KeptSecret, POWERS, Participant, Srs};

/// A ceremony in which every participant kept their secret: the final string and the secrets.
fn ceremony_of_keepers() -> (Srs, Vec<KeptSecret>) {
    let mut srs = Srs::initial();
    let mut secrets = Vec::new();
    for _ in 0..DEFAULT_PARTICIPANTS {
        let (next, _, secret) = Participant::contribute_keeping_secret(&srs, &mut OsRng);
        srs = next;
        secrets.push(secret);
    }
    (srs, secrets)
}

/// `3 + 1·X + 4·X² + 1·X³ + 5·X⁴`, committed over `srs`.
fn committed(srs: &Srs) -> (Vec<Scalar>, G1Affine) {
    let coefficients: Vec<Scalar> = [3u64, 1, 4, 1, 5].into_iter().map(Scalar::from).collect();
    let commitment = commit(srs, &coefficients).unwrap();
    (coefficients, commitment)
}

#[test]
fn an_honest_opening_verifies_and_the_same_proof_for_another_value_does_not() {
    let (srs, _) = ceremony_of_keepers();
    let (coefficients, commitment) = committed(&srs);
    let z = Scalar::from(2u64);
    let (y, proof) = open(&srs, &coefficients, z).unwrap();
    assert_eq!(y, Scalar::from(3u64 + 2 + 4 * 4 + 8 + 5 * 16));
    assert!(verify(&srs, &commitment, z, y, &proof));
    assert!(!verify(&srs, &commitment, z, y + Scalar::ONE, &proof));
}

#[test]
fn colluders_holding_every_secret_open_a_commitment_to_a_wrong_value() {
    let (srs, secrets) = ceremony_of_keepers();
    let (coefficients, commitment) = committed(&srs);
    let z = Scalar::random(OsRng);
    let (y, _) = open(&srs, &coefficients, z).unwrap();
    let wrong = y + Scalar::from(1_000u64);
    let started = Instant::now();
    let forged = Collusion::from_secrets(&secrets).forge_open(&commitment, z, wrong).unwrap();
    println!("KZG forgery with {DEFAULT_PARTICIPANTS} kept secrets: {:?}", started.elapsed());
    assert!(verify(&srs, &commitment, z, wrong, &forged));
}

#[test]
fn with_one_secret_missing_the_forgery_fails() {
    let (srs, secrets) = ceremony_of_keepers();
    let (coefficients, commitment) = committed(&srs);
    let z = Scalar::random(OsRng);
    let (y, _) = open(&srs, &coefficients, z).unwrap();
    let wrong = y + Scalar::from(1_000u64);
    let partial = Collusion::from_secrets(&secrets[1..]);
    let forged = partial.forge_open(&commitment, z, wrong).unwrap();
    assert!(!verify(&srs, &commitment, z, wrong, &forged));
}

#[test]
fn a_polynomial_longer_than_the_string_is_refused() {
    let srs = Srs::initial();
    let too_long = vec![Scalar::ONE; POWERS + 1];
    let refusal = KzgError::TooManyCoefficients { given: POWERS + 1, powers: POWERS };
    assert_eq!(commit(&srs, &too_long), Err(refusal));
    assert_eq!(open(&srs, &too_long, Scalar::ONE), Err(refusal));
}
