//! The live conversation: an honest prover passes every round, and each round reports the card,
//! what was opened and every check a screen needs to draw it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use zk_core::{Control, ExampleId, InstanceKind, Verdict};
use zk_trio::{
    Card, CheckKind, Coins, Fp, Friend, HonestProver, INTERACTIVE_ROUNDS, Opened, RoundProver,
    Session, Statement, TrioError, play,
};

fn prover(statement: &Statement, example: ExampleId) -> HonestProver<'_> {
    let assignment = example.instance::<Fp>(InstanceKind::Honest, &[9; 32]);
    let claim = statement.claim(&assignment).unwrap();
    HonestProver::new(statement, claim, Coins::seeded([10; 32])).unwrap()
}

#[test]
fn an_honest_prover_passes_all_55_rounds() {
    let example = ExampleId::Membership;
    let statement = Statement::new(example).unwrap();
    let mut prover = prover(&statement, example);
    let public = example.instance::<Fp>(InstanceKind::Honest, &[9; 32]).public;
    let mut cards = Coins::seeded([11; 32]);
    let summary =
        play(&statement, public, &mut prover, INTERACTIVE_ROUNDS, &mut cards, &Control::new())
            .unwrap();
    assert_eq!(summary.verdict, Verdict::Accepted);
    assert_eq!(summary.caught_at, None);
    assert_eq!(summary.events.len(), 55);
    let rounds: Vec<u32> = summary.events.iter().map(|event| event.round).collect();
    assert_eq!(rounds, (1..=55).collect::<Vec<_>>());
    assert!(summary.events.iter().all(|event| event.passed && event.of == 55));
    for card in Card::ALL {
        assert!(summary.events.iter().any(|event| event.card == card), "{card:?} never drawn");
    }
}

#[test]
fn a_dealer_round_opens_every_card_seed_and_checks_every_card() {
    let example = ExampleId::Age;
    let statement = Statement::new(example).unwrap();
    let mut prover = prover(&statement, example);
    let public = example.instance::<Fp>(InstanceKind::Honest, &[9; 32]).public;
    let mut session = Session::new(&statement, public, 2).unwrap();
    let event = session.play_round_with(&mut prover, Card::DealerB).unwrap();
    assert_eq!(event.hidden, None);
    let seeds = Friend::ALL.map(Opened::CardSeed);
    assert_eq!(event.opened, [&seeds[..], &[Opened::CardCorrection]].concat());
    let kinds: Vec<_> = event.checks.iter().map(|check| check.kind).collect();
    let expected = [
        CheckKind::OpeningFitsCard,
        CheckKind::CardsMultiply,
        CheckKind::CardCommitment(Friend::P1),
        CheckKind::CardCommitment(Friend::P2),
        CheckKind::CardCommitment(Friend::P3),
    ];
    assert_eq!(kinds, expected);
    assert!(event.passed);
    assert!(!session.is_over());
    assert_eq!(session.verdict(), None);
}

#[test]
fn a_peek_round_hides_one_friend_and_rechecks_all_three_views() {
    let example = ExampleId::Sudoku;
    let statement = Statement::new(example).unwrap();
    let mut prover = prover(&statement, example);
    let public = example.instance::<Fp>(InstanceKind::Honest, &[9; 32]).public;
    let mut session = Session::new(&statement, public, 1).unwrap();
    let event = session.play_round_with(&mut prover, Card::Peek(Friend::P1)).unwrap();
    assert_eq!(event.hidden, Some(Friend::P1));
    let opened = [
        Opened::CardSeed(Friend::P2),
        Opened::InputSeed(Friend::P2),
        Opened::CardSeed(Friend::P3),
        Opened::InputSeed(Friend::P3),
        Opened::CardCorrection,
        Opened::InputCorrection,
        Opened::ViewKey(Friend::P1),
        Opened::Broadcasts(Friend::P1),
    ];
    assert_eq!(event.opened, opened);
    let kinds: Vec<_> = event.checks.iter().map(|check| check.kind).collect();
    let expected = [
        CheckKind::OpeningFitsCard,
        CheckKind::CardCommitment(Friend::P2),
        CheckKind::CardCommitment(Friend::P3),
        CheckKind::ViewCommitment(Friend::P2),
        CheckKind::ViewCommitment(Friend::P3),
        CheckKind::ViewCommitment(Friend::P1),
        CheckKind::AssertionsVanish,
    ];
    assert_eq!(kinds, expected);
    assert!(event.passed);
    assert_eq!(session.verdict(), Some(Verdict::Accepted));
    let after = session.play_round_with(&mut prover, Card::DealerA);
    assert_eq!(after, Err(TrioError::SessionOver));
}

#[test]
fn a_prover_cannot_open_a_round_it_never_committed() {
    let example = ExampleId::OnePlusOne;
    let statement = Statement::new(example).unwrap();
    let mut prover = prover(&statement, example);
    assert_eq!(prover.respond(Card::DealerA), Err(TrioError::NothingCommitted));
}

#[test]
fn a_session_refuses_the_wrong_number_of_public_inputs() {
    let statement = Statement::new(ExampleId::OnePlusOne).unwrap();
    let error = Session::new(&statement, Vec::new(), 1).unwrap_err();
    assert_eq!(error, TrioError::PublicInputCount { expected: 1, got: 0 });
}
