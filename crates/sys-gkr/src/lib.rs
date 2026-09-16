//! GKR (Goldwasser, Kalai and Rothblum, 2008), run by Worldcoin's Remainder in its Hyrax
//! zero-knowledge mode over BN254.
//!
//! GKR proves that a layered arithmetic circuit was evaluated correctly. The verifier holds a claim
//! about the output layer; one sum-check per layer turns a claim about layer `i + 1` into a claim
//! about layer `i`, until only a claim about the input remains, which the verifier checks itself.
//! That verifier must see the whole input, so plain GKR hides nothing: it is a proof for delegating
//! computation. Hyrax (Wahby, Tzialla, shelat, Thaler and Walfish, 2018) makes it a zero-knowledge
//! argument: every prover message is a Pedersen commitment rather than a field element, the sum-checks
//! are checked through proofs about the commitments, and the private part of the input is committed
//! with the Hyrax polynomial commitment, so the final input claim is opened in zero knowledge.
//!
//! [Remainder](https://github.com/worldcoin/Remainder_CE) (MIT or Apache-2.0, pinned to one commit)
//! implements both; this exhibit runs its `hyrax` prover and verifier. Each statement is lowered to
//! the museum's R1CS, whose rows are rewritten as sums of products (`graph`), laid out in levels
//! where each level reads only the one below and values needed higher up are copied upward
//! (`levels`), and built with Remainder's circuit builder (`layered`). The public inputs and every
//! coefficient sit in a public input layer the verifier fills in itself; the private inputs and the
//! prover's hint values sit in a committed input layer. Remainder computes every layer from those
//! two tables, and its `HyraxProvableCircuit::prove` and `verify_hyrax_proof` do the rest
//! (`prepared`); the proof travels as bincode, as Remainder's own provers write it (`proof`).
//!
//! Remainder's layers may also read layers further down. This exhibit uses that only at the bottom,
//! where level 1 adds the public input layer to the committed values lifted into its slots; every
//! layer above reads only the layer directly below, as in the layered circuits the GKR paper proves.

mod field;
mod graph;
mod layered;
mod levels;
mod meta;
mod prepared;
mod proof;

use zk_core::catalog::SystemMeta;
use zk_core::{Control, ExampleId, Prepared, ProofSystem, SystemError};

pub use field::Bn254Scalar;
pub use meta::META;

/// The field the statements are built over here, under the name every exhibit exports, so an
/// activity such as the Toy Shielded Pool can compute values in it without knowing the curve.
pub type Field = Bn254Scalar;

/// The GKR exhibit; stateless, since every run builds its own circuit and generators.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gkr;

impl ProofSystem for Gkr {
    fn meta(&self) -> &'static SystemMeta {
        &META
    }

    fn prepare(
        &self,
        example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        Ok(Box::new(prepared::Ready::build(example, control)?))
    }
}
