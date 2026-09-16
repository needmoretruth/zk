//! The first network's demonstration: Mick convinces, the double is caught, and the tape shows only
//! the fork.

use zk_cave::{Event, MagicWords, Prover, SCENES, Setting, Side, WallOutcome, demonstration, odds};
use zk_core::ExampleId;

use crate::scripted::{HEADS, L, R, Scripted, TAILS};
use crate::{SEED, seeded, setting};

#[test]
fn mick_convinces_the_reporter_in_forty_scenes() {
    let setting = Setting::for_example(ExampleId::Sudoku, &SEED).unwrap();
    let mick = Prover::mick_ali(setting.words.clone());
    let played = demonstration(&setting.wall, &mick, SCENES, &mut seeded(1)).unwrap();
    assert!(played.convinced());
    assert_eq!(played.caught_at, None);
    assert_eq!(played.scenes.len(), 40);
    assert!(played.tape().all_succeeded());
    for scene in &played.scenes {
        assert_eq!(scene.wall_opened(), scene.entered != scene.call, "{scene:?}");
    }
    assert!(played.scenes.iter().any(|scene| scene.wall_opened()));
    assert!(played.scenes.iter().any(|scene| scene.wall == WallOutcome::NotNeeded));
}

#[test]
fn a_double_without_words_is_caught_at_the_first_scene_called_on_the_other_side() {
    let setting = setting();
    let double = Prover::double(None);
    let mut script = Scripted::new(&[R, HEADS, L, TAILS, L, HEADS]);
    let played = demonstration(&setting.wall, &double, SCENES, &mut script).unwrap();
    assert!(!played.convinced());
    assert_eq!(played.caught_at, Some(3));
    assert_eq!(played.scenes.len(), 3);
    let caught = played.scenes[2];
    assert_eq!(caught.wall, WallOutcome::NothingToWhisper);
    assert_eq!((caught.call, caught.exit), (Side::Right, Side::Left));
    assert!(script.is_spent());
}

#[test]
fn a_double_with_wrong_words_finds_the_wall_shut() {
    let setting = setting();
    let double = Prover::double(Some(setting.wrong_words.clone()));
    let mut script = Scripted::new(&[L, HEADS]);
    let played = demonstration(&setting.wall, &double, SCENES, &mut script).unwrap();
    assert_eq!(played.caught_at, Some(1));
    assert_eq!(played.scenes[0].wall, WallOutcome::StayedShut);
}

#[test]
fn the_right_words_open_every_example_wall_and_the_wrong_words_open_none() {
    for example in ExampleId::ALL {
        let setting = Setting::for_example(example, &SEED).unwrap();
        assert!(setting.wall.whisper(&setting.words), "{example:?}");
        assert!(!setting.wall.whisper(&setting.wrong_words), "{example:?}");
        assert!(!setting.wall.whisper(&MagicWords::new(Vec::new())), "{example:?}");
    }
}

#[test]
fn the_tape_does_not_depend_on_the_passage_mick_entered() {
    let setting = setting();
    let mick = Prover::mick_ali(setting.words.clone());
    let coins = [HEADS, TAILS, TAILS, HEADS];
    let script = |passage| coins.iter().flat_map(|coin| [passage, *coin]).collect::<Vec<_>>();
    let all_left = demonstration(&setting.wall, &mick, 4, &mut Scripted::new(&script(L))).unwrap();
    let all_right = demonstration(&setting.wall, &mick, 4, &mut Scripted::new(&script(R))).unwrap();
    assert_ne!(all_left.scenes, all_right.scenes);
    assert_eq!(all_left.tape().to_bytes(), all_right.tape().to_bytes());
}

#[test]
fn the_tape_is_what_the_camera_filmed_and_nothing_else() {
    let setting = setting();
    let mick = Prover::mick_ali(setting.words.clone());
    let played = demonstration(&setting.wall, &mick, SCENES, &mut seeded(2)).unwrap();
    let filmed: Vec<&Event> = played.events.iter().filter(|event| event.on_camera()).collect();
    let calls = filmed.iter().filter_map(|e| match e {
        Event::Called { side, .. } => Some(*side),
        _ => None,
    });
    let exits = filmed.iter().filter_map(|e| match e {
        Event::ProverExited { side, .. } => Some(*side),
        _ => None,
    });
    let tape = played.tape();
    let from_camera: Vec<(Side, Side)> = calls.zip(exits).collect();
    let on_tape: Vec<(Side, Side)> = tape.scenes.iter().map(|s| (s.call, s.exit)).collect();
    assert_eq!(from_camera, on_tape);
    assert!(filmed.iter().all(|e| !matches!(e, Event::ProverEntered { .. })));
    assert!(filmed.iter().all(|e| !matches!(e, Event::AtTheWall { .. })));
}

#[test]
fn the_odds_halve_with_every_scene_and_an_edit_costs_twice_its_length() {
    assert_eq!(odds::survival_probability(1), 0.5);
    assert_eq!(odds::survival_probability(40), 1.0 / 1_099_511_627_776.0);
    assert_eq!(odds::survival_one_in(40), Some(1_099_511_627_776));
    assert_eq!(odds::survival_one_in(127), Some(1 << 127));
    assert_eq!(odds::survival_one_in(128), None);
    assert_eq!(odds::expected_takes(40), 80);
}
