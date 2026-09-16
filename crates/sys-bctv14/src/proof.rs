//! The proof object and its serialization.
//!
//! A proof is the eight group elements `pi = (A, A', B, B', C, C', K, H)`: `B` in G2, the rest in
//! G1 (matching the Zcash Sprout ordering of §5.4.10). They are written back to back in arkworks
//! compressed form, in that fixed order, so the on-wire size is honest.

use ark_bn254::{G1Affine, G2Affine};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// A BCTV14 proof: three knowledge commitments, the same-coefficient element and the H element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// `pi_A = rho_A A_mid(t) G1`.
    pub a: G1Affine,
    /// `pi'_A = alpha_A rho_A A_mid(t) G1`.
    pub a_prime: G1Affine,
    /// `pi_B = rho_B B(t) G2`.
    pub b: G2Affine,
    /// `pi'_B = alpha_B rho_B B(t) G1`.
    pub b_prime: G1Affine,
    /// `pi_C = rho_C C(t) G1`.
    pub c: G1Affine,
    /// `pi'_C = alpha_C rho_C C(t) G1`.
    pub c_prime: G1Affine,
    /// `pi_K = beta (rho_A A + rho_B B + rho_C C)(t) G1`.
    pub k: G1Affine,
    /// `pi_H = H(t) G1`.
    pub h: G1Affine,
}

impl Proof {
    /// Writes the eight elements compressed, in order `A, A', B, B', C, C', K, H`. Writing to a
    /// `Vec` cannot fail, so the ignored results below never hide a real error.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_len());
        let _ = self.a.serialize_compressed(&mut bytes);
        let _ = self.a_prime.serialize_compressed(&mut bytes);
        let _ = self.b.serialize_compressed(&mut bytes);
        let _ = self.b_prime.serialize_compressed(&mut bytes);
        let _ = self.c.serialize_compressed(&mut bytes);
        let _ = self.c_prime.serialize_compressed(&mut bytes);
        let _ = self.k.serialize_compressed(&mut bytes);
        let _ = self.h.serialize_compressed(&mut bytes);
        bytes
    }

    /// Parses the eight elements back; `None` on any malformed or off-curve encoding.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let g1 = G1Affine::default().compressed_size();
        let g2 = G2Affine::default().compressed_size();
        if bytes.len() != 7 * g1 + g2 {
            return None;
        }
        let mut cursor = 0;
        let read_g1 = |cursor: &mut usize| -> Option<G1Affine> {
            let point = G1Affine::deserialize_compressed(&bytes[*cursor..*cursor + g1]).ok()?;
            *cursor += g1;
            Some(point)
        };
        let a = read_g1(&mut cursor)?;
        let a_prime = read_g1(&mut cursor)?;
        let b = G2Affine::deserialize_compressed(&bytes[cursor..cursor + g2]).ok()?;
        cursor += g2;
        let b_prime = read_g1(&mut cursor)?;
        let c = read_g1(&mut cursor)?;
        let c_prime = read_g1(&mut cursor)?;
        let k = read_g1(&mut cursor)?;
        let h = read_g1(&mut cursor)?;
        Some(Self { a, a_prime, b, b_prime, c, c_prime, k, h })
    }

    /// The serialized length, `7 * |G1| + |G2|` compressed bytes.
    pub fn byte_len() -> usize {
        7 * G1Affine::default().compressed_size() + G2Affine::default().compressed_size()
    }
}
