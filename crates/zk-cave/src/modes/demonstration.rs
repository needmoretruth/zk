//! The first network's demonstration: scene after scene until the planned number or the first
//! failure. The reporter follows the same steps whoever is in the cave, so Mick's demonstration
//! and the double being caught are this one procedure with a different prover.

use rand_core::TryRng;

use crate::cast::Prover;
use crate::error::CaveError;
use crate::event::{At, Event};
use crate::scene::{self, Scene};
use crate::tape::Tape;
use crate::wall::Wall;

/// "In memory of the forty thieves this demonstration scene was played forty times."
pub const SCENES: u32 = 40;

/// A filmed demonstration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Demonstration {
    /// Scenes the reporter planned.
    pub planned: u32,
    /// Scenes played, in order; when someone was caught, the last one is the failure.
    pub scenes: Vec<Scene>,
    /// The scene number (from 1) of the first failure, where filming stopped.
    pub caught_at: Option<u32>,
    /// Every step, for an animation.
    pub events: Vec<Event>,
}

impl Demonstration {
    /// Whether every planned scene was played and succeeded: the reporter at the fork is convinced.
    pub fn convinced(&self) -> bool {
        self.caught_at.is_none() && self.scenes.len() == self.planned as usize
    }

    /// What the camera recorded.
    pub fn tape(&self) -> Tape {
        self.scenes.iter().map(Scene::on_tape).collect()
    }
}

/// Films up to `scenes` scenes of `prover` in the cave behind `wall`, stopping at the first failure.
///
/// Mick Ali with the right words is never caught. Anyone without them is on the wrong side half the
/// time and is caught at the first such scene. Each scene draws one bit for the passage, then one
/// bit for the coin.
pub fn demonstration<R: TryRng + ?Sized>(
    wall: &Wall,
    prover: &Prover,
    scenes: u32,
    rng: &mut R,
) -> Result<Demonstration, CaveError> {
    demonstrate(wall, prover, scenes, rng, |_| Ok(()))
}

/// [`demonstration`], calling `before_scene` with each scene number first, so the harness can check
/// for cancellation and report progress.
pub(crate) fn demonstrate<R, E>(
    wall: &Wall,
    prover: &Prover,
    planned: u32,
    rng: &mut R,
    mut before_scene: impl FnMut(u32) -> Result<(), E>,
) -> Result<Demonstration, E>
where
    R: TryRng + ?Sized,
    E: From<CaveError>,
{
    let mut events = vec![Event::Tour];
    let mut scenes = Vec::new();
    let mut caught_at = None;
    for take in 1..=planned {
        before_scene(take)?;
        let scene = scene::play(wall, prover, At { take, floor: 1 }, rng, &mut events)?;
        scenes.push(scene);
        if !scene.succeeded() {
            caught_at = Some(take);
            break;
        }
    }
    Ok(Demonstration { planned, scenes, caught_at, events })
}
