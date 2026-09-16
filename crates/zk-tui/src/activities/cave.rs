//! `/cave`: Ali Baba's cave filmed six ways, scene by scene, with the real engine's coin and wall.

use serde_json::json;
use zk_cave::court::{Averages, court};
use zk_cave::{
    Demonstration, Floor, OsRng, Prover, Scene, Setting, Tape, apartment_building, demonstration,
    jealous_edit, prior_agreement,
};
use zk_core::{Control, ExampleId};
use zk_i18n::Language;

use super::{CAVE_BEAT, Pace, fresh_seed, pause};
use crate::activity::{Activity, CellRef, Outbox};
use crate::doc::{Entry, Kind};
use crate::phrases::cave::Msg as M;
use crate::phrases::fill;
use crate::views::cave as view;

/// Scenes when none are asked for: forty, for the forty thieves.
pub const DEFAULT_SCENES: u32 = zk_cave::SCENES;
/// Most scenes one filming takes, so a mistyped number cannot keep the screen busy for hours.
pub const MAX_SCENES: u32 = 1_000;
/// Tapes of each kind the court averages its counts over.
const COURT_TAPES: u32 = 100;

/// A way of filming the cave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CaveMode {
    /// Mick Ali, who knows the words.
    Demonstration,
    /// A double without the words, caught at the first scene called on the other side.
    Impostor,
    /// The jealous reporter films the double and keeps only the good scenes.
    JealousEdit,
    /// Mick's tape and the edited tape counted side by side.
    Court,
    /// The reporter and the double agree every call before filming.
    PriorAgreement,
    /// One cave per floor, every floor tested in the same round.
    Apartment,
}

impl CaveMode {
    /// Every mode, in the order the story tells them.
    pub const ALL: [CaveMode; 6] = [
        Self::Demonstration,
        Self::Impostor,
        Self::JealousEdit,
        Self::Court,
        Self::PriorAgreement,
        Self::Apartment,
    ];

    /// The word typed after `/cave`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Demonstration => "demonstration",
            Self::Impostor => "impostor",
            Self::JealousEdit => "jealous-edit",
            Self::Court => "court",
            Self::PriorAgreement => "prior-agreement",
            Self::Apartment => "apartment",
        }
    }

    /// The mode a typed word names.
    pub fn from_key(key: &str) -> Option<CaveMode> {
        Self::ALL.into_iter().find(|mode| mode.key() == key)
    }
}

/// `/cave [mode] [example] [--scenes N]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaveRun {
    /// How it is filmed.
    pub mode: CaveMode,
    /// Whose claim the wall is locked to.
    pub example: ExampleId,
    /// Scenes to film; floors in the apartment building.
    pub scenes: u32,
    /// The language of every cell.
    pub language: Language,
    /// Whether scenes wait for the reader.
    pub pace: Pace,
}

impl Activity for CaveRun {
    fn subject(&self) -> String {
        zk_cave::META.name.to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        let language = self.language;
        let setting = fresh_seed().and_then(|seed| {
            if self.mode == CaveMode::Apartment {
                Setting::floors(self.example, self.scenes, &seed).map_err(|e| e.to_string())
            } else {
                Setting::for_example(self.example, &seed)
                    .map(|s| vec![s])
                    .map_err(|e| e.to_string())
            }
        });
        let settings = match setting {
            Ok(settings) => settings,
            Err(why) => return failed(outbox, &why, language),
        };
        let cell = outbox.open(Entry::new(Kind::Story, view::opening(&self)));
        let film = Film { run: *self, outbox, control, cell };
        let outcome = match (self.mode, settings.first()) {
            (CaveMode::Apartment, _) => film.apartment(settings),
            (_, None) => Ok(()),
            (CaveMode::Demonstration, Some(s)) => {
                film.scenes(s, &Prover::mick_ali(s.words.clone()))
            }
            (CaveMode::Impostor, Some(s)) => film.scenes(s, &Prover::double(None)),
            (CaveMode::PriorAgreement, Some(s)) => film.agreement(s),
            (CaveMode::JealousEdit, Some(s)) => film.edit(s),
            (CaveMode::Court, Some(s)) => film.court(s),
        };
        if let Err(why) = outcome {
            failed(outbox, &why, language);
        }
    }
}

