//! Carrying out a command the reader entered.

use zk_core::{ExampleId, RunOptions};

use super::{App, Popup};
use crate::activity::Activity;
use crate::commands::{self, Command, ParseError, Target};
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
                let entry = self.parse_error(error);
                self.transcript.push(entry);
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

    fn parse_error(&self, error: ParseError) -> Entry {
        let language = self.language;
        let text = match error {
            ParseError::Empty | ParseError::NoSlash => Msg::NoSlash.text(language).to_string(),
            ParseError::Unknown(command) => {
                fill(Msg::UnknownCommand.text(language), &[("command", &command)])
            }
            ParseError::Usage(usage) => fill(Msg::Usage.text(language), &[("usage", usage)]),
            ParseError::Shelf(shelf) => {
                let keys: Vec<&str> =
                    zk_core::catalog::Shelf::ALL.iter().map(|s| s.key()).collect();
                fill(
                    Msg::UnknownShelf.text(language),
                    &[("shelf", &shelf), ("shelves", &keys.join(", "))],
                )
            }
            ParseError::Example(example) => {
                fill(Msg::UnknownExample.text(language), &[("example", &example)])
            }
            ParseError::Language(code) => {
                let codes: Vec<&str> = zk_i18n::Language::ALL.iter().map(|l| l.code()).collect();
                fill(
                    Msg::UnknownLanguage.text(language),
                    &[("language", &code), ("languages", &codes.join(", "))],
                )
            }
        };
        Entry::text(Kind::Error, text)
    }
}
