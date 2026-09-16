//! What the protocol promises, one property per test: a broken multiplication is seen by one
//! challenge in three, two opened views fit another witness as well, and proofs are bound to what
//! they prove.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sys_zkboo::{
    Cheat, CheckKind, Claim, Fp, MODULUS, PROOF_ROUNDS, Party, Round, Statement, ZkBoo,
    check_round, hidden_view_for, proof,
};
use zk_circuit::{Assignment, Circuit, CircuitBuilder, LinearCombination, ZkField, check_field};
use zk_core::{Control, ExampleId, Instance, InstanceKind, ProofSystem, Verdict};

fn failed(checks: &[sys_zkboo::Check]) -> Vec<CheckKind> {
    checks.iter().filter(|check| !check.passed).map(|check| check.kind).collect()
}

/// `x · y = product`, with a label to vary: the label is part of the circuit digest.
fn product_circuit(label: &str) -> Circuit<Fp> {
    let mut builder = CircuitBuilder::<Fp>::new().unwrap();
    let product = builder.public_input("product");
    let x = builder.private_input("x");
    let y = builder.private_input("y");
    let z = builder.mul(x, y);
    builder.assert_zero(LinearCombination::from(z) - product, label);
    builder.finish().unwrap()
}

/// The false claim `5 · 6 = 35`, whose one assertion is off by `30 − 35 = −5`.
fn false_product() -> (Statement, Claim, Fp) {
    let statement = Statement::from_circuit("product", product_circuit("x times y"));
    let assignment =
        Assignment { public: vec![Fp::from_u64(35)], private: [5, 6].map(Fp::from_u64).to_vec() };
    let claim = statement.claim(&assignment).unwrap();
    (statement, claim, Fp::from_u64(5))
}

#[test]
fn a_broken_multiplication_is_seen_only_by_the_challenge_that_recomputes_it() {
    let (statement, claim, missing) = false_product();
    for cheater in Party::ALL {
        let cheat = Cheat { party: cheater, multiplication: 0, shift: missing };
        let cheating = Round::commit(&statement, &claim, Some(cheat)).unwrap();
        let plain = Round::commit(&statement, &claim, None).unwrap();
        for challenge in Party::ALL {
            let open = |round: &Round| {
                let first = round.first_message();
                check_round(&statement, &claim.public, first, challenge, &round.open(challenge))
            };
            // Without the cheat the false claim's shares cannot sum to zero, whatever is opened.
            assert_eq!(failed(&open(&plain)), [CheckKind::OutputsSumToZero]);
            let checks = open(&cheating);
            if challenge == cheater {
                assert_eq!(failed(&checks), [CheckKind::MultiplicationsRecomputed], "{checks:?}");
                let recomputed = checks.iter().find(|c| !c.passed).unwrap();
                assert_eq!(recomputed.first_failure, Some(0));
            } else {
                assert!(
                    failed(&checks).is_empty(),
                    "{cheater:?} escaped {challenge:?}? {checks:?}"
                );
            }
        }
    }
}

#[test]
fn a_cheating_proof_is_caught_in_about_a_third_of_its_rounds() {
    let (statement, claim, missing) = false_product();
    let control = Control::new();
    let cheat = Cheat { party: Party::P2, multiplication: 0, shift: missing };
    let bytes = proof::prove_with_cheat(&statement, &claim, cheat, &control).unwrap();
    let rounds = proof::parse(&statement, &claim.public, &bytes).unwrap();
    let caught: Vec<Party> = rounds
        .iter()
        .filter(|r| {
            !check_round(&statement, &claim.public, &r.first, r.challenge, &r.response)
                .iter()
                .all(|check| check.passed)
        })
        .map(|round| round.challenge)
        .collect();
    println!("caught in {} of {PROOF_ROUNDS} rounds", caught.len());
    assert!(caught.iter().all(|challenge| *challenge == Party::P2));
    assert_eq!(caught.len(), rounds.iter().filter(|r| r.challenge == Party::P2).count());
    // Each round is caught with probability 1/3: mean 45.7 of 137, standard deviation 5.5, and this
    // range spans 4.5 deviations either side.
    assert!((21..=70).contains(&caught.len()), "{}", caught.len());
    assert_eq!(
        proof::verify(&statement, &claim.public, &bytes, &control).unwrap(),
        Verdict::Rejected
    );
}

