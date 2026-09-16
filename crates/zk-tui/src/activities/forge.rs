//! `/forge bctv14`: CVE-2019-7167 replayed on the BCTV14 teaching implementation.

use serde_json::json;
use sys_bctv14::cve_2019_7167::{forge, forge_without_redundant_elements, generate_flawed};
use sys_bctv14::{Bn254Fr, adapter, prover, verifier};
use zk_circuit::lower::r1cs::R1cs;
use zk_core::{Control, ExampleId, InstanceKind};
use zk_i18n::Language;

use super::{CEREMONY_BEAT, Pace, Steps, fresh_seed};
use crate::activity::{Activity, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::fill;
use crate::phrases::forge::Msg as M;
use crate::views::forge as view;

/// A known forgery the museum can replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForgeTarget {
    /// CVE-2019-7167: redundant setup elements let a BCTV14 proof be rewritten for any input.
    Bctv14,
}

impl ForgeTarget {
    /// Every target.
    pub const ALL: [ForgeTarget; 1] = [Self::Bctv14];

    /// The word typed after `/forge`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Bctv14 => "bctv14",
        }
    }

    /// The target a typed word names.
    pub fn from_key(key: &str) -> Option<ForgeTarget> {
        Self::ALL.into_iter().find(|target| target.key() == key)
    }
}

/// `/forge bctv14`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForgeRun {
    /// What to forge.
    pub target: ForgeTarget,
    /// The language of every cell.
    pub language: Language,
    /// Whether steps wait for the reader.
    pub pace: Pace,
}

/// Steps of the forgery: keys, the honest proof, the honest proof against the false input, the
/// rewrite, and the rewrite against corrected keys.
const STEPS: usize = 5;

impl Activity for ForgeRun {
    fn subject(&self) -> String {
        sys_bctv14::META.name.to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let language = self.language;
        let cell = outbox.open(Entry::new(Kind::Story, view::opening(language)));
        let steps = Steps::new(
            outbox,
            control,
            (self.pace, CEREMONY_BEAT),
            (cell, language),
            (STEPS, view::stopped),
        );
        let failure = match self.bctv14(&steps) {
            Err(why) => Some(fill(M::Failed.text(language), &[("why", &why)])),
            Ok(()) if steps.unexpected.get() => Some(M::Unexpected.text(language).to_string()),
            Ok(()) => None,
        };
        if let Some(text) = failure {
            outbox.open(Entry::text(Kind::Error, text));
        }
    }
}

fn record(step: &str, accepted: Option<bool>) -> serde_json::Value {
    json!({ "activity": "forge", "target": "bctv14", "step": step, "accepted": accepted })
}

impl ForgeRun {
    fn bctv14(&self, steps: &Steps<'_>) -> Result<(), String> {
        let language = self.language;
        steps.outbox.status(M::WorkingKeys.text(language));
        let example = ExampleId::OnePlusOne;
        let circuit = example.circuit::<Bn254Fr>().map_err(|e| format!("{e:?}"))?;
        let r1cs = R1cs::from_circuit(&circuit);
        let mut rng = adapter::os_rng().map_err(|e| e.to_string())?;
        let (pk, vk, flaw) = generate_flawed(&r1cs, &mut rng).ok_or("the key generator failed")?;
        let extra = flaw.a_prime_ic.len();
        steps.show(view::keys(extra, language), record("flawed-keys", None), true);
        if !steps.next(M::WorkingHonest.text(language)) {
            return Ok(());
        }
        let seed = fresh_seed()?;
        let honest = example.instance::<Bn254Fr>(InstanceKind::Honest, &seed);
        let wires = circuit.evaluate(&honest).map_err(|e| format!("{e:?}"))?;
        let z: Vec<_> = r1cs.assignment(&wires).iter().map(|v| v.inner()).collect();
        let x_true: Vec<_> = honest.public.iter().map(|v| v.inner()).collect();
        let proof = prover::prove(&pk, &r1cs, &z, &mut rng).ok_or("the prover failed")?;
        let accepted = verifier::verify(&vk, &x_true, &proof);
        let beat = view::honest(accepted, language);
        steps.show(beat, record("honest-proof", Some(accepted)), accepted);
        if !steps.next(M::WorkingFalse.text(language)) {
            return Ok(());
        }
        let false_claim = example.instance::<Bn254Fr>(InstanceKind::Dishonest, &seed);
        let x_false: Vec<_> = false_claim.public.iter().map(|v| v.inner()).collect();
        let accepted = verifier::verify(&vk, &x_false, &proof);
        let beat = view::honest_for_false(accepted, language);
        steps.show(beat, record("honest-proof-false-input", Some(accepted)), !accepted);
        if !steps.next(M::WorkingRewrite.text(language)) {
            return Ok(());
        }
        let forged = forge(&flaw, &vk, &proof, &x_true, &x_false).ok_or("the rewrite failed")?;
        let accepted = verifier::verify(&vk, &x_false, &forged);
        let beat = view::rewritten(accepted, language);
        steps.show(beat, record("rewritten", Some(accepted)), accepted);
        if !steps.next(M::WorkingCorrected.text(language)) {
            return Ok(());
        }
        let attempt = forge_without_redundant_elements(&vk, &proof, &x_true, &x_false)
            .ok_or("the rewrite failed")?;
        let accepted = verifier::verify(&vk, &x_false, &attempt);
        let beat = view::corrected(accepted, language);
        steps.show(beat, record("corrected-keys", Some(accepted)), !accepted);
        Ok(())
    }
}
