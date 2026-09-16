//! `/ceremony`: why a pairing setup must forget its secrets, and why one honest participant among
//! many is enough, run on this machine with the real curve.

use serde_json::json;
use zk_ceremony::kzg::{Collusion, commit, open, verify as kzg_verify};
use zk_ceremony::tau::{
    Check, Contribution, KeptSecret, Participant, Srs, cheat, digest, verify_chain,
};
use zk_ceremony::toxic::{self, ToxicWaste, false_claim, forge, keys};
use zk_ceremony::{OsRng, Scalar};
use zk_core::Control;
use zk_i18n::Language;

use super::{CEREMONY_BEAT, Pace, Steps, fresh_seed, hex};
use crate::activity::{Activity, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::ceremony::Msg as M;
use crate::phrases::fill;
use crate::views::ceremony as view;

/// Participants when none are asked for.
pub const DEFAULT_PARTICIPANTS: usize = zk_ceremony::tau::DEFAULT_PARTICIPANTS;
/// Most participants one ceremony seats.
pub const MAX_PARTICIPANTS: u32 = 100;
/// The polynomial the colluders commit to, lowest degree first: `3 + X + 4X² + X³ + 5X⁴`.
pub(crate) const COEFFICIENTS: [u64; 5] = [3, 1, 4, 1, 5];
/// Where the colluders open it.
pub(crate) const POINT: u64 = 2;
/// How far the value they claim is from the true one.
pub(crate) const LIE: u64 = 1_000;
/// The powers a cheat swaps for a random point, with the check the design says catches each: a
/// power past the second breaks the sequence, the second is `[τ]G1` itself, the first must be the
/// generator.
const REPLACED_POWERS: [(usize, Check); 3] =
    [(17, Check::ConsistentPowers), (1, Check::BuildsOnPrevious), (0, Check::WellFormed)];

/// Which fact of the ceremony to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CeremonyPart {
    /// Keys made from known toxic waste forge a Groth16 proof of a false claim.
    Toxic,
    /// Powers of Tau: turns anyone can check, and the cheats the checks catch.
    Tau,
    /// Every participant keeps their secret: a KZG opening to a wrong value verifies.
    Collude,
}

impl CeremonyPart {
    /// Every part.
    pub const ALL: [CeremonyPart; 3] = [Self::Toxic, Self::Tau, Self::Collude];

    /// The word typed after `/ceremony`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Toxic => "toxic",
            Self::Tau => "tau",
            Self::Collude => "collude",
        }
    }

    /// The part a typed word names.
    pub fn from_key(key: &str) -> Option<CeremonyPart> {
        Self::ALL.into_iter().find(|part| part.key() == key)
    }
}

/// `/ceremony [toxic|tau|collude] [--participants N]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CeremonyRun {
    /// Which part.
    pub part: CeremonyPart,
    /// Participants taking turns; the toxic-waste part has none.
    pub participants: usize,
    /// The language of every cell.
    pub language: Language,
    /// Whether turns wait for the reader.
    pub pace: Pace,
}

impl Activity for CeremonyRun {
    fn subject(&self) -> String {
        M::Subject.text(self.language).to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let language = self.language;
        let cell = outbox.open(Entry::new(Kind::Story, view::opening(&self)));
        let total = match self.part {
            CeremonyPart::Toxic => 4,
            CeremonyPart::Tau => 2 * self.participants + 2 + REPLACED_POWERS.len(),
            CeremonyPart::Collude => self.participants + 3,
        };
        let steps = Steps::new(
            outbox,
            control,
            (self.pace, CEREMONY_BEAT),
            (cell, language),
            (total, view::stopped),
        );
        let outcome = match self.part {
            CeremonyPart::Toxic => self.toxic(&steps),
            CeremonyPart::Tau => self.tau(&steps),
            CeremonyPart::Collude => self.collude(&steps),
        };
        let failure = match outcome {
            Err(why) => Some(fill(M::Failed.text(language), &[("why", &why)])),
            Ok(()) if steps.unexpected.get() => Some(M::Unexpected.text(language).to_string()),
            Ok(()) => None,
        };
        if let Some(text) = failure {
            outbox.open(Entry::text(Kind::Error, text));
        }
    }
}

