//! The circuit digest the Fiat–Shamir challenge binds, so a proof belongs to the gates it was made
//! for and to no other circuit.
//!
//! `digest = SHA-256("zk/trio/v1/circuit" ‖ u64 wires ‖ u64 public inputs ‖ u64 private inputs ‖
//! u64 gates ‖ gate…)`, every integer little-endian, every gate in circuit order:
//!
//! | tag | gate        | then                                                                 |
//! |-----|-------------|----------------------------------------------------------------------|
//! | 0   | constant    | u64 output ‖ element                                                 |
//! | 1   | input       | u64 output ‖ u8 visibility (0 public, 1 private) ‖ string name       |
//! | 2   | linear      | u64 output ‖ lc                                                      |
//! | 3   | mul         | u64 output ‖ lc left ‖ lc right                                      |
//! | 4   | assert-zero | lc ‖ string label                                                    |
//! | 5   | hint        | u32 count ‖ u64 output… ‖ u8 kind (0 bits, 1 inverse) ‖ u32 bits ‖ lc |
//!
//! where `lc = u32 terms ‖ (u64 wire ‖ element)… ‖ element constant` with the terms exactly as the
//! circuit stores them (not normalised), `string = u32 byte length ‖ UTF-8`, `element` = 8
//! canonical little-endian bytes, and `bits` is the bit count of a bits hint (0 for inverse).

use sha2::{Digest, Sha256};
use zk_circuit::{Circuit, Gate, Hint, LinearCombination, Visibility};

use crate::field::Fp;
use crate::hash::{Bytes32, DOMAIN_CIRCUIT};

/// SHA-256 of `circuit` in the format above.
pub(crate) fn circuit_digest(circuit: &Circuit<Fp>) -> Bytes32 {
    let mut hasher = Sha256::new_with_prefix(DOMAIN_CIRCUIT);
    for count in [
        circuit.num_wires(),
        circuit.public_inputs().len(),
        circuit.private_inputs().len(),
        circuit.gates().len(),
    ] {
        hasher.update((count as u64).to_le_bytes());
    }
    for gate in circuit.gates() {
        gate_bytes(&mut hasher, gate);
    }
    hasher.finalize().into()
}

fn gate_bytes(hasher: &mut Sha256, gate: &Gate<Fp>) {
    let wire = |hasher: &mut Sha256, index: usize| hasher.update((index as u64).to_le_bytes());
    match gate {
        Gate::Constant { output, value } => {
            hasher.update([0]);
            wire(hasher, output.index());
            hasher.update(value.encode());
        }
        Gate::Input { output, name, visibility } => {
            hasher.update([1]);
            wire(hasher, output.index());
            hasher.update([u8::from(*visibility == Visibility::Private)]);
            string_bytes(hasher, name);
        }
        Gate::Linear { output, lc } => {
            hasher.update([2]);
            wire(hasher, output.index());
            lc_bytes(hasher, lc);
        }
        Gate::Mul { output, left, right } => {
            hasher.update([3]);
            wire(hasher, output.index());
            lc_bytes(hasher, left);
            lc_bytes(hasher, right);
        }
        Gate::AssertZero { lc, label } => {
            hasher.update([4]);
            lc_bytes(hasher, lc);
            string_bytes(hasher, label);
        }
        Gate::Hint { outputs, kind, input } => {
            hasher.update([5]);
            hasher.update((outputs.len() as u32).to_le_bytes());
            for output in outputs {
                wire(hasher, output.index());
            }
            let (tag, bits) = match kind {
                Hint::Bits(bits) => (0u8, *bits),
                Hint::Inverse => (1u8, 0),
            };
            hasher.update([tag]);
            hasher.update(bits.to_le_bytes());
            lc_bytes(hasher, input);
        }
    }
}

fn lc_bytes(hasher: &mut Sha256, lc: &LinearCombination<Fp>) {
    hasher.update((lc.terms().len() as u32).to_le_bytes());
    for (wire, coefficient) in lc.terms() {
        hasher.update((wire.index() as u64).to_le_bytes());
        hasher.update(coefficient.encode());
    }
    hasher.update(lc.constant_term().encode());
}

fn string_bytes(hasher: &mut Sha256, text: &str) {
    hasher.update((text.len() as u32).to_le_bytes());
    hasher.update(text.as_bytes());
}
