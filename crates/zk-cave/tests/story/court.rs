//! The court: what a judge can count, and why it cannot tell a genuine tape from an edited one.

use zk_cave::court::{Averages, TapeStats};
use zk_cave::{Prover, SCENES, Side, Tape, TapeScene, demonstration, jealous_edit};

use crate::{seeded, setting};

/// Tapes of each kind compared.
const TAPES: u64 = 400;

/// Allowed gaps between the two averages. With 400 tapes of 40 scenes, the standard deviation of
/// the gap is about 0.0065 for the right-call share, 0.22 for the number of runs and 0.12 for the
/// longest run; each tolerance is more than four of those.
const SHARE_TOLERANCE: f64 = 0.03;
const RUNS_TOLERANCE: f64 = 1.0;
const LONGEST_RUN_TOLERANCE: f64 = 0.5;

#[test]
fn a_judge_counts_calls_matches_and_runs() {
    let scene = |call, exit| TapeScene { call, exit };
    let tape = Tape {
        scenes: vec![
            scene(Side::Right, Side::Right),
            scene(Side::Right, Side::Right),
            scene(Side::Left, Side::Right),
            scene(Side::Right, Side::Right),
        ],
    };
    let stats = TapeStats::of(&tape);
    assert_eq!((stats.scenes, stats.right_calls, stats.left_calls), (4, 3, 1));
    assert_eq!(stats.exits_matching_call, 3);
    assert_eq!((stats.runs, stats.longest_run), (3, 2));
    assert_eq!(stats.run_lengths, vec![2, 1]);
}

#[test]
fn genuine_and_edited_tapes_agree_over_many_tapes_but_unedited_ones_do_not() {
    let setting = setting();
    let mick = Prover::mick_ali(setting.words.clone());
    let double = Prover::double(None);
    let mut genuine = Vec::new();
    let mut edited = Vec::new();
    let mut unedited = Vec::new();
    for stream in 0..TAPES {
        let played = demonstration(&setting.wall, &mick, SCENES, &mut seeded(1000 + stream));
        genuine.push(played.unwrap().tape());
        let edit =
            jealous_edit(&setting.wall, &double, SCENES, &mut seeded(5000 + stream)).unwrap();
        edited.push(edit.tape());
        unedited.push(edit.takes.iter().map(|take| take.on_tape()).collect::<Tape>());
    }
    let (genuine, edited, unedited) =
        (Averages::of(&genuine), Averages::of(&edited), Averages::of(&unedited));
    println!("genuine {genuine:?}\nedited {edited:?}\nunedited {unedited:?}");
    assert!((genuine.right_call_share - edited.right_call_share).abs() < SHARE_TOLERANCE);
    assert_eq!((genuine.match_share, edited.match_share), (1.0, 1.0));
    assert!((genuine.runs - edited.runs).abs() < RUNS_TOLERANCE);
    assert!((genuine.longest_run - edited.longest_run).abs() < LONGEST_RUN_TOLERANCE);
    assert!(genuine.match_share - unedited.match_share > 0.3);
}
