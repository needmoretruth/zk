//! The proof object and its bytes.
//!
//! # Byte format
//!
//! Integers are little-endian, field elements 8 canonical little-endian bytes, salts and hashes
//! 32 bytes. `ℓ`, `k`, `n`, `t` are the circuit's [`Params`], `σ = 2`, and `R` is the number of
//! committed rows: the tested rows `w, x, y, z`, then for each repetition its code, linear and
//! quadratic blinding rows.
//!
//! ```text
//! u32 ℓ ‖ u32 k ‖ u32 n ‖ u32 t            must equal what the circuit fixes
//! root                                     Merkle root over the n salted columns
//! σ × ( code:      k elements              Σ r_i·p_i + code blind            (proximity test)
//!       linear:    k + ℓ − 1 elements      Σ r_i(X)·p_i + linear blind       (linear test)
//!       quadratic: 2k − 1 elements )       Σ r_i·(p_x·p_y − p_z) + blind     (quadratic test)
//! t × ( salt ‖ R elements )                opened columns, ascending; entries in row order
//! H × hash                                 Merkle siblings, in the order of crate::merkle
//! ```
//!
//! The three responses are polynomials, sent as coefficients from `X^0` up. A proof is therefore
//! `16 + 32 + σ·8·(4k + ℓ − 2) + t·(32 + 8R) + 32·H` bytes. `H` depends on which columns the hash
//! chose, at most `t·log₂ n`; everything before the siblings has a length the circuit fixes, so any
//! byte missing there, or a sibling section not made of whole hashes, makes the proof malformed. A
//! wrong number of whole hashes decodes and fails the Merkle check.

use crate::field::{ELEMENT_BYTES, Fp};
use crate::hash::Bytes32;
use crate::params::{OPENED_COLUMNS, Params, REPETITIONS};

/// The four parameters at the front of every proof.
pub const HEADER_BYTES: usize = 16;

/// One repetition's answers to the random combinations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// The proximity test's combined row, degree below `k`.
    pub code: Vec<Fp>,
    /// The linear test's polynomial, degree below `k + ℓ − 1`.
    pub linear: Vec<Fp>,
    /// The quadratic test's polynomial, degree below `2k − 1`.
    pub quadratic: Vec<Fp>,
}

/// One opened column with the salt of its leaf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedColumn {
    /// The leaf's salt.
    pub salt: Bytes32,
    /// One entry per committed row.
    pub entries: Vec<Fp>,
}

/// A decoded proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// The commitment to every column.
    pub root: Bytes32,
    /// One per repetition.
    pub responses: Vec<Response>,
    /// The opened columns, ascending by column index.
    pub columns: Vec<OpenedColumn>,
    /// The Merkle siblings the opening needs.
    pub siblings: Vec<Bytes32>,
}

/// Bytes of everything but the siblings, which the circuit's parameters fix.
pub fn fixed_bytes(params: &Params) -> usize {
    let response = ELEMENT_BYTES * (4 * params.dimension + params.message_length - 2);
    HEADER_BYTES
        + 32
        + REPETITIONS * response
        + OPENED_COLUMNS * (32 + ELEMENT_BYTES * params.rows())
}

/// The first byte of the first opened column's first entry (row 0 of the block `w`). The verifier
/// hashes it into that column's Merkle leaf, so a changed entry no longer hashes to the root; the
/// same entry also enters the proximity check at that column.
pub fn tamper_offset(params: &Params) -> usize {
    let response = ELEMENT_BYTES * (4 * params.dimension + params.message_length - 2);
    HEADER_BYTES + 32 + REPETITIONS * response + 32
}

fn put_elements(out: &mut Vec<u8>, elements: &[Fp]) {
    for element in elements {
        out.extend_from_slice(&element.encode());
    }
}

/// The responses section exactly as the proof carries it, which the column challenge hashes.
pub(crate) fn encode_responses(responses: &[Response]) -> Vec<u8> {
    let mut out = Vec::new();
    for response in responses {
        put_elements(&mut out, &response.code);
        put_elements(&mut out, &response.linear);
        put_elements(&mut out, &response.quadratic);
    }
    out
}

impl Proof {
    /// The bytes of the format above.
    pub fn encode(&self, params: &Params) -> Vec<u8> {
        let mut out = Vec::with_capacity(fixed_bytes(params) + 32 * self.siblings.len());
        for value in [params.message_length, params.dimension, params.length, OPENED_COLUMNS] {
            out.extend_from_slice(&(value as u32).to_le_bytes());
        }
        out.extend_from_slice(&self.root);
        out.extend(encode_responses(&self.responses));
        for column in &self.columns {
            out.extend_from_slice(&column.salt);
            put_elements(&mut out, &column.entries);
        }
        for sibling in &self.siblings {
            out.extend_from_slice(sibling);
        }
        out
    }

    /// Reads the format above for a circuit with `params`; every failure is a sentence for
    /// [`zk_core::Verdict::Malformed`].
    pub fn decode(params: &Params, bytes: &[u8]) -> Result<Self, String> {
        let mut reader = Reader { bytes, position: 0 };
        let expected = [params.message_length, params.dimension, params.length, OPENED_COLUMNS];
        for (name, value) in ["ℓ", "k", "n", "t"].into_iter().zip(expected) {
            let got = reader.u32()?;
            if got as usize != value {
                return Err(format!("this circuit fixes {name} = {value}, the proof says {got}"));
            }
        }
        let root = reader.bytes32()?;
        let (k, ell) = (params.dimension, params.message_length);
        let responses = (0..REPETITIONS)
            .map(|_| {
                Ok(Response {
                    code: reader.elements(k)?,
                    linear: reader.elements(k + ell - 1)?,
                    quadratic: reader.elements(2 * k - 1)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let columns = (0..OPENED_COLUMNS)
            .map(|_| {
                Ok(OpenedColumn {
                    salt: reader.bytes32()?,
                    entries: reader.elements(params.rows())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let rest = &bytes[reader.position..];
        let (hashes, partial) = rest.as_chunks::<32>();
        if !partial.is_empty() {
            return Err(format!("{} bytes follow the last whole sibling hash", partial.len()));
        }
        Ok(Self { root, responses, columns, siblings: hashes.to_vec() })
    }
}

/// Reads a proof front to back.
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position.checked_add(count).filter(|end| *end <= self.bytes.len());
        let end =
            end.ok_or_else(|| format!("the proof ends early, at byte {}", self.bytes.len()))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, String> {
        let mut array = [0u8; 4];
        array.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(array))
    }

    fn bytes32(&mut self) -> Result<Bytes32, String> {
        let mut array = [0u8; 32];
        array.copy_from_slice(self.take(32)?);
        Ok(array)
    }

    fn elements(&mut self, count: usize) -> Result<Vec<Fp>, String> {
        let bytes = self.take(count.checked_mul(ELEMENT_BYTES).ok_or("too many elements")?)?;
        bytes.as_chunks::<ELEMENT_BYTES>().0.iter().map(|chunk| Fp::decode(chunk)).collect()
    }
}
