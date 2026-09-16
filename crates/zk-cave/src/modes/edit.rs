//! The jealous reporter: film the double, keep only the good scenes.

use rand_core::TryRng;

use crate::cast::Prover;
use crate::error::CaveError;
use crate::event::{At, Event};
use crate::scene::{self, Scene};
use crate::tape::Tape;
use crate::wall::Wall;

/// Takes allowed per scene to keep before the edit gives up. A fair coin needs two on average;
/// sixty-four keeps a broken random source from filming forever and is never reached by a fair one
/// in practice.
pub const MAX_TAKES_PER_KEPT_SCENE: u32 = 64;

/// A filming session followed by the edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Every take filmed, in order, the spoiled ones included.
    pub takes: Vec<Scene>,
    /// Take numbers kept, in the order they appear on the edited tape.
    pub kept: Vec<u32>,
    /// Every step of the filming, then one [`Event::Edited`] per take.
    pub events: Vec<Event>,
}

impl Edit {
    /// Takes thrown away.
    pub fn discarded(&self) -> usize {
        self.takes.len() - self.kept.len()
    }

    /// The edited tape: the kept takes, renumbered as scenes 1, 2, 3, …
    pub fn tape(&self) -> Tape {
        self.takes.iter().filter(|take| take.succeeded()).map(Scene::on_tape).collect()
    }
}

/// Films `double` in the cave behind `wall` until `keep` scenes succeeded, then cuts every spoiled
/// take.
///
/// "The jealous reporter edited the tape and only kept the successful scenes until he had forty of
/// them." Nobody is caught here: a failed take is simply cut. Each take draws one bit for the
/// passage, then one bit for the coin.
pub fn jealous_edit<R: TryRng + ?Sized>(
    wall: &Wall,
    double: &Prover,
    keep: u32,
    rng: &mut R,
) -> Result<Edit, CaveError> {
    let limit = keep.saturating_mul(MAX_TAKES_PER_KEPT_SCENE);
    let mut events = vec![Event::Tour];
    let mut takes: Vec<Scene> = Vec::new();
    let mut kept = Vec::new();
    while kept.len() < keep as usize {
        let take = u32::try_from(takes.len()).unwrap_or(u32::MAX).saturating_add(1);
        if take > limit {
            let kept = u32::try_from(kept.len()).unwrap_or(u32::MAX);
            return Err(CaveError::EditNeverFinished { takes: limit, kept });
        }
        let scene = scene::play(wall, double, At { take, floor: 1 }, rng, &mut events)?;
        if scene.succeeded() {
            kept.push(take);
        }
        takes.push(scene);
    }
    let mut next_scene = 0u32;
    for scene in &takes {
        let kept_as = scene.succeeded().then(|| {
            next_scene += 1;
            next_scene
        });
        events.push(Event::Edited { take: scene.at.take, kept_as });
    }
    Ok(Edit { takes, kept, events })
}
