//! Prior agreement: the reporter and the double settle every call before filming.

use rand_core::TryRng;

use crate::cast::{Prover, Side};
use crate::error::CaveError;
use crate::event::{At, Event};
use crate::modes::demonstration::Demonstration;
use crate::reporter;
use crate::scene::{self, AfterCall};
use crate::wall::Wall;

/// Films `scenes` perfect scenes of `double` without a coin: the calls are drawn and agreed before
/// filming, the double walks into the passage that will be called, and the wall is never needed.
///
/// The paper credits "some European researchers" with this "simulation technique of prior agreement
/// between prover and verifier". The calls are random, so the tape looks like a genuine one; the
/// scenes carry no coin. Draws one bit per scene, all before the first scene.
pub fn prior_agreement<R: TryRng + ?Sized>(
    wall: &Wall,
    double: &Prover,
    scenes: u32,
    rng: &mut R,
) -> Result<Demonstration, CaveError> {
    let calls = (0..scenes).map(|_| Side::draw(rng)).collect::<Result<Vec<_>, _>>()?;
    let mut events = vec![Event::Tour, Event::CallsAgreed { calls: calls.clone() }];
    let mut played = Vec::with_capacity(calls.len());
    for (take, call) in (1..).zip(calls) {
        let at = At { take, floor: 1 };
        scene::enter(double, at, call, &mut events);
        reporter::call_agreed(at, call, &mut events);
        let after = AfterCall { at, entered: call, coin: None, call };
        played.push(scene::come_out(wall, double, after, &mut events));
    }
    Ok(Demonstration { planned: scenes, scenes: played, caught_at: None, events })
}
