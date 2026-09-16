//! The cheating provers: each is caught on exactly the cards that can see its lie, and a lie
//! survives one round about three times in five.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_core::{Control, ExampleId, Verdict};
use zk_trio::{
    Card, Cheat, CheatingProver, CheckKind, Coins, Friend, INTERACTIVE_ROUNDS, RoundProver,
    Session, Statement, check_round, escape_odds, escape_probability, play_cheat,
};

fn failed(checks: &[zk_trio::Check]) -> Vec<CheckKind> {
    checks.iter().filter(|check| !check.passed).map(|check| check.kind).collect()
}

#[test]
fn a_rigged_card_is_caught_only_by_a_dealer_card() {
    let (cheat, cards) = (Cheat::BadCard, &mut Coins::seeded([1; 32]));
    let run = play_cheat(
        ExampleId::Password,
        cheat,
        INTERACTIVE_ROUNDS,
        Coins::seeded([2; 32]),
        cards,
        &Control::new(),
    )
    .unwrap();
    let rigged = run.claim.rigged.unwrap();
    let caught = run.summary.caught_at.expect("a rigged card survived 55 rounds");
    println!("rigged card caught in round {caught}: {:?}", run.claim.source);
    assert_eq!(run.summary.verdict, Verdict::Rejected);
    let (last, earlier) = run.summary.events.split_last().unwrap();
    assert!(earlier.iter().all(|event| event.passed && !event.card.is_dealer()));
    assert!(last.card.is_dealer());
    assert_eq!(failed(&last.checks), [CheckKind::CardsMultiply]);
    let multiply = last.checks.iter().find(|c| c.kind == CheckKind::CardsMultiply).unwrap();
    assert_eq!(multiply.first_failure, Some(rigged.multiplication));
}

#[test]
fn a_friend_who_computed_wrongly_is_caught_only_when_opened() {
    let liar = Friend::P2;
    let run = play_cheat(
        ExampleId::Factoring,
        Cheat::BadComputation(liar),
        INTERACTIVE_ROUNDS,
        Coins::seeded([3; 32]),
        &mut Coins::seeded([4; 32]),
        &Control::new(),
    )
    .unwrap();
    let caught = run.summary.caught_at.expect("a lying friend survived 55 rounds");
    println!("lying friend caught in round {caught}");
    let (last, earlier) = run.summary.events.split_last().unwrap();
    for event in earlier {
        assert!(event.passed);
        assert!(event.card.is_dealer() || event.hidden == Some(liar), "{:?}", event.card);
    }
    assert!(last.hidden.is_some_and(|hidden| hidden != liar), "{:?}", last.card);
    // Opened, the liar is re-run honestly: its commitment no longer matches and the false claim's
    // assertions no longer vanish.
    assert_eq!(
        failed(&last.checks),
        [CheckKind::ViewCommitment(liar), CheckKind::AssertionsVanish]
    );
}

#[test]
fn every_example_has_a_false_claim_one_rigged_card_makes_add_up() {
    for example in ExampleId::ALL {
        let statement = Statement::new(example).unwrap();
        let mut prover =
            CheatingProver::new(&statement, example, Cheat::BadCard, Coins::seeded([5; 32]))
                .unwrap();
        let claim = prover.false_claim().clone();
        println!("{}: {:?}, {:?}", example.id(), claim.source, claim.rigged);
        let commitments = prover.commit().unwrap();
        let card = Card::Peek(Friend::P3);
        let opening = prover.respond(card).unwrap();
        let checks = check_round(&statement, &claim.public, &commitments, card, &opening);
        assert!(checks.iter().all(|check| check.passed), "{}: {checks:?}", example.id());
    }
}

/// 2000 single rounds each: the escape rate's standard deviation is sqrt(0.24 / 2000) ≈ 0.011,
/// so ±0.04 is more than three and a half deviations.
#[test]
fn a_cheat_escapes_one_round_about_three_times_in_five() {
    const RUNS: u32 = 2000;
    let statement = Statement::new(ExampleId::OnePlusOne).unwrap();
    for cheat in [Cheat::BadCard, Cheat::BadComputation(Friend::P1)] {
        let mut cards = Coins::seeded([6; 32]);
        let mut prover =
            CheatingProver::new(&statement, ExampleId::OnePlusOne, cheat, Coins::seeded([7; 32]))
                .unwrap();
        let public = prover.false_claim().public.clone();
        let mut escaped = 0;
        for _ in 0..RUNS {
            let mut session = Session::new(&statement, public.clone(), 1).unwrap();
            session.play_round(&mut prover, &mut cards).unwrap();
            if session.verdict() == Some(Verdict::Accepted) {
                escaped += 1;
            }
        }
        let rate = f64::from(escaped) / f64::from(RUNS);
        println!("{cheat:?} escaped one round in {rate:.3} of {RUNS} runs");
        assert!((rate - 0.6).abs() < 0.04, "{cheat:?}: {rate}");
    }
}

#[test]
fn the_first_draft_forgery_is_caught_on_every_peek_card() {
    let statement = Statement::new(ExampleId::Factoring).unwrap();
    let cheat = Cheat::RewriteHiddenShares;
    let mut prover =
        CheatingProver::new(&statement, ExampleId::Factoring, cheat, Coins::seeded([8; 32]))
            .unwrap();
    let public = prover.false_claim().public.clone();
    for _ in 0..3 {
        for card in Card::ALL {
            let mut session = Session::new(&statement, public.clone(), 1).unwrap();
            let event = session.play_round_with(&mut prover, card).unwrap();
            match card.hidden() {
                None => assert!(event.passed, "a dealer card never looks at the assertions"),
                Some(hidden) => {
                    // The rewrite does make every assertion sum to zero: the draft, which had no
                    // way to recompute the hidden friend's V, would have accepted this round.
                    assert_eq!(failed(&event.checks), [CheckKind::ViewCommitment(hidden)]);
                }
            }
        }
    }
}

#[test]
fn the_escape_probability_is_three_fifths_per_round() {
    assert_eq!(escape_probability(0), 1.0);
    assert!((escape_probability(1) - 0.6).abs() < 1e-15);
    assert!((escape_probability(2) - 0.36).abs() < 1e-15);
    assert_eq!(escape_odds(3), Some((27, 125)));
    assert_eq!(escape_odds(55), Some((3u128.pow(55), 5u128.pow(55))));
    assert_eq!(escape_odds(56), None);
    assert!(escape_probability(55) < 2f64.powi(-40));
    assert!(escape_probability(109) < 2f64.powi(-80));
    assert!(escape_probability(108) >= 2f64.powi(-80));
}
