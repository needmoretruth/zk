//! The simulator: with the cards known in advance and no witness, every round it makes passes the
//! verifier's checks — so a recording proves nothing to anyone who did not draw the cards.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_circuit::ZkField;
use zk_core::{ExampleId, InstanceKind, Verdict};
use zk_trio::{
    Card, Coins, Fp, Friend, Session, Simulator, Statement, TrioError, check_round, simulate,
};

fn plan() -> Vec<Card> {
    (0..4).flat_map(|_| Card::ALL).collect()
}

fn public(example: ExampleId) -> Vec<Fp> {
    example.instance::<Fp>(InstanceKind::Honest, &[12; 32]).public
}

#[test]
fn simulated_rounds_pass_every_check_without_a_witness() {
    for example in [ExampleId::Password, ExampleId::Factoring, ExampleId::Membership] {
        let statement = Statement::new(example).unwrap();
        let public = public(example);
        let rounds =
            simulate(&statement, public.clone(), &plan(), Coins::seeded([13; 32])).unwrap();
        assert_eq!(rounds.len(), 20);
        for round in &rounds {
            let checks =
                check_round(&statement, &public, &round.commitments, round.card, &round.opening);
            assert!(checks.iter().all(|check| check.passed), "{}: {checks:?}", example.id());
        }
    }
}

#[test]
fn a_simulated_recording_of_a_false_claim_passes_too() {
    let example = ExampleId::Factoring;
    let statement = Statement::new(example).unwrap();
    let mut false_public = public(example);
    false_public[0] = false_public[0].add(Fp::one());
    let plan = plan();
    let mut simulator =
        Simulator::new(&statement, false_public.clone(), plan.clone(), Coins::seeded([14; 32]))
            .unwrap();
    let mut session = Session::new(&statement, false_public, plan.len() as u32).unwrap();
    for card in plan {
        assert!(session.play_round_with(&mut simulator, card).unwrap().passed);
    }
    assert_eq!(session.verdict(), Some(Verdict::Accepted));
}

#[test]
fn the_simulator_is_stuck_when_a_different_card_is_drawn() {
    let example = ExampleId::OnePlusOne;
    let statement = Statement::new(example).unwrap();
    let plan = vec![Card::Peek(Friend::P2)];
    let mut simulator =
        Simulator::new(&statement, public(example), plan, Coins::seeded([15; 32])).unwrap();
    let mut session = Session::new(&statement, public(example), 1).unwrap();
    let error = session.play_round_with(&mut simulator, Card::DealerA).unwrap_err();
    let expected =
        TrioError::UnplannedCard { planned: Card::Peek(Friend::P2), drawn: Card::DealerA };
    assert_eq!(error, expected);
}
