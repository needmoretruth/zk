//! What keys do: history, quitting, stopping a run, completion, and the activity extension point.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use support::*;
use zk_core::Control;
use zk_i18n::Language;
use zk_tui::doc::{Doc, Entry, Kind, plain};
use zk_tui::{Activity, Museum, Outbox};

#[test]
fn up_and_down_walk_command_history() {
    let mut app = app(Museum::empty());
    enter(&mut app, "/help");
    enter(&mut app, "/examples");
    type_text(&mut app, "draft");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.input(), "/examples");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.input(), "/help");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.input(), "/help", "the oldest entry stays");
    press(&mut app, KeyCode::Down);
    assert_eq!(app.input(), "/examples");
    press(&mut app, KeyCode::Down);
    assert_eq!(app.input(), "draft", "past the newest entry the draft comes back");
}

#[test]
fn recalled_commands_do_not_reopen_the_popup_under_the_arrows() {
    let mut app = app(Museum::empty());
    enter(&mut app, "/help");
    enter(&mut app, "/list");
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.input(), "/help");
}

#[test]
fn ctrl_c_clears_the_line_then_needs_a_second_press_to_quit() {
    let mut app = app(Museum::empty());
    type_text(&mut app, "/list");
    ctrl(&mut app, 'c');
    assert_eq!(app.input(), "");
    assert!(!app.should_quit());
    ctrl(&mut app, 'c');
    assert!(!app.should_quit(), "one press on an empty line only warns");
    let buffer = draw(&mut app, 80, 24);
    assert!(rows(&buffer)[23].contains("press ctrl+c again to quit"), "{}", screen(&buffer));
    ctrl(&mut app, 'c');
    assert!(app.should_quit());
}

#[test]
fn esc_stops_a_running_task_and_the_cell_says_where() {
    let mut app = app(museum(&WITH_SLOW));
    enter(&mut app, "/run slow one-plus-one");
    assert!(app.is_busy());
    std::thread::sleep(std::time::Duration::from_millis(20));
    app.pump();
    let buffer = draw(&mut app, 80, 24);
    let working = &rows(&buffer)[19];
    assert!(
        working.contains("Setting up Slow · ") && working.contains("s · esc to stop"),
        "{}",
        screen(&buffer)
    );
    assert_eq!(buffer[(0, 19)].fg, ratatui::style::Color::Cyan, "the spinner is the accent colour");
    press(&mut app, KeyCode::Esc);
    wait_idle(&mut app);
    let shown = screen(&draw(&mut app, 80, 24));
    assert!(shown.contains("⚠ Stopped Slow before setup finished."), "{shown}");
    assert!(!shown.contains("esc to stop"));
}

#[test]
fn a_second_run_waits_for_the_first() {
    let mut app = app(museum(&WITH_SLOW));
    enter(&mut app, "/run slow one-plus-one");
    enter(&mut app, "/run standin one-plus-one");
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(shown.contains("A run is still going. Press esc to stop it first."), "{shown}");
    app.stop();
    wait_idle(&mut app);
}

#[test]
fn tab_completes_commands_then_systems_then_examples() {
    let mut app = app(museum(&TWO));
    type_text(&mut app, "/ru");
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.input(), "/run ");
    type_text(&mut app, "st");
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.input(), "/run standin ");
    type_text(&mut app, "fac");
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.input(), "/run standin factoring");
}

#[test]
fn arrows_move_the_popup_selection_and_enter_takes_it() {
    let mut app = app(museum(&TWO));
    type_text(&mut app, "/");
    press(&mut app, KeyCode::Down);
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(reversed_rows(&buffer), [13], "the second row is selected");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.input(), "", "/list takes an optional shelf, so Enter runs it");
    assert!(screen(&draw(&mut app, 120, 40)).contains("Systems in this build: 2"));
}

#[test]
fn enter_on_a_command_that_needs_arguments_keeps_editing() {
    let mut app = app(museum(&TWO));
    type_text(&mut app, "/ab");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.input(), "/about ");
    let shown = screen(&draw(&mut app, 80, 24));
    assert!(shown.contains("standin-b"), "the popup now offers systems:\n{shown}");
}

#[test]
fn esc_closes_the_popup_until_the_line_changes() {
    let mut app = app(Museum::empty());
    type_text(&mut app, "/");
    press(&mut app, KeyCode::Esc);
    assert!(reversed_rows(&draw(&mut app, 80, 24)).is_empty());
    type_text(&mut app, "h");
    assert_eq!(reversed_rows(&draw(&mut app, 80, 24)).len(), 1);
}