fn record(part: CeremonyPart, mut value: serde_json::Value) -> serde_json::Value {
    value["activity"] = json!("ceremony");
    value["part"] = json!(part.key());
    value
}

fn debug(error: impl core::fmt::Debug) -> String {
    format!("{error:?}")
}

impl CeremonyRun {
    fn working(&self, msg: M, number: usize) -> String {
        let (number, total) = (number.to_string(), self.participants.to_string());
        fill(msg.text(self.language), &[("number", &number), ("total", &total)])
    }

    fn toxic(&self, steps: &Steps<'_>) -> Result<(), String> {
        let (language, part) = (self.language, self.part);
        steps.outbox.status(M::WorkingKeys.text(language));
        let waste = ToxicWaste::sample();
        let parameters = keys(&waste).map_err(debug)?;
        steps.show(view::toxic_keys(language), record(part, json!({ "step": "keys" })), true);
        if !steps.next(M::WorkingClaim.text(language)) {
            return Ok(());
        }
        let claim = false_claim(&fresh_seed()?);
        let fields = record(part, json!({ "step": "false-claim" }));
        steps.show(view::toxic_claim(language), fields, true);
        if !steps.next(M::WorkingForge.text(language)) {
            return Ok(());
        }
        let forged = forge(&parameters.vk, &waste, &claim).map_err(debug)?;
        let accepted = toxic::verify(&parameters.vk, &forged, &claim);
        let points = [
            hex(&forged.a.to_compressed()),
            hex(&forged.b.to_compressed()),
            hex(&forged.c.to_compressed()),
        ];
        let fields = record(part, json!({ "step": "forged", "accepted": accepted }));
        steps.show(view::toxic_forged(&points, accepted, language), fields, accepted);
        if !steps.next(M::WorkingGuess.text(language)) {
            return Ok(());
        }
        let guessed = ToxicWaste { delta: ToxicWaste::sample().delta, ..waste };
        let wrong = forge(&parameters.vk, &guessed, &claim).map_err(debug)?;
        let accepted = toxic::verify(&parameters.vk, &wrong, &claim);
        let fields = record(part, json!({ "step": "guessed-delta", "accepted": accepted }));
        steps.show(view::toxic_guessed(accepted, language), fields, !accepted);
        Ok(())
    }

    fn tau(&self, steps: &Steps<'_>) -> Result<(), String> {
        let (language, part) = (self.language, self.part);
        let initial = Srs::initial();
        steps.outbox.append(steps.cell, view::tau_start(&digest(&initial), language));
        let mut chain: Vec<(Contribution, Srs)> = Vec::new();
        for number in 1..=self.participants {
            if !steps.next(self.working(M::WorkingTurn, number)) {
                return Ok(());
            }
            let previous = chain.last().map_or(&initial, |(_, srs)| srs);
            let (next, contribution) = Participant::contribute(previous, &mut OsRng);
            let fingerprint = digest(&next);
            let fields =
                json!({ "step": "turn", "participant": number, "fingerprint": hex(&fingerprint) });
            steps.show(
                view::turn(number, &fingerprint, false, language),
                record(part, fields),
                true,
            );
            chain.push((contribution, next));
        }
        for number in 1..=chain.len() {
            if !steps.next(self.working(M::WorkingCheck, number)) {
                return Ok(());
            }
            let previous = if number == 1 { &initial } else { &chain[number - 2].1 };
            let failed = verify_chain(previous, &chain[number - 1..number]).err().map(|e| e.check);
            let fields = json!({
                "step": "check",
                "participant": number,
                "failed_check": failed.map(view::check_key),
            });
            let beat = view::checked(number, failed, language);
            steps.show(beat, record(part, fields), failed.is_none());
        }
        let whole = verify_chain(&initial, &chain).is_ok();
        let fields = json!({ "step": "chain", "participants": chain.len(), "verified": whole });
        steps.show(view::chain_verdict(chain.len(), whole, language), record(part, fields), whole);
        let last = chain.last().map_or(initial, |(_, srs)| srs.clone());
        self.cheats(steps, &last, chain.len() + 1)
    }