fn failed(outbox: &Outbox, why: &str, language: Language) {
    let text = fill(M::Failed.text(language), &[("why", why)]);
    outbox.open(Entry::text(Kind::Error, text));
}

/// One filming in progress: where its beats go and whether to keep going.
struct Film<'a> {
    run: CaveRun,
    outbox: &'a Outbox,
    control: &'a Control,
    cell: CellRef,
}

impl Film<'_> {
    /// Waits a beat and says which step is next; `false` after appending where it stopped.
    fn beat(&self, working: String, done: usize, total: usize) -> bool {
        if pause(self.control, self.run.pace, CAVE_BEAT) {
            self.outbox.status(working);
            return true;
        }
        self.outbox.append(self.cell, view::stopped(done, total, self.run.language));
        false
    }

    fn record(&self, value: serde_json::Value) {
        let mut value = value;
        value["activity"] = json!("cave");
        value["mode"] = json!(self.run.mode.key());
        self.outbox.record(value);
    }

    /// Streams scenes already filmed; `false` when a stop cut them short.
    fn stream(&self, scenes: &[Scene], total: usize) -> bool {
        let language = self.run.language;
        for (index, scene) in scenes.iter().enumerate() {
            let working = fill(
                M::WorkingScene.text(language),
                &[("scene", &(index + 1).to_string()), ("total", &total.to_string())],
            );
            if !self.beat(working, index, scenes.len()) {
                return false;
            }
            self.outbox.append(self.cell, view::scene_beat(M::SceneLabel, scene, None, language));
            self.record(view::scene_record(scene));
        }
        true
    }

    /// The demonstration and the impostor: the same steps, a different person in the cave.
    fn scenes(&self, setting: &Setting, prover: &Prover) -> Result<(), String> {
        let played = demonstration(&setting.wall, prover, self.run.scenes, &mut OsRng)
            .map_err(|e| e.to_string())?;
        if !self.stream(&played.scenes, self.run.scenes as usize) {
            return Ok(());
        }
        self.finish(&played);
        Ok(())
    }

    fn finish(&self, played: &Demonstration) {
        let language = self.run.language;
        self.outbox.append(self.cell, view::review(&played.scenes, language));
        self.outbox.append(self.cell, view::verdict(self.run.mode, played, language));
        self.record(json!({
            "example": self.run.example.id(),
            "planned": played.planned,
            "played": played.scenes.len(),
            "caught_at": played.caught_at,
            "convinced": played.convinced(),
            "survival_probability": zk_cave::odds::survival_probability(played.planned),
        }));
    }

    fn agreement(&self, setting: &Setting) -> Result<(), String> {
        let double = Prover::double(None);
        let played = prior_agreement(&setting.wall, &double, self.run.scenes, &mut OsRng)
            .map_err(|e| e.to_string())?;
        let calls: Vec<_> = played.scenes.iter().map(|scene| scene.call).collect();
        self.outbox.append(self.cell, view::agreed(&calls, self.run.language));
        if !self.stream(&played.scenes, self.run.scenes as usize) {
            return Ok(());
        }
        self.finish(&played);
        Ok(())
    }

    fn edit(&self, setting: &Setting) -> Result<(), String> {
        let language = self.run.language;
        let double = Prover::double(None);
        let edit = jealous_edit(&setting.wall, &double, self.run.scenes, &mut OsRng)
            .map_err(|e| e.to_string())?;
        let mut kept = 0u32;
        for (index, take) in edit.takes.iter().enumerate() {
            let working =
                fill(M::WorkingTake.text(language), &[("take", &(index + 1).to_string())]);
            if !self.beat(working, index, edit.takes.len()) {
                return Ok(());
            }
            let kept_as = take.succeeded().then(|| {
                kept += 1;
                kept
            });
            let beat = view::scene_beat(M::TakeLabel, take, Some(kept_as), language);
            self.outbox.append(self.cell, beat);
            self.record(
                json!({ "take": index + 1, "kept_as": kept_as, "scene": view::scene_record(take) }),
            );
        }
        self.outbox.append(self.cell, view::edit_summary(&edit, self.run.scenes, language));
        self.record(json!({
            "example": self.run.example.id(),
            "takes": edit.takes.len(),
            "kept": edit.kept.len(),
            "cut": edit.discarded(),
        }));
        Ok(())
    }

    fn court(&self, setting: &Setting) -> Result<(), String> {
        let language = self.run.language;
        let (mick, double) = (Prover::mick_ali(setting.words.clone()), Prover::double(None));
        let scenes = self.run.scenes;
        let genuine = demonstration(&setting.wall, &mick, scenes, &mut OsRng)
            .map_err(|e| e.to_string())?
            .tape();
        let edited = jealous_edit(&setting.wall, &double, scenes, &mut OsRng)
            .map_err(|e| e.to_string())?
            .tape();
        for (index, (left, right)) in genuine.scenes.iter().zip(&edited.scenes).enumerate() {
            let working = fill(
                M::WorkingScene.text(language),
                &[("scene", &(index + 1).to_string()), ("total", &scenes.to_string())],
            );
            if !self.beat(working, index, genuine.scenes.len()) {
                return Ok(());
            }
            self.outbox.append(self.cell, view::court_beat(index + 1, left, right, language));
        }
        let hearing = court(&genuine, &edited);
        self.outbox.append(self.cell, view::hearing(&hearing, language));
        let working = fill(M::WorkingCourt.text(language), &[("tapes", &COURT_TAPES.to_string())]);
        self.outbox.status(working);
        let Some((mine, theirs)) = self.averages(setting, &mick, &double)? else {
            self.outbox.append(self.cell, view::stopped_counting(language));
            return Ok(());
        };
        self.outbox.append(self.cell, view::averages(COURT_TAPES, &mine, &theirs, language));
        self.record(view::court_record(&hearing, &mine, &theirs));
        Ok(())
    }

    /// Counts over many tapes of each kind; `None` when a stop was asked for in between.
    fn averages(
        &self,
        setting: &Setting,
        mick: &Prover,
        double: &Prover,
    ) -> Result<Option<(Averages, Averages)>, String> {
        let (mut genuine, mut edited) = (Vec::<Tape>::new(), Vec::<Tape>::new());
        for _ in 0..COURT_TAPES {
            if self.control.is_cancelled() {
                return Ok(None);
            }
            let scenes = self.run.scenes;
            let played = demonstration(&setting.wall, mick, scenes, &mut OsRng);
            genuine.push(played.map_err(|e| e.to_string())?.tape());
            let edit = jealous_edit(&setting.wall, double, scenes, &mut OsRng);
            edited.push(edit.map_err(|e| e.to_string())?.tape());
        }
        Ok(Some((Averages::of(&genuine), Averages::of(&edited))))
    }

    fn apartment(&self, settings: Vec<Setting>) -> Result<(), String> {
        let language = self.run.language;
        let floors: Vec<Floor> = settings
            .into_iter()
            .map(|setting| Floor { wall: setting.wall, prover: Prover::mick_ali(setting.words) })
            .collect();
        let round = apartment_building(&floors, &mut OsRng).map_err(|e| e.to_string())?;
        let total = round.floors.len();
        for (index, scene) in round.floors.iter().enumerate() {
            let working = fill(
                M::WorkingFloor.text(language),
                &[("floor", &(index + 1).to_string()), ("total", &total.to_string())],
            );
            if !self.beat(working, index, total) {
                return Ok(());
            }
            self.outbox.append(self.cell, view::scene_beat(M::FloorLabel, scene, None, language));
            self.record(view::scene_record(scene));
        }
        self.outbox.append(self.cell, view::review(&round.floors, language));
        self.outbox
            .append(self.cell, view::building_verdict(&round.failed_floors(), total, language));
        self.record(json!({
            "example": self.run.example.id(),
            "floors": total,
            "failed_floors": round.failed_floors(),
            "convinced": round.convinced(),
        }));
        Ok(())
    }
}
