//! `/trio`: Trio's live conversation round by round, with an honest prover, a cheat or the
//! simulator across from the verifier.

use serde_json::json;
use zk_core::{Control, ExampleId, InstanceKind, Verdict};
use zk_i18n::Language;
use zk_trio::{
    Card, Cheat, CheatingProver, Coins, Friend, HonestProver, RoundProver, Session, Simulator,
    Statement, TrioError,
};

use super::{Pace, TRIO_BEAT, fresh_seed, pause};
use crate::activity::{Activity, CellRef, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::fill;
use crate::phrases::trio::Msg as M;
use crate::views::trio as view;

/// Rounds when none are asked for: the live conversation's 55, (3/5)^55 < 2^-40.
pub const DEFAULT_ROUNDS: u32 = zk_trio::INTERACTIVE_ROUNDS;
/// Most rounds one conversation plays.
pub const MAX_ROUNDS: u32 = 1_000;

/// A way to lie to the verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrioCheat {
    /// The dealer rigs one multiplication card.
    BadCard,
    /// Friend 2 broadcasts shares that make every assertion sum to zero.
    BadComputation,
    /// The forgery that broke the first draft: rewrite the hidden friend's shares after the card.
    RewriteHidden,
}

impl TrioCheat {
    /// Every cheat.
    pub const ALL: [TrioCheat; 3] = [Self::BadCard, Self::BadComputation, Self::RewriteHidden];

    /// The word typed after `/trio cheat`.
    pub fn key(self) -> &'static str {
        match self {
            Self::BadCard => "bad-card",
            Self::BadComputation => "bad-computation",
            Self::RewriteHidden => "rewrite-hidden",
        }
    }

    /// The cheat a typed word names.
    pub fn from_key(key: &str) -> Option<TrioCheat> {
        Self::ALL.into_iter().find(|cheat| cheat.key() == key)
    }

    /// The engine's cheat; the lying friend of a bad computation is friend 2.
    pub fn engine(self) -> Cheat {
        match self {
            Self::BadCard => Cheat::BadCard,
            Self::BadComputation => Cheat::BadComputation(Friend::P2),
            Self::RewriteHidden => Cheat::RewriteHiddenShares,
        }
    }
}

/// Who sits across from the verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrioMode {
    /// The prover with the true witness.
    Play,
    /// A prover telling one lie.
    Cheat(TrioCheat),
    /// A prover with no witness who knows every card in advance.
    Simulate,
}

impl TrioMode {
    /// The word typed after `/trio`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Cheat(_) => "cheat",
            Self::Simulate => "simulate",
        }
    }
}

/// `/trio [play|cheat <kind>|simulate] [example] [--rounds N]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrioRun {
    /// Who proves.
    pub mode: TrioMode,
    /// The statement.
    pub example: ExampleId,
    /// Rounds the verifier plans.
    pub rounds: u32,
    /// The language of every cell.
    pub language: Language,
    /// Whether rounds wait for the reader.
    pub pace: Pace,
}

impl Activity for TrioRun {
    fn subject(&self) -> String {
        zk_trio::META.name.to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        if let Err(why) = self.converse(outbox, control) {
            let text = fill(M::Failed.text(self.language), &[("why", &why)]);
            outbox.open(Entry::text(Kind::Error, text));
        }
    }
}

impl TrioRun {
    fn converse(&self, outbox: &Outbox, control: &Control) -> Result<(), String> {
        let text = |error: TrioError| error.to_string();
        let statement = Statement::new(self.example).map_err(text)?;
        let seed = fresh_seed()?;
        let honest = self.example.instance::<zk_trio::Fp>(InstanceKind::Honest, &seed);
        let cell = outbox.open(Entry::new(Kind::Story, view::opening(self)));
        let talk = Talk { run: *self, outbox, control, cell };
        match self.mode {
            TrioMode::Play => {
                let claim = statement.claim(&honest).map_err(text)?;
                let public = claim.public.clone();
                let mut prover = HonestProver::new(&statement, claim, Coins::os()).map_err(text)?;
                talk.rounds(&statement, public, &mut prover, None)
            }
            TrioMode::Cheat(cheat) => {
                let mut prover =
                    CheatingProver::new(&statement, self.example, cheat.engine(), Coins::os())
                        .map_err(text)?;
                let lie = prover.false_claim().clone();
                outbox.append(cell, view::lie(cheat, &lie, self.language));
                talk.rounds(&statement, lie.public, &mut prover, None)
            }
            TrioMode::Simulate => {
                let mut cards = Coins::os();
                let plan = (0..self.rounds).map(|_| cards.card()).collect::<Result<Vec<_>, _>>();
                let plan = plan.map_err(text)?;
                outbox.append(cell, view::plan(&plan, self.language));
                let mut simulator =
                    Simulator::new(&statement, honest.public.clone(), plan.clone(), Coins::os())
                        .map_err(text)?;
                talk.rounds(&statement, honest.public, &mut simulator, Some(&plan))
            }
        }
    }
}

/// A conversation in progress.
struct Talk<'a> {
    run: TrioRun,
    outbox: &'a Outbox,
    control: &'a Control,
    cell: CellRef,
}

impl Talk<'_> {
    /// Plays round after round until the verifier decides or a stop is asked for. With `plan`, each
    /// round's card is the planned one instead of a fresh draw.
    fn rounds(
        &self,
        statement: &Statement,
        public: Vec<zk_trio::Fp>,
        prover: &mut dyn RoundProver,
        plan: Option<&[Card]>,
    ) -> Result<(), String> {
        let language = self.run.language;
        let rounds = self.run.rounds;
        let mut session = Session::new(statement, public, rounds).map_err(|e| e.to_string())?;
        let mut cards = Coins::os();
        let mut last = None;
        while !session.is_over() {
            let played = session.played();
            if !pause(self.control, self.run.pace, TRIO_BEAT) {
                self.outbox.append(self.cell, view::stopped(played, rounds, language));
                return Ok(());
            }
            let working = fill(
                M::WorkingRound.text(language),
                &[("round", &(played + 1).to_string()), ("total", &rounds.to_string())],
            );
            self.outbox.status(working);
            let event = match plan.and_then(|plan| plan.get(played as usize)) {
                Some(card) => session.play_round_with(prover, *card),
                None => session.play_round(prover, &mut cards),
            };
            let event = event.map_err(|e| e.to_string())?;
            self.outbox.append(self.cell, view::round(&event, language));
            self.record(view::round_record(&event));
            last = Some(event);
        }
        let verdict = session.verdict().unwrap_or(Verdict::Rejected);
        self.outbox.append(self.cell, view::verdict(&self.run, &session, last.as_ref(), language));
        self.record(json!({
            "example": self.run.example.id(),
            "verdict": if verdict.accepted() { "accepted" } else { "rejected" },
            "rounds_planned": rounds,
            "rounds_played": session.played(),
            "caught_at": session.caught_at(),
            "escape_probability": zk_trio::escape_probability(rounds),
        }));
        Ok(())
    }

    fn record(&self, mut value: serde_json::Value) {
        value["activity"] = json!("trio");
        value["mode"] = json!(self.run.mode.key());
        if let TrioMode::Cheat(cheat) = self.run.mode {
            value["cheat"] = json!(cheat.key());
        }
        self.outbox.record(value);
    }
}
