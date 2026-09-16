//! Part B: an honest chain checks out, and each cheat is caught by the check meant for it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use rand_core::OsRng;
use zk_ceremony::tau::{
    ChainError, Check, Contribution, DEFAULT_PARTICIPANTS, POWERS, Participant, Srs, cheat, digest,
    verify_chain,
};

/// `count` honest turns starting from `start`.
fn honest_turns(start: &Srs, count: usize) -> Vec<(Contribution, Srs)> {
    let mut steps: Vec<(Contribution, Srs)> = Vec::new();
    for _ in 0..count {
        let previous = steps.last().map_or(start, |(_, srs)| srs);
        let (next, contribution) = Participant::contribute(previous, &mut OsRng);
        steps.push((contribution, next));
    }
    steps
}

#[test]
fn five_honest_participants_build_a_chain_anyone_can_check() {
    let initial = Srs::initial();
    let started = Instant::now();
    let steps = honest_turns(&initial, DEFAULT_PARTICIPANTS);
    let contributing = started.elapsed();
    let started = Instant::now();
    let verdict = verify_chain(&initial, &steps);
    let verifying = started.elapsed();
    println!(
        "n = {POWERS}, {DEFAULT_PARTICIPANTS} participants: contribute {contributing:?} in all · \
         verify chain {verifying:?}"
    );
    assert_eq!(verdict, Ok(()));
    let mut fingerprints: Vec<[u8; 32]> = steps.iter().map(|(_, srs)| digest(srs)).collect();
    fingerprints.push(digest(&initial));
    fingerprints.sort_unstable();
    fingerprints.dedup();
    assert_eq!(fingerprints.len(), DEFAULT_PARTICIPANTS + 1);
}

#[test]
fn a_participant_who_ignores_everyone_before_them_is_caught_by_check_three() {
    let initial = Srs::initial();
    let mut steps = honest_turns(&initial, 2);
    let (next, contribution) = cheat::ignore_previous(&steps[1].1, &mut OsRng);
    steps.push((contribution, next));
    let error = verify_chain(&initial, &steps).unwrap_err();
    assert_eq!(error, ChainError { step: 2, check: Check::BuildsOnPrevious });
}

#[test]
fn a_participant_who_replaces_one_power_is_caught_by_check_four() {
    let initial = Srs::initial();
    let mut steps = honest_turns(&initial, 2);
    let (next, contribution) = cheat::replace_power(&steps[1].1, 17, &mut OsRng).unwrap();
    steps.push((contribution, next));
    let error = verify_chain(&initial, &steps).unwrap_err();
    assert_eq!(error, ChainError { step: 2, check: Check::ConsistentPowers });
}

#[test]
fn a_turn_copied_onto_another_chain_is_caught_by_check_one() {
    let initial = Srs::initial();
    let steps = honest_turns(&initial, 2);
    let copied = [steps[1].clone()];
    let error = verify_chain(&initial, &copied).unwrap_err();
    assert_eq!(error, ChainError { step: 0, check: Check::KnowledgeProof });
}

#[test]
fn a_replaced_first_or_second_power_is_caught_before_check_four() {
    let initial = Srs::initial();
    let caught = |index| {
        let (next, contribution) = cheat::replace_power(&initial, index, &mut OsRng).unwrap();
        verify_chain(&initial, &[(contribution, next)]).unwrap_err().check
    };
    assert_eq!(caught(0), Check::WellFormed);
    assert_eq!(caught(1), Check::BuildsOnPrevious);
    assert!(cheat::replace_power(&initial, POWERS, &mut OsRng).is_none());
}
