//! One scene: the prover goes in, the reporter calls, the prover comes out.

use rand_core::TryRng;

use crate::cast::{Character, Coin, Prover, Side, WallOutcome};
use crate::error::CaveError;
use crate::event::{At, Event};
use crate::reporter;
use crate::tape::TapeScene;
use crate::wall::Wall;

/// Everything that happened in one scene, including what only the prover knows.
///
/// The camera recorded only [`Scene::on_tape`]; the rest exists so an animation can show the
/// inside of the cave and a reader can see why the tape alone proves nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scene {
    /// Take and floor.
    pub at: At,
    /// Who was in the cave. Not on tape: the double looks like Mick.
    pub who: Character,
    /// The passage the prover entered. Hidden from the reporter and not on tape.
    pub entered: Side,
    /// The coin, `None` when the call was agreed in advance.
    pub coin: Option<Coin>,
    /// The side the reporter called.
    pub call: Side,
    /// What happened at the wall. Hidden from the reporter and not on tape.
    pub wall: WallOutcome,
    /// The side the prover came out on.
    pub exit: Side,
}

impl Scene {
    /// Whether the prover came out on the called side.
    pub fn succeeded(&self) -> bool {
        self.exit == self.call
    }

    /// Whether the wall slid open in this scene.
    pub fn wall_opened(&self) -> bool {
        self.wall == WallOutcome::Opened
    }

    /// What the camera at the fork recorded of this scene.
    pub fn on_tape(&self) -> TapeScene {
        TapeScene { call: self.call, exit: self.exit }
    }
}

/// A whole scene with a coin: the prover picks a passage, then the reporter flips.
///
/// Draw order, which scripted tests rely on: one bit for the passage, then one bit for the coin.
pub(crate) fn play<R: TryRng + ?Sized>(
    wall: &Wall,
    prover: &Prover,
    at: At,
    rng: &mut R,
    events: &mut Vec<Event>,
) -> Result<Scene, CaveError> {
    let entered = Side::draw(rng)?;
    enter(prover, at, entered, events);
    let (coin, call) = reporter::flip_and_call(at, rng, events)?;
    Ok(come_out(wall, prover, AfterCall { at, entered, coin: Some(coin), call }, events))
}

/// The prover goes in alone to the dead end of `passage`.
pub(crate) fn enter(prover: &Prover, at: At, passage: Side, events: &mut Vec<Event>) {
    events.push(Event::ProverEntered { at, who: prover.who(), passage });
}

/// The state of a scene once the call has been made.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AfterCall {
    pub(crate) at: At,
    pub(crate) entered: Side,
    pub(crate) coin: Option<Coin>,
    pub(crate) call: Side,
}

/// The prover answers the call: crosses at the wall if needed, walks back, comes out.
pub(crate) fn come_out(
    wall: &Wall,
    prover: &Prover,
    called: AfterCall,
    events: &mut Vec<Event>,
) -> Scene {
    let AfterCall { at, entered, coin, call } = called;
    let (outcome, exit) = prover.respond(wall, entered, call);
    if outcome != WallOutcome::NotNeeded {
        events.push(Event::AtTheWall { at, outcome });
    }
    events.push(Event::WalkedBack { at, passage: exit });
    events.push(Event::ProverExited { at, side: exit });
    let scene = Scene { at, who: prover.who(), entered, coin, call, wall: outcome, exit };
    events.push(Event::SceneEnded { at, succeeded: scene.succeeded() });
    scene
}
