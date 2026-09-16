//! Stand-in systems and helpers for driving the TUI without a terminal.
//!
//! The stand-ins live only here, never in the shipped registry.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};
use zk_core::{
    CircuitShape, Control, ExampleId, FieldBytes, Instance, InstanceKind, Prepared, ProofSystem,
    Proven, ShapeForm, SystemError, Verdict,
};
use zk_i18n::Language;
use zk_tui::{App, Look, Museum, Settings};

const PUBLIC: [u8; 4] = [9, 0, 0, 0];
const SECRET: [u8; 5] = [2, 0, 0, 0, 0];

/// A system that proves instantly; `slow` waits in setup until it is cancelled.
pub struct StandIn {
    pub meta: &'static SystemMeta,
    pub slow: bool,
}

pub static TEACHING: SystemMeta = SystemMeta {
    id: "standin",
    name: "Stand-in",
    shelf: Shelf::Others,
    year: 2026,
    authors: &["Nobody"],
    paper: None,
    trusted_setup: TrustedSetup::None,
    zero_knowledge: ZeroKnowledge::No,
    assumptions: &[Assumption::Hash],
    proof_size: ProofSize::Linear,
    recursion: Recursion::None,
    mode: Mode::NonInteractive,
    field: "toy field",
    implementation: Implementation::Teaching { built_on: &["nothing"] },
    status: Status::Experimental,
    status_as_of: "2026-09",
    status_sources: &[],
    deployments: &[],
};

pub static UPSTREAM: SystemMeta = SystemMeta {
    id: "standin-b",
    name: "Stand-in B",
    shelf: Shelf::Homemade,
    zero_knowledge: ZeroKnowledge::Yes,
    implementation: Implementation::Homemade,
    ..TEACHING
};

pub static SLOW: SystemMeta =
    SystemMeta { id: "slow", name: "Slow", shelf: Shelf::Zcash, ..UPSTREAM };

pub static FAST: StandIn = StandIn { meta: &TEACHING, slow: false };
pub static OTHER: StandIn = StandIn { meta: &UPSTREAM, slow: false };
pub static STUCK: StandIn = StandIn { meta: &SLOW, slow: true };

pub static TWO: [&dyn ProofSystem; 2] = [&FAST, &OTHER];
pub static WITH_SLOW: [&dyn ProofSystem; 2] = [&STUCK, &FAST];

pub const PAGE: &str = "# Stand-in\n\nA **toy** system with *no* security.\n\n## History\n\n- first\n- second\n\n| Year | Event |\n|---:|---|\n| 2026 | written |\n\n```\nfn prove() {}\n```\n\n[source](https://example.org/standin)\n";

fn page(id: &str, _language: Language) -> Option<&'static str> {
    (id == "standin").then_some(PAGE)
}

pub fn museum(systems: &'static [&'static dyn ProofSystem]) -> Museum {
    Museum { systems, page }
}

impl ProofSystem for StandIn {
    fn meta(&self) -> &'static SystemMeta {
        self.meta
    }

    fn prepare(
        &self,
        _example: ExampleId,
        control: &Control,
    ) -> Result<Box<dyn Prepared>, SystemError> {
        if self.slow {
            loop {
                control.checkpoint()?;
                thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(Box::new(Toy))
    }
}

struct Toy;

fn proof() -> Vec<u8> {
    let mut proof = vec![7u8; 30];
    proof.extend(SECRET);
    proof.extend([7u8; 29]);
    proof
}

impl Prepared for Toy {
    fn shape(&self) -> CircuitShape {
        CircuitShape {
            form: ShapeForm::R1cs,
            counts: vec![("constraints".into(), 1234), ("variables".into(), 1240)],
        }
    }

    fn setup_bytes(&self) -> Option<u64> {
        Some(4096)
    }

    fn prove(&mut self, instance: &Instance, _control: &Control) -> Result<Proven, SystemError> {
        match instance.kind {
            InstanceKind::Honest => Ok(Proven {
                proof: proof(),
                public: vec![PUBLIC.to_vec()],
                secrets: vec![SECRET.to_vec()],
            }),
            InstanceKind::Dishonest => {
                Err(SystemError::Unsatisfied("the answer is not 1 + 1".into()))
            }
        }
    }

    fn verify(
        &mut self,
        public: &[FieldBytes],
        candidate: &[u8],
        _control: &Control,
    ) -> Result<Verdict, SystemError> {
        let honest = public == [PUBLIC.to_vec()] && candidate == proof().as_slice();
        Ok(if honest { Verdict::Accepted } else { Verdict::Rejected })
    }

    fn bump_public(&self, value: &FieldBytes) -> Result<FieldBytes, SystemError> {
        let mut bumped = value.clone();
        bumped[0] = bumped[0].wrapping_add(1);
        Ok(bumped)
    }
}

pub fn app(museum: Museum) -> App {
    App::new(museum, Settings { language: Language::ENGLISH, look: Look::default() })
}

pub fn draw(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    terminal.draw(|frame| app.draw(frame)).expect("draw");
    terminal.backend().buffer().clone()
}

/// Every row of the buffer as text.
pub fn rows(buffer: &Buffer) -> Vec<String> {
    let area = buffer.area;
    (0..area.height)
        .map(|y| {
            let mut row = String::new();
            let mut x = 0;
            while x < area.width {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                // A wide character covers the next cell too; that cell is not text of its own.
                x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
            }
            row
        })
        .collect()
}

pub fn screen(buffer: &Buffer) -> String {
    rows(buffer).join("\n")
}

/// The position of the first cell of `needle`, searching row by row.
pub fn find(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
    rows(buffer).iter().enumerate().find_map(|(y, row)| {
        let byte = row.find(needle)?;
        let x = row[..byte].chars().count();
        Some((x as u16, y as u16))
    })
}

pub fn press(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}

pub fn ctrl(app: &mut App, character: char) {
    app.key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL));
}

pub fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        press(app, KeyCode::Char(character));
    }
}

pub fn enter(app: &mut App, line: &str) {
    type_text(app, line);
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

pub fn wait_idle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.is_busy() {
        assert!(Instant::now() < deadline, "the activity did not finish");
        thread::sleep(Duration::from_millis(2));
        app.pump();
    }
    app.pump();
}

/// No cell paints a background, and every foreground is one of the named colours the design allows.
pub fn assert_theme_respected(buffer: &Buffer) {
    let allowed = [
        Color::Reset,
        Color::Cyan,
        Color::Green,
        Color::Red,
        Color::Yellow,
        Color::Magenta,
        Color::DarkGray,
    ];
    for cell in &buffer.content {
        assert_eq!(cell.bg, Color::Reset, "a background was painted under {:?}", cell.symbol());
        assert!(allowed.contains(&cell.fg), "{:?} uses {:?}", cell.symbol(), cell.fg);
    }
}

/// Rows whose cells carry the reversed attribute.
pub fn reversed_rows(buffer: &Buffer) -> Vec<u16> {
    let area = buffer.area;
    (0..area.height)
        .filter(|y| (0..area.width).any(|x| buffer[(x, *y)].modifier.contains(Modifier::REVERSED)))
        .collect()
}
