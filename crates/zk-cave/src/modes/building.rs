//! The apartment building: one cave per floor, each with its own words, all tested in one round.

use rand_core::TryRng;

use crate::cast::{Prover, Side};
use crate::error::CaveError;
use crate::event::{At, Event};
use crate::reporter;
use crate::scene::{self, AfterCall, Scene};
use crate::tape::Tape;
use crate::wall::Wall;

/// One floor: its own cave behind its own wall, and the actor sent into it.
#[derive(Clone, Debug)]
pub struct Floor {
    /// This floor's wall, locked to its own claim.
    pub wall: Wall,
    /// The actor in this floor's cave.
    pub prover: Prover,
}

/// One round played on every floor at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingRound {
    /// One scene per floor, floor 1 first.
    pub floors: Vec<Scene>,
    /// Every step: all actors enter, then every floor's coin and call, then every floor's exit.
    pub events: Vec<Event>,
}

impl BuildingRound {
    /// Floors (from 1) where the actor came out on the wrong side.
    pub fn failed_floors(&self) -> Vec<u32> {
        self.floors.iter().filter(|scene| !scene.succeeded()).map(|scene| scene.at.floor).collect()
    }

    /// Whether every floor succeeded. Without the words, surviving `n` floors in one round is as
    /// unlikely as surviving `n` scenes one after another.
    pub fn convinced(&self) -> bool {
        self.floors.iter().all(Scene::succeeded)
    }

    /// What the cameras recorded, floor 1 first, in the same format as a tape of scenes.
    pub fn tape(&self) -> Tape {
        self.floors.iter().map(Scene::on_tape).collect()
    }
}

/// Plays one round in the building: every actor goes in first, then every floor's coin is flipped
/// and its exit called, then every actor comes out.
///
/// "Other researchers in Israel" proposed testing in parallel this way. Draws one bit per floor for
/// the passages, floor 1 first, then one bit per floor for the coins.
pub fn apartment_building<R: TryRng + ?Sized>(
    floors: &[Floor],
    rng: &mut R,
) -> Result<BuildingRound, CaveError> {
    let mut events = vec![Event::Tour];
    let mut entered: Vec<(At, Side)> = Vec::with_capacity(floors.len());
    for (number, floor) in (1..).zip(floors) {
        let at = At { take: 1, floor: number };
        let passage = Side::draw(rng)?;
        scene::enter(&floor.prover, at, passage, &mut events);
        entered.push((at, passage));
    }
    let mut calls = Vec::with_capacity(floors.len());
    for (at, _) in &entered {
        calls.push(reporter::flip_and_call(*at, rng, &mut events)?);
    }
    let played = floors
        .iter()
        .zip(entered)
        .zip(calls)
        .map(|((floor, (at, entered)), (coin, call))| {
            let after = AfterCall { at, entered, coin: Some(coin), call };
            scene::come_out(&floor.wall, &floor.prover, after, &mut events)
        })
        .collect();
    Ok(BuildingRound { floors: played, events })
}
