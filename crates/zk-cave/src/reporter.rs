//! The reporter at the fork: the verifier.
//!
//! Everything the reporter does is here, and none of it touches the wall: the reporter never goes
//! past the fork, never sees which passage the prover took, and learns only which side the prover
//! comes out on. The wall's lock is private to its own module, so this code could not read it even
//! by mistake.

use rand_core::TryRng;

use crate::cast::{Coin, Side};
use crate::error::CaveError;
use crate::event::{At, Event};

/// Goes in as far as the fork, flips a fair coin and calls the side it names.
pub(crate) fn flip_and_call<R: TryRng + ?Sized>(
    at: At,
    rng: &mut R,
    events: &mut Vec<Event>,
) -> Result<(Coin, Side), CaveError> {
    events.push(Event::ReporterAtFork { at });
    let coin = Coin::flip(rng)?;
    events.push(Event::CoinFlipped { at, coin });
    let side = coin.call();
    events.push(Event::Called { at, side });
    Ok((coin, side))
}

/// Goes in as far as the fork and calls the side agreed in advance; no coin is flipped.
pub(crate) fn call_agreed(at: At, side: Side, events: &mut Vec<Event>) {
    events.push(Event::ReporterAtFork { at });
    events.push(Event::Called { at, side });
}
