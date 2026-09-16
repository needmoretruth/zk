//! The other ways of filming: the jealous reporter's edit, prior agreement, the apartment building.

use zk_cave::{
    Event, Floor, Prover, SCENES, Setting, WallOutcome, apartment_building, jealous_edit,
    prior_agreement,
};
use zk_core::ExampleId;

use crate::scripted::{HEADS, L, R, Scripted, TAILS};
use crate::{SEED, seeded, setting};

#[test]
fn the_jealous_edit_keeps_forty_successful_scenes() {
    let setting = setting();
    let edit = jealous_edit(&setting.wall, &Prover::double(None), 40, &mut seeded(3)).unwrap();
    let tape = edit.tape();
    assert_eq!(tape.scenes.len(), 40);
    assert!(tape.all_succeeded());
    assert_eq!(edit.kept.len(), 40);
    assert_eq!(edit.takes.len(), 40 + edit.discarded());
    assert!(edit.discarded() > 0);
    for take in &edit.takes {
        assert_eq!(edit.kept.contains(&take.at.take), take.succeeded(), "{take:?}");
    }
    let kept_as: Vec<u32> = edit
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Edited { kept_as, .. } => Some(*kept_as),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(kept_as, (1..=40).collect::<Vec<_>>());
}

#[test]
fn the_edit_counts_the_takes_it_threw_away() {
    let setting = setting();
    let mut script = Scripted::new(&[L, HEADS, R, HEADS, R, TAILS, L, TAILS]);
    let edit = jealous_edit(&setting.wall, &Prover::double(None), 2, &mut script).unwrap();
    assert_eq!(edit.takes.len(), 4);
    assert_eq!(edit.kept, vec![2, 4]);
    assert_eq!(edit.discarded(), 2);
    assert!(script.is_spent());
}

#[test]
fn prior_agreement_films_forty_perfect_scenes_without_a_coin() {
    let setting = setting();
    let played =
        prior_agreement(&setting.wall, &Prover::double(None), SCENES, &mut seeded(4)).unwrap();
    assert!(played.convinced());
    assert_eq!(played.scenes.len(), 40);
    assert!(played.scenes.iter().all(|s| s.coin.is_none() && s.entered == s.call));
    assert!(played.scenes.iter().all(|s| s.wall == WallOutcome::NotNeeded));
    assert!(!played.events.iter().any(|e| matches!(e, Event::CoinFlipped { .. })));
    let agreed = played.events.iter().find_map(|e| match e {
        Event::CallsAgreed { calls } => Some(calls.clone()),
        _ => None,
    });
    assert_eq!(agreed, Some(played.scenes.iter().map(|s| s.call).collect()));
}

#[test]
fn the_building_tests_forty_floors_in_one_round() {
    let floors: Vec<Floor> = Setting::floors(ExampleId::OnePlusOne, 40, &SEED)
        .unwrap()
        .into_iter()
        .map(|s| Floor { prover: Prover::mick_ali(s.words), wall: s.wall })
        .collect();
    let round = apartment_building(&floors, &mut seeded(5)).unwrap();
    assert!(round.convinced());
    assert_eq!(round.floors.len(), 40);
    assert!(round.floors.iter().all(|scene| scene.at.take == 1));
    let last_entry = round.events.iter().rposition(|e| matches!(e, Event::ProverEntered { .. }));
    let first_coin = round.events.iter().position(|e| matches!(e, Event::CoinFlipped { .. }));
    assert!(last_entry.unwrap() < first_coin.unwrap());
}

#[test]
fn a_double_in_the_building_fails_on_the_floors_called_on_the_other_side() {
    let setting = setting();
    let floor = Floor { wall: setting.wall, prover: Prover::double(None) };
    let floors = vec![floor.clone(), floor.clone(), floor];
    let mut script = Scripted::new(&[L, R, L, TAILS, TAILS, HEADS]);
    let round = apartment_building(&floors, &mut script).unwrap();
    assert_eq!(round.failed_floors(), vec![2, 3]);
    assert!(!round.convinced());
    assert!(script.is_spent());
}

#[test]
fn each_floor_of_a_salted_example_has_its_own_words() {
    let floors = Setting::floors(ExampleId::OnePlusOne, 2, &SEED).unwrap();
    assert!(floors[0].wall.whisper(&floors[0].words));
    assert!(!floors[1].wall.whisper(&floors[0].words));
}
