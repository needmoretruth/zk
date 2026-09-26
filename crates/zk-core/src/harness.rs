//! The one procedure that runs, times and attacks every in-process system the same way.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use zk_examples::{ExampleId, InstanceKind};

use crate::catalog::Mode;
use crate::error::{RunError, SystemError};
use crate::report::{
    AttackKind, AttackOutcome, AttackReport, PROOF_HEAD_BYTES, RunReport, Timings,
};
use crate::scan::scan_secrets;
use crate::system::{
    Control, FieldBytes, Instance, Prepared, ProofSystem, Proven, RunOptions, Stage, Support,
    Verdict,
};

/// Why an attack did not apply, as phrase keys a screen turns into words.
pub mod not_applicable {
    /// The proof has no bytes to flip.
    pub const EMPTY_PROOF: &str = "empty-proof";
    /// The example has no public input to change.
    pub const NO_PUBLIC_INPUT: &str = "no-public-input";
    /// A live conversation leaves no proof object behind.
    pub const LIVE_CONVERSATION: &str = "live-conversation";
}

/// Runs `example` on `system`: setup, honest proof, verification, then the attack set.
///
/// A panic inside the system's code, which upstream crates use for errors they did not expect, ends
/// this run as a failure at the stage it reached rather than taking every other run down with it.
pub fn run<S: ProofSystem + ?Sized>(
    system: &S,
    example: ExampleId,
    options: &RunOptions,
    control: &Control,
) -> Result<RunReport, RunError> {
    let reached = Arc::new(Mutex::new(None::<Stage>));
    let watched = {
        let reached = Arc::clone(&reached);
        control.also_reporting(move |stage| {
            *reached.lock().unwrap_or_else(PoisonError::into_inner) = Some(stage);
        })
    };
    let outcome =
        panic::catch_unwind(AssertUnwindSafe(|| run_unguarded(system, example, options, &watched)));
    outcome.unwrap_or_else(|payload| {
        let stage = reached.lock().unwrap_or_else(PoisonError::into_inner).unwrap_or(Stage::Setup);
        let error = SystemError::Failed(format!("panicked: {}", panic_text(payload.as_ref())));
        Err(RunError::Failed { stage, error })
    })
}

/// The text a panic carried.
fn panic_text(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(text), _) => (*text).to_string(),
        (_, Some(text)) => text.clone(),
        _ => "no message".to_string(),
    }
}

fn run_unguarded<S: ProofSystem + ?Sized>(
    system: &S,
    example: ExampleId,
    options: &RunOptions,
    control: &Control,
) -> Result<RunReport, RunError> {
    if let Support::Unsupported { reason } = system.support(example) {
        return Err(RunError::Unsupported { reason });
    }
    let seed = match options.seed {
        Some(seed) => seed,
        None => fresh_seed()?,
    };
    let meta = system.meta();
    control.report(Stage::Setup);
    let started = Instant::now();
    let mut prepared = system.prepare(example, control).map_err(|e| fail(Stage::Setup, None, e))?;
    let setup_micros = micros(started);
    let honest = Instance { example, kind: InstanceKind::Honest, seed };
    let context = Context { example, meta_field: meta.field, id: meta.id, setup_micros };
    match meta.mode {
        Mode::NonInteractive => {
            non_interactive(prepared.as_mut(), &honest, options, control, context)
        }
        Mode::Interactive => interactive(prepared.as_mut(), &honest, options, control, context),
    }
}

struct Context {
    example: ExampleId,
    meta_field: &'static str,
    id: &'static str,
    setup_micros: u64,
}

fn non_interactive(
    prepared: &mut dyn Prepared,
    honest: &Instance,
    options: &RunOptions,
    control: &Control,
    context: Context,
) -> Result<RunReport, RunError> {
    control.report(Stage::Prove);
    let started = Instant::now();
    let proven =
        prepared.prove(honest, control).map_err(|e| fail(Stage::Prove, Some(Stage::Setup), e))?;
    let prove_micros = micros(started);
    control.report(Stage::Verify);
    let started = Instant::now();
    let verdict = prepared
        .verify(&proven.public, &proven.proof, control)
        .map_err(|e| fail(Stage::Verify, Some(Stage::Prove), e))?;
    let verify_micros = micros(started);
    let attacks = if options.attacks {
        attack_non_interactive(prepared, honest, &proven, control)?
    } else {
        Vec::new()
    };
    let secret_scan = scan_secrets(prepared, context.example, &proven.secrets, &proven.proof);
    Ok(RunReport {
        system: context.id.to_string(),
        example: context.example.id().to_string(),
        field: context.meta_field.to_string(),
        mode: Mode::NonInteractive.into(),
        shape: prepared.shape(),
        timings: Timings { setup_micros: context.setup_micros, prove_micros, verify_micros },
        setup_bytes: prepared.setup_bytes(),
        proof_bytes: proven.proof.len() as u64,
        proof_head: proven.proof.iter().take(PROOF_HEAD_BYTES).copied().collect(),
        rounds: None,
        verdict,
        attacks,
        secret_scan,
    })
}

