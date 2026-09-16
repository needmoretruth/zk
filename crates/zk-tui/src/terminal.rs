//! The real terminal: raw mode, the alternate screen, mouse capture, and putting all of it back.

use std::io::{self, Stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crossterm::cursor::Show;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{App, JOB_THREAD, Settings};
use crate::museum::Museum;

/// Redraw interval while something animates (the spinner's frame length).
const FRAME: Duration = Duration::from_millis(80);
/// Redraw interval while nothing moves.
const IDLE: Duration = Duration::from_millis(250);

/// Whether keyboard enhancement was pushed, so restoring pops it exactly once.
static ENHANCED: AtomicBool = AtomicBool::new(false);

/// Runs the TUI until the reader quits, then restores the terminal.
pub fn run(museum: Museum, settings: Settings) -> io::Result<()> {
    let session = Session::start()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(museum, settings);
    let result = event_loop(&mut terminal, &mut app);
    app.stop();
    drop(session);
    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        if app.should_quit() {
            return Ok(());
        }
        let wait = if app.animating() { FRAME } else { IDLE };
        if event::poll(wait)? {
            app.handle_event(event::read()?);
            while event::poll(Duration::ZERO)? {
                app.handle_event(event::read()?);
            }
        }
        app.pump();
        if app.take_clear_request() {
            terminal.clear()?;
        }
    }
}

/// The terminal modes the TUI needs, undone on drop and on a panic in the screen thread.
struct Session;

impl Session {
    fn start() -> io::Result<Session> {
        terminal::enable_raw_mode()?;
        let session = Session;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
        // Terminals that speak the kitty keyboard protocol can then tell Shift+Enter from Enter.
        if matches!(terminal::supports_keyboard_enhancement(), Ok(true)) {
            execute!(
                out,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
            ENHANCED.store(true, Ordering::SeqCst);
        }
        install_panic_hook();
        Ok(session)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let mut out = io::stdout();
    if ENHANCED.swap(false, Ordering::SeqCst) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(out, DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen, Show);
    let _ = terminal::disable_raw_mode();
}

/// A panic in a worker thread is caught and shown as a cell, so its message must not scribble over
/// the screen; any other panic restores the terminal first so the message is readable.
fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if thread::current().name() == Some(JOB_THREAD) {
            return;
        }
        restore();
        previous(info);
    }));
}