#[test]
fn two_opened_views_fit_a_different_secret_as_well() {
    let statement = Statement::new(ExampleId::Factoring).unwrap();
    let claim_of = |p: u64, q: u64| {
        let private = vec![p, q].into_iter().map(Fp::from_u64).collect();
        let assignment = Assignment { public: vec![Fp::from_u64(181 * 191)], private };
        statement.claim(&assignment).unwrap()
    };
    let (proven, swapped, trivial) =
        (claim_of(181, 191), claim_of(191, 181), claim_of(1, 181 * 191));
    let round = Round::commit(&statement, &proven, None).unwrap();
    let first = round.first_message();
    for challenge in Party::ALL {
        let response = round.open(challenge);
        assert!(
            failed(&check_round(&statement, &proven.public, first, challenge, &response))
                .is_empty()
        );
        let fit = |claim: &Claim| {
            let hidden = hidden_view_for(&statement, claim, challenge, &response).unwrap();
            let sent: Vec<Fp> =
                first.output_shares.iter().map(|s| s[hidden.party.index()]).collect();
            (hidden.output_shares == sent, hidden)
        };
        let ((proven_fits, proven_view), (swapped_fits, swapped_view)) =
            (fit(&proven), fit(&swapped));
        assert!(proven_fits && swapped_fits, "{challenge:?}");
        // Two different hidden views: the hidden share of p moves by 191 − 181.
        let moved = swapped_view.input_shares[0].sub(proven_view.input_shares[0]);
        assert_eq!(moved, Fp::from_u64(10));
        // The fit is not automatic: a claim that breaks the circuit does not fit.
        assert!(!fit(&trivial).0, "{challenge:?}");
    }
}

fn honest() -> Instance {
    Instance { example: ExampleId::OnePlusOne, kind: InstanceKind::Honest, seed: [18; 32] }
}

#[test]
fn bytes_after_the_proof_a_wrong_round_count_or_a_huge_public_input_are_malformed() {
    let control = Control::new();
    let mut prepared = ZkBoo.prepare(ExampleId::OnePlusOne, &control).unwrap();
    let proven = prepared.prove(&honest(), &control).unwrap();
    let mut longer = proven.proof.clone();
    longer.push(0);
    let mut fewer = proven.proof.clone();
    fewer[0] = 136;
    for bytes in [longer, fewer, Vec::new()] {
        let verdict = prepared.verify(&proven.public, &bytes, &control).unwrap();
        assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    }
    let too_big = MODULUS.to_le_bytes().to_vec();
    let verdict =
        prepared.verify(core::slice::from_ref(&too_big), &proven.proof, &control).unwrap();
    assert!(matches!(verdict, Verdict::Malformed(_)), "{verdict:?}");
    assert!(prepared.bump_public(&too_big).is_err());
}

#[test]
fn a_proof_is_bound_to_the_example_id_and_to_the_gates() {
    let control = Control::new();
    let statement = Statement::from_circuit("product", product_circuit("x times y"));
    let assignment =
        Assignment { public: vec![Fp::from_u64(35)], private: [5, 7].map(Fp::from_u64).to_vec() };
    let claim = statement.claim(&assignment).unwrap();
    let bytes = proof::prove(&statement, &claim, &control).unwrap();
    let verdict = |statement: &Statement| proof::verify(statement, &claim.public, &bytes, &control);
    assert_eq!(verdict(&statement).unwrap(), Verdict::Accepted);
    let renamed = Statement::from_circuit("other", product_circuit("x times y"));
    assert_ne!(verdict(&renamed).unwrap(), Verdict::Accepted);
    let relabelled = Statement::from_circuit("product", product_circuit("y times x"));
    assert_ne!(verdict(&relabelled).unwrap(), Verdict::Accepted);
}

#[test]
fn a_caller_supplied_assignment_is_proved_without_being_checked() {
    let control = Control::new();
    let mut prepared = ZkBoo.prepare(ExampleId::OnePlusOne, &control).unwrap();
    for (kind, expected) in
        [(InstanceKind::Honest, Verdict::Accepted), (InstanceKind::Dishonest, Verdict::Rejected)]
    {
        let instance = Instance { example: ExampleId::OnePlusOne, kind, seed: [21; 32] };
        let sample = prepared.prove(&instance, &control).unwrap();
        let proven = prepared.prove_assignment(&sample.public, &sample.secrets, &control).unwrap();
        assert_eq!(proven.secrets, sample.secrets);
        assert_eq!(prepared.verify(&proven.public, &proven.proof, &control).unwrap(), expected);
    }
}

#[test]
fn the_field_is_accepted_by_the_circuit_layer() {
    check_field::<Fp>().unwrap();
    assert_eq!(Fp::from_u64(MODULUS), Fp::zero());
    assert_eq!(Fp::decode(&Fp::from_u64(7).to_le_bytes()), Ok(Fp::from_u64(7)));
}