#[test]
fn shift_enter_and_ctrl_j_add_lines_and_the_composer_grows() {
    let mut app = app(Museum::empty());
    type_text(&mut app, "one");
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    type_text(&mut app, "two");
    ctrl(&mut app, 'j');
    type_text(&mut app, "three");
    assert_eq!(app.input(), "one\ntwo\nthree");
    let text = rows(&draw(&mut app, 80, 24));
    assert!(text[18].starts_with("╭"), "five rows with the border:\n{}", text.join("\n"));
    assert!(text[19].starts_with("│ › one") && text[20].starts_with("│   two"));
    for _ in 0..10 {
        ctrl(&mut app, 'j');
    }
    let text = rows(&draw(&mut app, 80, 24));
    assert!(
        text[15].starts_with("╭"),
        "the composer stops growing at eight rows:\n{}",
        text.join("\n")
    );
}

#[test]
fn unknown_input_and_language_switches_answer_in_cells() {
    let mut app = app(Museum::empty());
    enter(&mut app, "/nope");
    enter(&mut app, "/lang ko");
    enter(&mut app, "/run all one-plus-one");
    assert_eq!(app.language(), Language::KOREAN);
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(shown.contains("✗ Unknown command /nope. Type /help for the list."), "{shown}");
    assert!(shown.contains("• Language: 한국어"), "{shown}");
    assert!(shown.contains("No proof systems are built into this binary yet."), "{shown}");
    enter(&mut app, "/clear");
    assert!(!screen(&draw(&mut app, 120, 40)).contains("nmtzk"));
    enter(&mut app, "/quit");
    assert!(app.should_quit());
}

#[test]
fn question_mark_on_an_empty_line_shows_the_shortcuts() {
    let mut app = app(Museum::empty());
    type_text(&mut app, "?");
    let shown = screen(&draw(&mut app, 80, 24));
    assert!(shown.contains("Keyboard shortcuts") && shown.contains("shift+drag"), "{shown}");
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.input(), "", "the key that closes the overlay is not typed");
    type_text(&mut app, "a?");
    assert_eq!(app.input(), "a?", "on a non-empty line ? is just a character");
}

/// A story that arrives one beat at a time, the way the cave will tell it.
struct Story;

impl Activity for Story {
    fn subject(&self) -> String {
        "the cave".to_string()
    }

    fn run(self: Box<Self>, outbox: &Outbox, control: &Control) {
        outbox.status("Mick walks into the cave");
        let cell = outbox.open(Entry::text(Kind::Story, "Beat one."));
        for beat in ["Beat two.", "Beat three."] {
            if control.is_cancelled() {
                return;
            }
            let mut doc = Doc::new();
            doc.line(vec![plain(beat)]);
            outbox.append(cell, doc);
        }
    }
}

#[test]
fn activities_stream_story_beats_into_one_cell() {
    let mut app = app(Museum::empty());
    assert!(app.start(Box::new(Story)));
    wait_idle(&mut app);
    let text = rows(&draw(&mut app, 80, 24));
    let first = text.iter().position(|row| row.starts_with("• Beat one.")).expect("story cell");
    assert!(text[first + 1].starts_with("  Beat two."));
    assert!(text[first + 2].starts_with("  Beat three."));
}

/// An activity that panics shows an error cell instead of taking the screen down.
struct Crash;

impl Activity for Crash {
    fn subject(&self) -> String {
        "crash".to_string()
    }

    fn run(self: Box<Self>, _outbox: &Outbox, _control: &Control) {
        panic!("deliberate");
    }
}

#[test]
fn a_crashing_activity_becomes_an_error_cell() {
    let mut app = app(Museum::empty());
    assert!(app.start(Box::new(Crash)));
    wait_idle(&mut app);
    assert!(
        screen(&draw(&mut app, 80, 24)).contains("✗ The task stopped unexpectedly: deliberate")
    );
}

#[test]
fn page_keys_and_the_wheel_scroll_without_following() {
    let mut app = app(museum(&TWO));
    for _ in 0..6 {
        enter(&mut app, "/examples");
    }
    draw(&mut app, 80, 24);
    press(&mut app, KeyCode::PageUp);
    assert!(app.start(Box::new(Story)));
    wait_idle(&mut app);
    let buffer = draw(&mut app, 80, 24);
    assert!(rows(&buffer)[23].contains("↓ new output"), "{}", screen(&buffer));
    app.handle_event(crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));
    for _ in 0..40 {
        press(&mut app, KeyCode::PageDown);
    }
    let buffer = draw(&mut app, 80, 24);
    assert!(!rows(&buffer)[23].contains("new output"), "back at the bottom:\n{}", screen(&buffer));
}
