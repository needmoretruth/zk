//! The layout handed to Remainder's circuit builder.

use frontend::layouter::builder::{Circuit, CircuitBuilder, LayerVisibility};
use remainder::layer::gate::BinaryOperation;
use shared_types::Fr;

use crate::levels::Layout;

/// Label of the input layer the verifier reads in the clear.
const PUBLIC_LAYER: &str = "public";
/// Label of the input layer Hyrax commits to with Pedersen vector commitments.
const COMMITTED_LAYER: &str = "committed";
/// The one shred of the public layer: one, the constants and the public inputs.
pub(crate) const PUBLIC_TABLE: &str = "public table";
/// The one shred of the committed layer: the private inputs and hint outputs.
pub(crate) const COMMITTED_TABLE: &str = "committed table";

/// Builds the layered circuit, without input values, so prover and verifier share one description.
///
/// Remainder's layers, bottom up: a gate layer lifting the committed table into level 1's slots, a
/// sector adding it to the public table (level 1), one multiplication gate layer per further level,
/// and a last gate layer copying the checks into the output node, which Remainder requires to be
/// zero everywhere. Above level 1, every gate layer reads only the layer directly below it.
pub(crate) fn build(layout: &Layout) -> Result<Circuit<Fr>, String> {
    let mut builder = CircuitBuilder::<Fr>::new();
    let public_layer = builder.add_input_layer(PUBLIC_LAYER, LayerVisibility::Public);
    let committed_layer = builder.add_input_layer(COMMITTED_LAYER, LayerVisibility::Committed);
    let public = builder.add_input_shred(PUBLIC_TABLE, layout.public_bits, &public_layer);
    let committed =
        builder.add_input_shred(COMMITTED_TABLE, layout.committed_bits, &committed_layer);
    let lifted =
        builder.add_gate_node(&committed, &public, layout.lift.clone(), BinaryOperation::Mul, None);
    let mut level = builder.add_sector(&public + &lifted);
    for gates in &layout.steps {
        level = builder.add_gate_node(&level, &level, gates.clone(), BinaryOperation::Mul, None);
    }
    let output =
        builder.add_gate_node(&level, &level, layout.output.clone(), BinaryOperation::Mul, None);
    builder.set_output(&output);
    builder.build().map_err(|e| format!("Remainder could not build the layered circuit: {e:#}"))
}
