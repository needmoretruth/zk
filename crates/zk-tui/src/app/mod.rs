//! The TUI's state. Keys change it (`keys`), commands act on it (`submit`), activities report into
//! it (`jobs`) and `draw` paints it; nothing here touches the real terminal, so tests drive it with
//! ratatui's test backend.

mod draw;
mod jobs;
mod keys;
mod submit;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use zk_i18n::Language;

use crate::activities::Pace;
use crate::activities::pool::PoolCache;
use crate::activity::JobEvent;
use crate::complete::{self, Candidate};
use crate::composer::Composer;
use crate::look::Look;
use crate::museum::Museum;
use crate::pager::Pager;
use crate::transcript::Transcript;
use crate::views::{self, Page};

pub(crate) use jobs::{JOB_THREAD, panic_message};

/// Smallest terminal every feature works in.
pub(crate) const MIN_WIDTH: u16 = 60;
/// Smallest terminal every feature works in.
pub(crate) const MIN_HEIGHT: u16 = 16;
/// How long the second ctrl+c may take.
const QUIT_WINDOW: Duration = Duration::from_secs(2);
/// How long a footer message stays.
const FLASH_FOR: Duration = Duration::from_secs(2);

/// Start-up choices from the command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The language the screen starts in; `/lang` changes it.
    pub language: Language,
    /// Colours and glyphs.
    pub look: Look,
    /// `--data-dir`: where the Toy Shielded Pool is kept; `None` for the platform's data directory.
    pub data_dir: Option<PathBuf>,
    /// Whether story beats wait for the reader; tests set [`Pace::Instant`].
    pub pace: Pace,
}

#[derive(Debug, Default)]
struct Popup {
    selected: usize,
    /// The text the reader closed the popup on; it reopens when the text changes.
    dismissed: Option<String>,
}

/// Everything the TUI shows and remembers.
pub struct App {
    museum: Museum,
    language: Language,
    look: Look,
    data_dir: Option<PathBuf>,
    pace: Pace,
    pool: PoolCache,
    transcript: Transcript,
    composer: Composer,
    popup: Popup,
    pager: Option<Pager>,
    last_page: Option<Page>,
    shortcuts: bool,
    job: Option<jobs::Job>,
    sender: Sender<JobEvent>,
    receiver: Receiver<JobEvent>,
    next_job: u64,
    quit_armed: Option<Instant>,
    flash: Option<(String, Instant)>,
    quit: bool,
    clear_requested: bool,
}

impl App {
    /// A fresh screen with the welcome box at the top.
    pub fn new(museum: Museum, settings: Settings) -> App {
        let (sender, receiver) = mpsc::channel();
        let mut transcript = Transcript::default();
        transcript.push(views::welcome(&museum, settings.language));
        App {
            museum,
            language: settings.language,
            look: settings.look,
            data_dir: settings.data_dir,
            pace: settings.pace,
            pool: PoolCache::default(),
            transcript,
            composer: Composer::default(),
            popup: Popup::default(),
            pager: None,
            last_page: None,
            shortcuts: false,
            job: None,
            sender,
            receiver,
            next_job: 0,
            quit_armed: None,
            flash: None,
            quit: false,
            clear_requested: false,
        }
    }

    /// Whether the reader asked to leave.
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Whether something on screen moves, so the loop should redraw at spinner speed.
    pub fn animating(&self) -> bool {
        self.job.is_some()
            || self.transcript.animating()
            || self.quit_armed.is_some()
            || self.flash.is_some()
    }

    /// Whether ctrl+l asked for a full redraw since the last call.
    pub fn take_clear_request(&mut self) -> bool {
        std::mem::take(&mut self.clear_requested)
    }

    /// The language the screen speaks now.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Whether the full-screen page view is open.
    pub fn pager_open(&self) -> bool {
        self.pager.is_some()
    }

    /// What the composer holds.
    pub fn input(&self) -> &str {
        self.composer.text()
    }

    /// The popup rows for what is typed now; empty when the popup is closed.
    fn candidates(&self) -> Vec<Candidate> {
        if self.composer.recalling()
            || self.popup.dismissed.as_deref() == Some(self.composer.text())
        {
            return Vec::new();
        }
        complete::candidates(self.composer.text(), &self.museum, self.language)
    }

    fn quit_armed(&self) -> bool {
        self.quit_armed.is_some_and(|armed| armed.elapsed() < QUIT_WINDOW)
    }

    fn flash(&mut self, text: impl Into<String>) {
        self.flash = Some((text.into(), Instant::now()));
    }

    fn open_page(&mut self, page: Page) {
        self.pager = Some(Pager::new(&page.title, page.markdown));
        self.last_page = Some(page);
    }

    /// Forgets expired footer messages so the loop can slow down again.
    fn expire(&mut self) {
        if self.quit_armed.is_some_and(|armed| armed.elapsed() >= QUIT_WINDOW) {
            self.quit_armed = None;
        }
        if self.flash.as_ref().is_some_and(|(_, since)| since.elapsed() >= FLASH_FOR) {
            self.flash = None;
        }
    }
}

impl core::fmt::Debug for App {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("App")
            .field("museum", &self.museum)
            .field("language", &self.language)
            .field("busy", &self.job.is_some())
            .finish_non_exhaustive()
    }
}
