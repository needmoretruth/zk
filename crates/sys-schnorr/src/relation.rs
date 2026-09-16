//! The one `sigma-proofs` relation a layout stands for, and the witness that satisfies it.

use sigma_proofs::LinearRelation;

use crate::field::PallasScalar;
use crate::layout::{Check, Layout};
use crate::pedersen::{Generators, Opening};
use crate::wrap::PallasPoint;

/// Every check as linear equations over one shared set of secret scalars, so the whole statement is
/// a single AND that `sigma-proofs` proves in one run.
///
/// - A product `c = a·b` with commitments `C_a`, `C_b`, `C_c`: secrets `(b, r_b, s)` with
///   `C_b = b·G + r_b·H` and `C_c = b·C_a + s·H`, where `s = r_c − b·r_a`. The first equation pins the
///   `b` used in the second to the committed one; the second then says `C_c` commits to `a·b`.
/// - A zero check on a commitment `C = v·G + r·H`: secret `t` with `C = t·H`. For `v ≠ 0` such a `t`
///   gives `log_G H = v / (t − r)`, so no prover who cannot take discrete logarithms finds one.
///
/// The images are points both sides compute ([`crate::affine::Affine::point`]), so the relation's
/// label, which `sigma-proofs` hashes into the challenge, binds them.
pub(crate) fn build(
    layout: &Layout,
    commitments: &[PallasPoint],
    generators: &Generators,
) -> LinearRelation<PallasPoint> {
    let mut relation = LinearRelation::new();
    let g = relation.allocate_element_with(generators.g);
    let h = relation.allocate_element_with(generators.h);
    for check in &layout.checks {
        match check {
            Check::Product { left, right, output } => {
                let [b, r_b, s] = relation.allocate_scalars();
                let c_b = relation.allocate_element_with(right.point(commitments, generators));
                let c_a = relation.allocate_element_with(left.point(commitments, generators));
                let c_c = relation.allocate_element_with(commitments[*output]);
                relation.append_equation(c_b, b * g + r_b * h);
                relation.append_equation(c_c, b * c_a + s * h);
            }
            Check::Zero(value) => {
                let t = relation.allocate_scalar();
                let c = relation.allocate_element_with(value.point(commitments, generators));
                relation.append_equation(c, t * h);
            }
            Check::KnownZero(_) => {}
        }
    }
    relation
}

/// The secret scalars in the order [`build`] allocates them, computed from the openings without
/// checking that they satisfy anything: a false claim yields a witness the verifier rejects.
pub(crate) fn witness(layout: &Layout, openings: &[Opening]) -> Vec<PallasScalar> {
    let mut scalars = Vec::with_capacity(layout.witness_scalars());
    for check in &layout.checks {
        match check {
            Check::Product { left, right, output } => {
                let (a, b) = (left.opening(openings), right.opening(openings));
                let c = openings[*output];
                scalars.extend([b.value, b.blinding, c.blinding - b.value * a.blinding]);
            }
            Check::Zero(value) => scalars.push(value.opening(openings).blinding),
            Check::KnownZero(_) => {}
        }
    }
    scalars
}
