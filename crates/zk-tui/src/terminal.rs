//! The real terminal: raw mode, the alternate screen, mouse capture, and putting all of it back.
//!
//! `nmtzk` points descriptor 1 at `/dev/null` so that proof-system crates cannot draw over the
//! screen, and crossterm writes its terminal queries to descriptor 1. Asking the terminal for the
//! cursor position or for keyboard support would then wait two seconds for an answer to a question
//! nobody asked, so this module never asks for the cursor, and asks about the keyboard itself.

use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
use ratatui::layout::Rect;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};

use crate::app::{App, JOB_THREAD, Settings};
use crate::museum::Museum;

/// Redraw interval while something animates (the spinner's frame length).
const FRAME: Duration = Duration::from_millis(80);
/// Redraw interval while nothing moves.
const IDLE: Duration = Duration::from_millis(250);

/// Whether keyboard enhancement was pushed, so restoring pops it exactly once.
static ENHANCED: AtomicBool = AtomicBool::new(false);

/// Signals that end the program from outside: `kill`, a closed terminal window, an interrupt sent by
/// another process (in raw mode ctrl+c is a key, not a signal).
const ENDING: [i32; 3] = [SIGTERM, SIGHUP, SIGINT];

/// The drawing surface. Buffered, because the backend writes one escape sequence per changed cell
/// and flushes once a frame.
type Screen = Terminal<CrosstermBackend<BufWriter<File>>>;

/// Runs the TUI on `screen` until the reader quits, then restores the terminal.
///
/// `screen` is the terminal's output, passed in rather than taken from descriptor 1: `nmtzk` points
/// descriptor 1 at `/dev/null` so that proof-system crates printing on their own cannot draw over
/// the screen, and hands over its private duplicate of standard output instead.
///
/// A signal that ends the program (`ENDING`) is caught: the screen loop stops, the terminal is put
/// back, and then the signal is taken the way it would have been, so the shell sees the program end
/// by it. A second one while the first is handled ends the program at once.
pub fn run(museum: Museum, settings: Settings, screen: File) -> io::Result<()> {
    let signalled = Arc::new(AtomicUsize::new(0));
    let armed = Arc::new(AtomicBool::new(false));
    for signal in ENDING {
        // In this order: the first signal finds the shutdown unarmed, then arms it and says which
        // signal came; the second one ends the program with the usual code for that signal.
        signal_hook::flag::register_conditional_shutdown(signal, 128 + signal, Arc::clone(&armed))?;
        signal_hook::flag::register(signal, Arc::clone(&armed))?;
        signal_hook::flag::register_usize(signal, Arc::clone(&signalled), signal as usize)?;
    }
    let session = Session::start(screen.try_clone()?)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(BufWriter::new(screen)))?;
    let mut app = App::new(museum, settings);
    let result = event_loop(&mut terminal, &mut app, &signalled);
    app.stop();
    drop(session);
    match i32::try_from(signalled.load(Ordering::SeqCst)) {
        Ok(0) => result,
        Ok(signal) => {
            signal_hook::low_level::emulate_default_handler(signal)?;
            result
        }
        Err(_) => result,
    }
}

fn event_loop(terminal: &mut Screen, app: &mut App, signalled: &AtomicUsize) -> io::Result<()> {
    loop {
        if signalled.load(Ordering::SeqCst) != 0 {
            return Ok(());
        }
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
            // `Terminal::clear` asks where the cursor is; resizing to the same size clears the
            // screen and forgets what was drawn without asking the terminal anything.
            let size = terminal.size()?;
            terminal.resize(Rect::new(0, 0, size.width, size.height))?;
        }
    }
}

/// The terminal modes the TUI needs, undone on drop and on a panic in the screen thread.
struct Session {
    screen: File,
}

impl Session {
    fn start(screen: File) -> io::Result<Session> {
        terminal::enable_raw_mode()?;
        let session = Session { screen };
        let mut out = &session.screen;
        execute!(out, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
        // crossterm sends this question to descriptor 1 and reads the answer from the terminal;
        // when descriptor 1 is not the terminal, the question is sent to the screen here instead,
        // so that the answer arrives at once. Keyboard flags, then device attributes, which every
        // terminal answers, so that one without the kitty protocol says so without a wait.
        if !io::stdout().is_terminal() {
            out.write_all(b"\x1b[?u\x1b[c")?;
            out.flush()?;
        }
        // Terminals that speak the kitty keyboard protocol can then tell Shift+Enter from Enter.
        if matches!(terminal::supports_keyboard_enhancement(), Ok(true)) {
            execute!(
                out,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
            ENHANCED.store(true, Ordering::SeqCst);
        }
        install_panic_hook(session.screen.try_clone()?);
        Ok(session)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        restore(&self.screen);
    }
}

fn restore(mut out: &File) {
    if ENHANCED.swap(false, Ordering::SeqCst) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(out, DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen, Show);
    let _ = terminal::disable_raw_mode();
}

/// A panic in a worker thread is caught and shown as a cell, so its message must not scribble over
/// the screen; any other panic restores the terminal first so the message is readable.
fn install_panic_hook(screen: File) {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if thread::current().name() == Some(JOB_THREAD) {
            return;
        }
        restore(&screen);
        previous(info);
    }));
}
