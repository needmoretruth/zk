//! The magic wall between the ends of the two passages: the one part of the cave everyone trusts.

use core::fmt;

use zk_circuit::{Assignment, Circuit};
use zk_examples::ExampleId;

use crate::error::CaveError;
use crate::field::Goldilocks;

/// The wall that joins the dead ends of the left and right passages and slides open only for the
/// magic words.
///
/// **This is where the trust lives.** In the story the wall is magic: nobody asks how it knows the
/// words, and the reporter at the fork never sees it. Here the wall holds a lock — an example's
/// circuit and the public inputs of the claim it was built for — and opens when the whispered
/// private inputs, together with those public inputs, satisfy every assertion of the circuit.
/// Nothing but trust stands behind that check: the reporter cannot watch it happen and has no way to
/// confirm it was done. A real zero-knowledge proof system replaces exactly this component with
/// mathematics, which is why the cave's catalog entry names a trusted component as its setup and
/// its only assumption.
///
/// The lock sits in private fields, so no code outside this module can read it — not the reporter
/// at the fork (the verifier), not the court, not the prover. The only question anyone can ask the
/// wall is [`Wall::whisper`]. The compiler enforces this:
///
/// ```compile_fail,E0616
/// use zk_cave::Wall;
/// use zk_examples::ExampleId;
///
/// let wall = Wall::for_example(ExampleId::Factoring, Vec::new())?;
/// let lock = &wall.public; // error[E0616]: field `public` of struct `Wall` is private
/// # Ok::<(), zk_cave::CaveError>(())
/// ```
///
/// The same wall, asked the only way it can be asked:
///
/// ```
/// use zk_cave::{Goldilocks, MagicWords, Wall};
/// use zk_examples::{ExampleId, InstanceKind};
///
/// let claim = ExampleId::Factoring.instance::<Goldilocks>(InstanceKind::Honest, &[0; 32]);
/// let false_claim = ExampleId::Factoring.instance::<Goldilocks>(InstanceKind::Dishonest, &[0; 32]);
/// let wall = Wall::for_example(ExampleId::Factoring, claim.public)?;
/// assert!(wall.whisper(&MagicWords::new(claim.private)));
/// assert!(!wall.whisper(&MagicWords::new(false_claim.private)));
/// # Ok::<(), zk_cave::CaveError>(())
/// ```
#[derive(Clone)]
pub struct Wall {
    circuit: Circuit<Goldilocks>,
    public: Vec<Goldilocks>,
}

impl Wall {
    /// A wall whose lock is `circuit`, set to the claim with these public inputs.
    pub fn new(circuit: Circuit<Goldilocks>, public: Vec<Goldilocks>) -> Self {
        Self { circuit, public }
    }

    /// A wall for one of the seven examples, set to the claim with these public inputs.
    pub fn for_example(example: ExampleId, public: Vec<Goldilocks>) -> Result<Self, CaveError> {
        let circuit =
            example.circuit::<Goldilocks>().map_err(|e| CaveError::Circuit(e.to_string()))?;
        Ok(Self::new(circuit, public))
    }

    /// Slides open only if `words`, together with the lock's public inputs, satisfy every assertion
    /// of the circuit. The wrong number of words keeps it shut like any other wrong words.
    pub fn whisper(&self, words: &MagicWords) -> bool {
        let assignment = Assignment { public: self.public.clone(), private: words.private.clone() };
        self.circuit.evaluate(&assignment).is_ok()
    }
}

impl fmt::Debug for Wall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wall").finish_non_exhaustive()
    }
}

/// What a prover whispers at the wall: the private inputs of a claim, in declaration order.
///
/// The values are private to this module, like the lock: the wall reads them, and nobody else —
/// the prover carries them into the cave without the rest of the engine ever looking inside.
#[derive(Clone)]
pub struct MagicWords {
    private: Vec<Goldilocks>,
}

impl MagicWords {
    /// Words made of these private inputs.
    pub fn new(private: Vec<Goldilocks>) -> Self {
        Self { private }
    }
}

impl fmt::Debug for MagicWords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MagicWords").finish_non_exhaustive()
    }
}
