//! Carrying out a command the reader entered.

use zk_core::{ExampleId, RunOptions};

use super::{App, Popup};
use crate::activities::cave::CaveRun;
use crate::activities::ceremony::CeremonyRun;
use crate::activities::forge::ForgeRun;
use crate::activities::pool::PoolRun;
use crate::activities::trio::TrioRun;
use crate::activity::Activity;
use crate::commands::{self, Command, Target};
use crate::doc::{Doc, Entry, Kind, Tone, span};
use crate::phrases::fill;
use crate::phrases::ui::Msg;
use crate::runs::{RunAll, RunOne};
use crate::views;

impl App {
    /// Runs what is in the composer: the line becomes a command cell, then the command's cells follow.
    pub(super) fn submit(&mut self) {
        let line = self.composer.take();
        self.popup = Popup::default();
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.transcript.follow();
        let mut doc = Doc::new();
        doc.line(vec![span(line, Tone::BODY.bold())]);
        self.transcript.push(Entry::new(Kind::Command, doc));
        match commands::parse(line) {
            Ok(command) => self.execute(command),
            Err(error) => {
                let text = commands::error_text(error, self.language);
                self.transcript.push(Entry::text(Kind::Error, text));
            }
        }
    }

    fn execute(&mut self, command: Command) {
        let language = self.language;
        match command {
            Command::Help => self.transcript.push(views::help(language)),
            Command::List(shelf) => {
                self.transcript.push(views::list(&self.museum, shelf, language))
            }
            Command::About(id) => {
                let (entry, page) = views::about(&self.museum, &id, language);
                self.transcript.push(entry);
                if let Some(page) = page {
                    self.open_page(page);
                }
            }
            Command::Examples => self.transcript.push(views::examples(language)),
            Command::Run { target, example } => self.run(target, example),
            Command::Cave { mode, example, scenes } => {
                self.start(Box::new(CaveRun { mode, example, scenes, language, pace: self.pace }));
            }
            Command::Trio { mode, example, rounds } => {
                self.start(Box::new(TrioRun { mode, example, rounds, language, pace: self.pace }));
            }
            Command::Pool(action) => {
                let (data_dir, cache) = (self.data_dir.clone(), self.pool.clone());
                self.start(Box::new(PoolRun { action, data_dir, cache, language }));
            }
            Command::Ceremony { part, participants } => {
                let pace = self.pace;
                self.start(Box::new(CeremonyRun { part, participants, language, pace }));
            }
            Command::Forge(target) => {
                self.start(Box::new(ForgeRun { target, language, pace: self.pace }));
            }
            Command::Lang(chosen) => {
                self.language = chosen;
                let text = fill(Msg::LanguageSet.text(chosen), &[("language", chosen.endonym())]);
                self.transcript.push(Entry::text(Kind::Result, text));
            }
            Command::Clear => self.transcript.clear(),
            Command::Quit => self.quit = true,
        }
    }

    fn run(&mut self, target: Target, example: ExampleId) {
        let language = self.language;
        let options = RunOptions::default();
        let activity: Box<dyn Activity> = match target {
            Target::All if self.museum.systems.is_empty() => {
                self.transcript.push(views::no_systems(language));
                return;
            }
            Target::All => {
                Box::new(RunAll { systems: self.museum.systems, example, language, options })
            }
            Target::System(id) => match self.museum.system(&id) {
                Some(system) => Box::new(RunOne { system, example, language, options }),
                None => {
                    self.transcript.push(views::unknown_system(&id, language));
                    return;
                }
            },
        };
        self.start(activity);
    }
}