    /// The design's two cheats, each played as the participant after an honest chain.
    fn cheats(&self, steps: &Steps<'_>, last: &Srs, number: usize) -> Result<(), String> {
        let (language, part) = (self.language, self.part);
        if !steps.next(M::WorkingCheat.text(language)) {
            return Ok(());
        }
        let (next, contribution) = cheat::ignore_previous(last, &mut OsRng);
        let caught = verify_chain(last, &[(contribution, next)]).err().map(|e| e.check);
        let fields = json!({
            "step": "cheat",
            "cheat": "ignore-previous",
            "caught_by": caught.map(view::check_key),
        });
        let beat = view::cheat_ignored(number, caught, language);
        steps.show(beat, record(part, fields), caught == Some(Check::BuildsOnPrevious));
        for (power, expected) in REPLACED_POWERS {
            if !steps.next(M::WorkingCheat.text(language)) {
                return Ok(());
            }
            let turn = cheat::replace_power(last, power, &mut OsRng);
            let (next, contribution) = turn.ok_or_else(|| format!("no power {power}"))?;
            let caught = verify_chain(last, &[(contribution, next)]).err().map(|e| e.check);
            let fields = json!({
                "step": "cheat",
                "cheat": "replace-power",
                "power": power,
                "caught_by": caught.map(view::check_key),
            });
            let beat = view::cheat_replaced(number, power, caught, language);
            steps.show(beat, record(part, fields), caught == Some(expected));
        }
        Ok(())
    }

    fn collude(&self, steps: &Steps<'_>) -> Result<(), String> {
        let (language, part) = (self.language, self.part);
        let mut srs = Srs::initial();
        let mut secrets: Vec<KeptSecret> = Vec::new();
        for number in 1..=self.participants {
            if !steps.next(self.working(M::WorkingTurn, number)) {
                return Ok(());
            }
            let (next, _, secret) = Participant::contribute_keeping_secret(&srs, &mut OsRng);
            let fingerprint = digest(&next);
            let fields = json!({
                "step": "turn",
                "participant": number,
                "kept_secret": true,
                "fingerprint": hex(&fingerprint),
            });
            steps.show(
                view::turn(number, &fingerprint, true, language),
                record(part, fields),
                true,
            );
            secrets.push(secret);
            srs = next;
        }
        self.forge_opening(steps, &srs, &secrets)
    }

    fn forge_opening(
        &self,
        steps: &Steps<'_>,
        srs: &Srs,
        secrets: &[KeptSecret],
    ) -> Result<(), String> {
        let (language, part) = (self.language, self.part);
        let coefficients: Vec<Scalar> = COEFFICIENTS.map(Scalar::from).to_vec();
        let commitment = commit(srs, &coefficients).map_err(debug)?;
        let z = Scalar::from(POINT);
        if !steps.next(M::WorkingOpen.text(language)) {
            return Ok(());
        }
        let (value, proof) = open(srs, &coefficients, z).map_err(debug)?;
        let honest = kzg_verify(srs, &commitment, z, value, &proof);
        let fields = record(part, json!({ "step": "honest-opening", "accepted": honest }));
        steps.show(view::honest_opening(honest, language), fields, honest);
        let claimed = value + Scalar::from(LIE);
        let groups =
            [("all-secrets", secrets), ("one-missing", secrets.get(1..).unwrap_or_default())];
        for (step, kept) in groups {
            if !steps.next(M::WorkingForgeOpening.text(language)) {
                return Ok(());
            }
            let forged = Collusion::from_secrets(kept).forge_open(&commitment, z, claimed);
            let accepted =
                forged.is_some_and(|forged| kzg_verify(srs, &commitment, z, claimed, &forged));
            let fields = json!({ "step": step, "secrets": kept.len(), "accepted": accepted });
            let beat = view::forged_opening(kept.len(), secrets.len(), accepted, language);
            let all = kept.len() == secrets.len();
            steps.show(beat, record(part, fields), accepted == all);
        }
        Ok(())
    }
}
