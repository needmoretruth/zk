//! One statement ready to prove: the teaching compiler and `sigma-proofs` behind the museum's contract.

use zk_circuit::{Assignment, Circuit};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, Prepared, Proven, ShapeForm,
    SystemError, Verdict,
};

use crate::field::{PallasScalar, decode, encode};
use crate::layout::Layout;
use crate::pedersen::Generators;
use crate::{prove, verify};

/// A statement's circuit, the generators, and the size of its compiled form.
pub(crate) struct Ready {
    example: ExampleId,
    circuit: Circuit<PallasScalar>,
    generators: Generators,
    shape: CircuitShape,
}

impl Ready {
    /// Builds the circuit and derives `H`. There is no other setup: a sigma protocol needs no keys.
    pub(crate) fn build(example: ExampleId, control: &Control) -> Result<Self, SystemError> {
        control.checkpoint()?;
        let circuit =
            example.circuit::<PallasScalar>().map_err(|e| SystemError::Failed(e.to_string()))?;
        control.checkpoint()?;
        let generators = Generators::derive();
        let shape = shape_of(&circuit, example);
        Ok(Self { example, circuit, generators, shape })
    }

    fn prove_values(
        &self,
        assignment: &Assignment<PallasScalar>,
        control: &Control,
    ) -> Result<Proven, SystemError> {
        control.checkpoint()?;
        let proof = prove::prove(&self.circuit, self.example, &self.generators, assignment)?;
        Ok(Proven {
            proof,
            public: assignment.public.iter().map(|value| encode(*value)).collect(),
            secrets: assignment.private.iter().map(|value| encode(*value)).collect(),
        })
    }
}

/// Counts for the example's sample public inputs. Public values change the layout only where a
/// public factor of zero strips every commitment from a value (a sudoku cell without a given), which
/// turns a later product linear or a zero check into one the verifier does itself, so another
/// puzzle's givens can give other counts.
fn shape_of(circuit: &Circuit<PallasScalar>, example: ExampleId) -> CircuitShape {
    let layout = Layout::new(circuit, &example.honest::<PallasScalar>().public);
    CircuitShape {
        form: ShapeForm::Native,
        counts: vec![
            ("commitments".into(), layout.committed.len() as u64),
            ("constraints".into(), layout.equations() as u64),
            ("variables".into(), layout.witness_scalars() as u64),
        ],
    }
}

// `secret_encodings` keeps the default: scalars are written big-endian and points compressed, never
// in Montgomery form, and the default already searches both byte orders.
impl Prepared for Ready {
    fn shape(&self) -> CircuitShape {
        self.shape.clone()
    }

    /// Evaluates the sample claim without checking it and proves whatever the wires hold.
    fn prove(&mut self, instance: &Instance, control: &Control) -> Result<Proven, SystemError> {
        let assignment = instance.example.instance::<PallasScalar>(instance.kind, &instance.seed);
        self.prove_values(&assignment, control)
    }

    /// Decodes the caller's inputs and proves them exactly as [`Prepared::prove`] proves a sample
    /// claim: unchecked, so a false assignment still reaches `sigma-proofs`.
    fn prove_assignment(
        &mut self,
        public: &[FieldBytes],
        private: &[FieldBytes],
        control: &Control,
    ) -> Result<Proven, SystemError> {
        let decode_all = |values: &[FieldBytes]| -> Result<Vec<PallasScalar>, SystemError> {
            values
                .iter()
                .map(|bytes| {
                    decode(bytes).ok_or_else(|| {
                        SystemError::Failed("an input is not a canonical Pallas scalar".into())
                    })
                })
                .collect()
        };
        let assignment = Assignment { public: decode_all(public)?, private: decode_all(private)? };
        self.prove_values(&assignment, control)
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        proof: &[u8],
        control: &Control,
    ) -> Result<Verdict, SystemError> {
        control.checkpoint()?;
        Ok(verify::verify(&self.circuit, self.example, &self.generators, public, proof))
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let value = decode(value).ok_or_else(|| {
            SystemError::Failed("a public input is not a canonical Pallas scalar".into())
        })?;
        Ok(encode(PallasScalar(value.0 + pasta_curves::pallas::Scalar::from(1))))
    }

    /// The last byte of the proof: the least significant byte of the last response scalar, which is
    /// written big-endian.
    ///
    /// The verifier multiplies every response into its equations, and the last one belongs to a
    /// product's `s` or a zero check's `t`, both multiplied by `H`. Flipping the lowest bit moves the
    /// scalar by one and keeps it canonical, so the proof still decodes, verification runs to the
    /// end, and the equation fails. A byte inside a point would mostly stop at decoding instead.
    fn tamper_offset(&self, proof: &[u8]) -> usize {
        proof.len().saturating_sub(1)
    }
}
