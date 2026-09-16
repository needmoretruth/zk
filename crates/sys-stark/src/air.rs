//! The lowered circuit as a lambdaworks `AIR`.

use std::sync::Arc;

use lambdaworks_math::traits::AsBytes;
use stark_platinum_prover::PrimeField;
use stark_platinum_prover::constraints::boundary::{BoundaryConstraint, BoundaryConstraints};
use stark_platinum_prover::constraints::transition::TransitionConstraint;
use stark_platinum_prover::context::AirContext;
use stark_platinum_prover::lookup::BusPublicInputs;
use stark_platinum_prover::proof::options::ProofOptions;
use stark_platinum_prover::traits::{AIR, TransitionEvaluationContext};
use zk_circuit::lower::wide_air::{AirConstraint, WideAir};

use crate::field::{Stark252, Val};

/// What lambdaworks' `AIR::new` receives besides the trace length: the statement and its claim.
///
/// The prover and the verifier each build the AIR from nothing but these inputs, so the lowered
/// circuit has to travel here. Only the claimed public values are bytes (`AsBytes`), which seed the
/// Fiat–Shamir transcript.
#[derive(Clone, Debug)]
pub(crate) struct Statement {
    wide: Arc<WideAir<Stark252>>,
    public: Vec<Val>,
}

impl Statement {
    /// A claim about `wide`'s public inputs.
    pub(crate) fn new(wide: Arc<WideAir<Stark252>>, public: Vec<Val>) -> Self {
        Self { wide, public }
    }

    /// Trace columns: one per wire.
    pub(crate) fn columns(&self) -> usize {
        self.wide.num_columns()
    }
}

/// The public values in lambdaworks' own byte form, one after another.
///
/// lambdaworks' README asks for the public inputs to seed the transcript (otherwise Fiat–Shamir is
/// the weak kind a prover can exploit), and its Stone-compatible serializer seeds it with
/// `public_inputs.as_bytes()`; this is that value for the museum's statement.
impl AsBytes for Statement {
    fn as_bytes(&self) -> Vec<u8> {
        self.public.iter().flat_map(AsBytes::as_bytes).collect()
    }
}

/// One gate of the wide row, asserted on every row of the trace.
struct GateConstraint {
    index: usize,
    wide: Arc<WideAir<Stark252>>,
}

impl GateConstraint {
    fn constraint(&self) -> &AirConstraint<Stark252> {
        &self.wide.constraints()[self.index]
    }
}

impl TransitionConstraint<PrimeField, PrimeField> for GateConstraint {
    /// The gate's own degree. lambdaworks at this commit reads it nowhere; the composition
    /// polynomial is sized by [`CircuitAir::composition_poly_degree_bound`].
    fn degree(&self) -> usize {
        self.constraint().degree()
    }

    fn constraint_idx(&self) -> usize {
        self.index
    }

    /// None: the constraint reads one row only, so it holds on the last row as on every other.
    fn end_exemptions(&self) -> usize {
        0
    }

    fn evaluate(
        &self,
        context: &TransitionEvaluationContext<PrimeField, PrimeField>,
        out: &mut [Val],
    ) {
        let (TransitionEvaluationContext::Prover { frame, .. }
        | TransitionEvaluationContext::Verifier { frame, .. }) = context;
        let row = frame.get_evaluation_step(0);
        out[self.index] =
            folded(self.constraint(), |column| *row.get_main_evaluation_element(0, column));
    }
}

/// `Σ c·col + Σ c·col_i·col_j + constant`, reading columns through `cell`.
fn folded(constraint: &AirConstraint<Stark252>, cell: impl Fn(usize) -> Val) -> Val {
    let linear = constraint
        .linear
        .iter()
        .fold(constraint.constant.0, |acc, (column, c)| acc + cell(*column) * c.0);
    constraint.quadratic.iter().fold(linear, |acc, (i, j, c)| acc + cell(*i) * cell(*j) * c.0)
}

/// A circuit lowered to one wide row, repeated over the trace.
///
/// Every gate is a transition constraint that reads only the current row (frame offsets `[0]`) and
/// exempts no row, so it is enforced on every row, the last included. Public inputs are boundary
/// constraints on row 0, which therefore satisfies every gate with the claimed public values on its
/// own; nothing needs to tie the rows together.
///
/// This is the museum's wide AIR as it stands, measured against what Winterfell 0.13.1 forced on its
/// exhibit: lambdaworks at this commit never compares declared degrees with the trace, asserts
/// nothing about the DEEP composition polynomial's degree (an all-constant trace proves and
/// verifies), and has no column limit (`pool-spend`'s 1008 columns prove), so there is no step
/// counter, no stand-in degree and no unsupported statement. Its one limit on the trace is FRI's,
/// which `stark::TRACE_LENGTH` explains.
pub(crate) struct CircuitAir {
    context: AirContext,
    trace_length: usize,
    statement: Statement,
    constraints: Vec<Box<dyn TransitionConstraint<PrimeField, PrimeField>>>,
}

impl AIR for CircuitAir {
    type Field = PrimeField;
    type FieldExtension = PrimeField;
    type PublicInputs = Statement;

    fn step_size(&self) -> usize {
        1
    }

    fn new(trace_length: usize, statement: &Statement, options: &ProofOptions) -> Self {
        let constraints: Vec<Box<dyn TransitionConstraint<PrimeField, PrimeField>>> = (0
            ..statement.wide.num_constraints())
            .map(|index| {
                let gate = GateConstraint { index, wide: Arc::clone(&statement.wide) };
                Box::new(gate) as Box<dyn TransitionConstraint<PrimeField, PrimeField>>
            })
            .collect();
        let context = AirContext {
            proof_options: options.clone(),
            trace_columns: statement.columns(),
            transition_offsets: vec![0],
            num_transition_constraints: constraints.len(),
        };
        Self { context, trace_length, statement: statement.clone(), constraints }
    }

    fn trace_layout(&self) -> (usize, usize) {
        (self.statement.columns(), 0)
    }

    /// The highest gate degree times the trace length, as lambdaworks' own examples size it (its
    /// quadratic AIR declares `2 · trace_length`); the prover splits the composition polynomial
    /// into that many trace lengths' worth of parts.
    fn composition_poly_degree_bound(&self) -> usize {
        self.statement.wide.max_degree().max(1) * self.trace_length
    }

    fn boundary_constraints(
        &self,
        _rap_challenges: &[Val],
        _bus_public_inputs: Option<&BusPublicInputs<PrimeField>>,
    ) -> BoundaryConstraints<PrimeField> {
        let columns = self.statement.wide.public_columns();
        let constraints = columns
            .iter()
            .zip(&self.statement.public)
            .map(|(column, value)| BoundaryConstraint::new_main(*column, 0, *value))
            .collect();
        BoundaryConstraints::from_constraints(constraints)
    }

    fn transition_constraints(
        &self,
    ) -> &Vec<Box<dyn TransitionConstraint<PrimeField, PrimeField>>> {
        &self.constraints
    }

    fn context(&self) -> &AirContext {
        &self.context
    }

    fn trace_length(&self) -> usize {
        self.trace_length
    }

    fn pub_inputs(&self) -> &Statement {
        &self.statement
    }
}
