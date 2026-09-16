//! The binary as a script sees it: output, JSON lines and exit codes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Read;
use std::process::{Command, Stdio};

fn nmtzk(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nmtzk"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run nmtzk")
}

#[test]
fn examples_print_as_text_and_as_json_lines() {
    let text = nmtzk(&["examples"]);
    assert!(text.status.success());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.contains("one-plus-one  The answer sealed in this envelope is 1 + 1."),
        "{stdout}"
    );
    assert!(!stdout.contains('\x1b'), "no colour codes into a pipe");

    let json = nmtzk(&["--json", "examples"]);
    assert!(json.status.success());
    let lines: Vec<&str> = std::str::from_utf8(&json.stdout).expect("utf-8").lines().collect();
    assert_eq!(lines.len(), 7);
    assert!(
        lines[0].starts_with('{') && lines[0].contains("\"id\":\"one-plus-one\""),
        "{}",
        lines[0]
    );
}

#[test]
fn unknown_names_exit_with_a_usage_error() {
    let run = nmtzk(&["run", "no-such-system", "one-plus-one"]);
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("Unknown system 'no-such-system'"));
    assert_eq!(nmtzk(&["run", "all", "two-plus-two"]).status.code(), Some(2));
    assert_eq!(nmtzk(&["about", "no-such-system"]).status.code(), Some(2));
    assert_eq!(nmtzk(&["--lang", "xx", "list"]).status.code(), Some(2));
}

#[test]
fn without_a_terminal_the_program_prints_the_welcome_box_instead_of_starting_the_tui() {
    let output = nmtzk(&[]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("nmtzk v"));
}

#[test]
fn a_reader_that_stops_early_does_not_crash_the_program() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nmtzk"))
        .args(["examples"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nmtzk");
    let mut stdout = child.stdout.take().expect("stdout");
    let mut first = [0u8; 1];
    stdout.read_exact(&mut first).expect("some output");
    drop(stdout);
    let output = child.wait_with_output().expect("wait");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    assert_ne!(output.status.code(), Some(101));
}
