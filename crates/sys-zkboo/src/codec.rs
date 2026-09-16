//! First messages and responses as bytes, in the layout [`crate::proof`] documents.
//!
//! A response's layout depends only on its challenge and on the circuit's counts (`M`
//! multiplications, `S` witness values, `A` assertions), so the proof carries no tags or lengths.

use crate::field::{ELEMENT_BYTES, Fp};
use crate::hash::Bytes32;
use crate::party::Party;
use crate::program::Statement;
use crate::round::{FirstMessage, OpenedView, Response};

/// Bytes of one first message for a circuit with `assertions` assert-zero gates.
pub(crate) fn first_message_bytes(assertions: usize) -> usize {
    3 * 32 + 3 * ELEMENT_BYTES * assertions
}

/// Reads a proof front to back; every failure is a sentence for [`zk_core::Verdict::Malformed`].
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position.checked_add(count).filter(|end| *end <= self.bytes.len());
        let end =
            end.ok_or_else(|| format!("the proof ends early, at byte {}", self.bytes.len()))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, String> {
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

    /// Refuses bytes after the proof, so one proof has one encoding.
    pub(crate) fn finish(self) -> Result<(), String> {
        let extra = self.bytes.len() - self.position;
        if extra == 0 { Ok(()) } else { Err(format!("{extra} bytes follow the proof")) }
    }
}

fn write_elements(out: &mut Vec<u8>, elements: &[Fp]) {
    for element in elements {
        out.extend_from_slice(&element.encode());
    }
}

/// `c_1 ‖ c_2 ‖ c_3 ‖ (y_1 ‖ y_2 ‖ y_3) × A`.
pub(crate) fn write_first(out: &mut Vec<u8>, first: &FirstMessage) {
    for commitment in &first.commitments {
        out.extend_from_slice(commitment);
    }
    for shares in &first.output_shares {
        write_elements(out, shares);
    }
}

pub(crate) fn read_first(
    reader: &mut Reader<'_>,
    statement: &Statement,
) -> Result<FirstMessage, String> {
    let commitments = [reader.bytes32()?, reader.bytes32()?, reader.bytes32()?];
    let output_shares = (0..statement.assertions())
        .map(|_| {
            let shares = reader.elements(3)?;
            Ok([shares[0], shares[1], shares[2]])
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(FirstMessage { commitments, output_shares })
}

/// View `e` then view `e + 1`, each `k ‖ (P3 only: x_3 × S) ‖ z × M`.
pub(crate) fn write_response(out: &mut Vec<u8>, response: &Response) {
    for view in &response.views {
        out.extend_from_slice(&view.seed);
        write_elements(out, &view.input_shares);
        write_elements(out, &view.mul_outputs);
    }
}

pub(crate) fn read_response(
    reader: &mut Reader<'_>,
    statement: &Statement,
    challenge: Party,
) -> Result<Response, String> {
    let mut view = |party: Party| -> Result<OpenedView, String> {
        let seed = reader.bytes32()?;
        let inputs = if party == Party::P3 { statement.witness_values() } else { 0 };
        let input_shares = reader.elements(inputs)?;
        let mul_outputs = reader.elements(statement.multiplications())?;
        Ok(OpenedView { party, seed, input_shares, mul_outputs })
    };
    Ok(Response { views: [view(challenge)?, view(challenge.next())?] })
}
