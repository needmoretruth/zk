//! The hands-on activities on the command line: the same cells as the screen, JSON lines for
//! scripts and the exit codes a script can branch on.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

fn nmtzk(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nmtzk"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run nmtzk")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Every line of standard output parsed as JSON.
fn records(output: &Output) -> Vec<serde_json::Value> {
    stdout(output).lines().map(|line| serde_json::from_str(line).expect("a JSON line")).collect()
}

/// A pool directory of this test's own: a process-wide counter and the process ID keep parallel
/// tests apart, and it is removed afterwards, so no test touches the home directory.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> ScratchDir {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = format!("nmtzk-cli-{label}-{}-{unique}", std::process::id());
        ScratchDir(std::env::temp_dir().join(name))
    }

    fn path(&self) -> &str {
        self.0.to_str().expect("a UTF-8 temporary directory")
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // Leftovers in the temporary directory are harmless if removal fails.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cave_films_every_scene_as_text_and_as_json_lines() {
    let text = nmtzk(&["cave", "--scenes", "3"]);
    assert!(text.status.success());
    let shown = stdout(&text);
    assert!(shown.contains("⚠ Homemade · not audited"), "{shown}");
    assert!(shown.contains("⚠ The trust is in the wall."), "{shown}");
    assert_eq!(shown.matches("✓ Scene ").count(), 3, "{shown}");
    assert!(!shown.contains('\x1b'), "no colour codes into a pipe");

    let json = records(&nmtzk(&["--json", "cave", "impostor", "--scenes", "200"]));
    let summary = json.last().expect("a summary");
    assert_eq!(summary["mode"], "impostor");
    assert_eq!(summary["convinced"], false);
    assert_eq!(json.len() as u64 - 1, summary["played"].as_u64().unwrap());

    let wrong = nmtzk(&["cave", "court", "--scenes", "0"]);
    assert_eq!(wrong.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("--scenes takes a whole number"));
}

#[test]
fn trio_stops_a_cheat_at_the_round_that_catches_it() {
    let output = nmtzk(&["--json", "trio", "cheat", "bad-card", "--rounds", "200"]);
    assert!(output.status.success());
    let json = records(&output);
    let summary = json.last().expect("a summary");
    assert_eq!(summary["verdict"], "rejected");
    let caught = json[json.len() - 2].clone();
    assert_eq!(caught["passed"], false);
    assert_eq!(caught["round"], summary["caught_at"]);
    let failed: Vec<_> = caught["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|check| check["passed"] == false)
        .map(|check| check["check"].as_str().unwrap())
        .collect();
    assert_eq!(failed, ["cards-multiply"]);
}

#[test]
fn pool_keeps_its_ledger_in_the_data_dir_and_asks_before_a_reset() {
    let dir = ScratchDir::new("pool");
    let pool = |words: &[&str]| {
        let mut args = vec!["--data-dir", dir.path(), "pool"];
        args.extend(words);
        nmtzk(&args)
    };
    for words in
        [&["wallet", "new", "alice"][..], &["faucet", "alice", "500"], &["shield", "alice", "300"]]
    {
        let output = pool(words);
        assert!(output.status.success(), "{words:?}: {}", String::from_utf8_lossy(&output.stderr));
    }
    let shown = stdout(&pool(&["ledger"]));
    assert!(shown.contains("value in the pool     300") && shown.contains("alice 200"), "{shown}");
    assert!(dir.0.join("pool").join("groth16").join("ledger.json").exists());

    assert_eq!(pool(&["send", "alice", "nobody", "5"]).status.code(), Some(1));
    let refused = pool(&["reset"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(stdout(&refused).contains("/pool reset --yes"), "{}", stdout(&refused));
    assert!(dir.0.join("pool").join("groth16").join("ledger.json").exists());

    assert!(pool(&["reset", "--yes"]).status.success());
    assert!(stdout(&pool(&["wallets"])).contains("No wallets yet."));
}

#[test]
fn ceremony_checks_every_turn_and_catches_both_cheats() {
    let output = nmtzk(&["--json", "ceremony", "tau", "--participants", "3"]);
    assert!(output.status.success());
    let json = records(&output);
    let steps = |step: &str| json.iter().filter(|record| record["step"] == step).count();
    assert_eq!((steps("turn"), steps("check"), steps("chain"), steps("cheat")), (3, 3, 1, 4));
    assert!(
        json.iter()
            .filter(|record| record["step"] == "cheat")
            .all(|cheat| cheat["caught_by"].is_string())
    );
}

#[test]
fn forge_rewrites_a_bctv14_proof_that_only_the_flawed_keys_accept() {
    let output = nmtzk(&["forge", "bctv14"]);
    assert!(output.status.success());
    let shown = stdout(&output);
    assert!(shown.contains("✗ The proof rewritten with the extra points"), "{shown}");
    assert!(shown.contains("✓ The same rewrite with corrected keys"), "{shown}");
    assert_eq!(nmtzk(&["forge", "bctv12"]).status.code(), Some(2));
}