fn attack_non_interactive(
    prepared: &mut dyn Prepared,
    honest: &Instance,
    proven: &Proven,
    control: &Control,
) -> Result<Vec<AttackReport>, RunError> {
    let mut reports = Vec::with_capacity(AttackKind::ALL.len());

    let kind = AttackKind::FlipProofByte;
    control.report(Stage::Attack(kind));
    let (outcome, offset) = if proven.proof.is_empty() {
        (AttackOutcome::NotApplicable(not_applicable::EMPTY_PROOF.to_string()), None)
    } else {
        let offset = prepared.tamper_offset(&proven.proof).min(proven.proof.len() - 1);
        let mut flipped = proven.proof.clone();
        flipped[offset] ^= 0x01;
        let verdict =
            prepared.verify(&proven.public, &flipped, control).map_err(|e| attack_fail(kind, e))?;
        (from_verdict(verdict), Some(offset as u64))
    };
    reports.push(AttackReport { kind, outcome, offset });

    let kind = AttackKind::BumpPublicInput;
    control.report(Stage::Attack(kind));
    let outcome = match bumped(prepared, honest.example, &proven.public)
        .map_err(|e| attack_fail(kind, e))?
    {
        None => AttackOutcome::NotApplicable(not_applicable::NO_PUBLIC_INPUT.to_string()),
        Some(public) => from_verdict(
            prepared.verify(&public, &proven.proof, control).map_err(|e| attack_fail(kind, e))?,
        ),
    };
    reports.push(AttackReport { kind, outcome, offset: None });

    let kind = AttackKind::DishonestWitness;
    control.report(Stage::Attack(kind));
    let dishonest = Instance { kind: InstanceKind::Dishonest, ..*honest };
    let outcome = match prepared.prove(&dishonest, control) {
        Err(SystemError::Unsatisfied(why)) => AttackOutcome::ProverRefused(why),
        Err(error) => return Err(attack_fail(kind, error)),
        Ok(forged) => from_verdict(
            prepared
                .verify(&forged.public, &forged.proof, control)
                .map_err(|e| attack_fail(kind, e))?,
        ),
    };
    reports.push(AttackReport { kind, outcome, offset: None });
    Ok(reports)
}

fn interactive(
    prepared: &mut dyn Prepared,
    honest: &Instance,
    options: &RunOptions,
    control: &Control,
    context: Context,
) -> Result<RunReport, RunError> {
    control.report(Stage::Prove);
    let started = Instant::now();
    let played = prepared
        .interact(honest, None, control)
        .map_err(|e| fail(Stage::Prove, Some(Stage::Setup), e))?;
    let prove_micros = micros(started);
    let mut attacks = Vec::new();
    if options.attacks {
        let kind = AttackKind::FlipProofByte;
        attacks.push(AttackReport {
            kind,
            outcome: AttackOutcome::NotApplicable(not_applicable::LIVE_CONVERSATION.to_string()),
            offset: None,
        });
        let kind = AttackKind::BumpPublicInput;
        control.report(Stage::Attack(kind));
        let outcome = match bumped(prepared, honest.example, &played.public)
            .map_err(|e| attack_fail(kind, e))?
        {
            None => AttackOutcome::NotApplicable(not_applicable::NO_PUBLIC_INPUT.to_string()),
            Some(public) => from_verdict(
                prepared
                    .interact(honest, Some(&public), control)
                    .map_err(|e| attack_fail(kind, e))?
                    .verdict,
            ),
        };
        attacks.push(AttackReport { kind, outcome, offset: None });
        let kind = AttackKind::DishonestWitness;
        control.report(Stage::Attack(kind));
        let dishonest = Instance { kind: InstanceKind::Dishonest, ..*honest };
        let outcome = from_verdict(
            prepared.interact(&dishonest, None, control).map_err(|e| attack_fail(kind, e))?.verdict,
        );
        attacks.push(AttackReport { kind, outcome, offset: None });
    }
    let secret_scan = scan_secrets(prepared, context.example, &played.secrets, &played.transcript);
    Ok(RunReport {
        system: context.id.to_string(),
        example: context.example.id().to_string(),
        field: context.meta_field.to_string(),
        mode: Mode::Interactive.into(),
        shape: prepared.shape(),
        timings: Timings { setup_micros: context.setup_micros, prove_micros, verify_micros: 0 },
        setup_bytes: prepared.setup_bytes(),
        proof_bytes: played.transcript.len() as u64,
        proof_head: played.transcript.iter().take(PROOF_HEAD_BYTES).copied().collect(),
        rounds: Some(played.rounds),
        verdict: played.verdict,
        attacks,
        secret_scan,
    })
}

/// The public inputs with the example's falsifying input moved on by one ([`ExampleId::falsifying_public_index`]).
fn bumped(
    prepared: &dyn Prepared,
    example: ExampleId,
    public: &[FieldBytes],
) -> Result<Option<Vec<FieldBytes>>, SystemError> {
    let index = example.falsifying_public_index();
    let Some(target) = public.get(index) else { return Ok(None) };
    let mut changed = public.to_vec();
    changed[index] = prepared.bump_public(target)?;
    Ok(Some(changed))
}

fn from_verdict(verdict: Verdict) -> AttackOutcome {
    match verdict {
        Verdict::Accepted => AttackOutcome::Accepted,
        Verdict::Rejected => AttackOutcome::Rejected,
        Verdict::Malformed(why) => AttackOutcome::Malformed(why),
    }
}

fn fail(stage: Stage, after: Option<Stage>, error: SystemError) -> RunError {
    match error {
        SystemError::Cancelled => RunError::Cancelled { after },
        error => RunError::Failed { stage, error },
    }
}

fn attack_fail(kind: AttackKind, error: SystemError) -> RunError {
    fail(Stage::Attack(kind), Some(Stage::Verify), error)
}

fn micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn fresh_seed() -> Result<[u8; 32], RunError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| RunError::Randomness(e.to_string()))?;
    Ok(seed)
}
