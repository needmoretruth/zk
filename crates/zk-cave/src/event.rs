//! Every step of a filming, in order, for an animation to follow one at a time.

use crate::cast::{Character, Coin, Side, WallOutcome};

/// Where a step happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct At {
    /// The take, from 1. In a demonstration every take is a scene of the tape; the jealous reporter
    /// cuts some; the apartment building plays a single take on every floor at once.
    pub take: u32,
    /// The floor, from 1. The single cave of the story is floor 1.
    pub floor: u32,
}

/// One step of the story.
///
/// Within one scene the steps come in this order: `ProverEntered`, `ReporterAtFork`, `CoinFlipped`
/// (absent under prior agreement), `Called`, `AtTheWall` (only when the prover is in the passage
/// that was not called), `WalkedBack`, `ProverExited`, `SceneEnded`. The apartment building
/// interleaves its floors: every floor's entry, then every floor's coin and call, then every
/// floor's exit. [`Event::on_camera`] says which steps the camera at the fork filmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Before the scenes, the crew films a tour of the cave showing both passages end in a dead end.
    Tour,
    /// Before the scenes, the reporter and the double agree every call in advance. Off camera.
    CallsAgreed {
        /// The calls, scene 1 first.
        calls: Vec<Side>,
    },
    /// Everyone has left; the prover goes in alone and walks to the dead end of one passage.
    /// Off camera: nobody sees which passage.
    ProverEntered {
        /// Where.
        at: At,
        /// Who went in.
        who: Character,
        /// The passage taken.
        passage: Side,
    },
    /// The reporter, with the camera, goes in only as far as the fork.
    ReporterAtFork {
        /// Where.
        at: At,
    },
    /// The reporter flips the coin.
    CoinFlipped {
        /// Where.
        at: At,
        /// The face that came up.
        coin: Coin,
    },
    /// The reporter calls out the side to come out on.
    Called {
        /// Where.
        at: At,
        /// The side called.
        side: Side,
    },
    /// The prover, in the passage that was not called, is at the wall. Off camera.
    AtTheWall {
        /// Where.
        at: At,
        /// Whether it opened; never [`WallOutcome::NotNeeded`] here.
        outcome: WallOutcome,
    },
    /// The prover walks back along a passage towards the fork. Off camera.
    WalkedBack {
        /// Where.
        at: At,
        /// The passage walked along: the called one after crossing, else the one entered.
        passage: Side,
    },
    /// The prover comes out at the fork.
    ProverExited {
        /// Where.
        at: At,
        /// The passage the prover came out of.
        side: Side,
    },
    /// The scene is over; it succeeded when the prover came out on the called side.
    SceneEnded {
        /// Where.
        at: At,
        /// Whether the exit matched the call.
        succeeded: bool,
    },
    /// In the editing room, after filming: a take is cut or kept as a scene of the edited tape.
    Edited {
        /// The take.
        take: u32,
        /// Its scene number on the edited tape, `None` when cut.
        kept_as: Option<u32>,
    },
}

impl Event {
    /// Whether the camera filmed this step: the tour, and at the fork the reporter arriving, the
    /// coin, the call and the prover coming out. Everything deeper in the cave, every agreement
    /// made beforehand and everything done in the editing room is off camera.
    pub fn on_camera(&self) -> bool {
        matches!(
            self,
            Self::Tour
                | Self::ReporterAtFork { .. }
                | Self::CoinFlipped { .. }
                | Self::Called { .. }
                | Self::ProverExited { .. }
        )
    }
}
