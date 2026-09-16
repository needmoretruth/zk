//! The hands-on activities on screen, driven by the real engines with small counts: the cave, Trio,
//! the pool, the ceremony and the forgery.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use crossterm::event::KeyCode;
use ratatui::style::Color;
use support::*;
use zk_tui::Museum;

const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];

/// Runs `line` to the end with every beat at once and returns the transcript at 120 columns, tall
/// enough to hold every cell.
fn finished(line: &str) -> String {
    let mut app = instant_app(Museum::empty(), None);
    enter(&mut app, line);
    wait_idle(&mut app);
    screen(&draw(&mut app, 120, 200))
}

fn count(shown: &str, needle: &str) -> usize {
    shown.matches(needle).count()
}

#[test]
fn the_cave_is_drawn_forked_and_every_scene_arrives_as_a_beat() {
    let shown = finished("/cave --scenes 4");
    for expected in [
        "• Ali Baba's cave: the demonstration · one-plus-one · 4 scenes",
        "⚠ Homemade · not audited",
        "dead end ┃ dead end",
        "╰─┬─╯",
        "entrance",
        "⚠ The trust is in the wall. In the story, too, the wall is magic.",
        "After filming",
        "not on the tape: the camera stayed at the fork",
        "✓ The reporter is convinced: all 4 scenes came out on the called side.",
        "with a chance of 1 in 16.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    assert_eq!(count(&shown, "✓ Scene "), 4, "{shown}");
    let went_in = shown.lines().find(|row| row.contains("went in")).expect("hidden passages");
    assert!(went_in.contains('L') || went_in.contains('R'), "{went_in}");
}

#[test]
fn the_cave_fits_both_sizes_in_colour_and_in_ascii() {
    for (width, height) in SIZES {
        let mut app = instant_app(Museum::empty(), None);
        enter(&mut app, "/cave --scenes 3");
        wait_idle(&mut app);
        let buffer = draw(&mut app, width, height);
        assert_theme_respected(&buffer);
        let (x, y) = find(&buffer, "✓ The reporter").expect("verdict");
        assert_eq!(buffer[(x, y)].fg, Color::Green);
    }
    let mut ascii = zk_tui::App::new(
        Museum::empty(),
        zk_tui::Settings {
            look: zk_tui::Look { color: false, ascii: true },
            pace: zk_tui::activities::Pace::Instant,
            ..zk_tui::Settings::default()
        },
    );
    enter(&mut ascii, "/cave --scenes 2");
    wait_idle(&mut ascii);
    let shown = screen(&draw(&mut ascii, 120, 60));
    assert!(shown.contains("dead end # dead end") && shown.contains("+-+-+"), "{shown}");
    assert!(shown.is_ascii(), "{shown}");
}

#[test]
fn the_impostor_is_caught_and_the_jealous_edit_counts_what_it_cut() {
    let shown = finished("/cave impostor");
    assert!(shown.contains("· caught") && shown.contains("✓ Caught in scene "), "{shown}");
    assert_eq!(count(&shown, "✗ Scene "), 1, "only the scene that caught the double fails");

    let shown = finished("/cave jealous-edit --scenes 4");
    assert_eq!(count(&shown, "kept as scene "), 4, "{shown}");
    let summary = shown.lines().find(|row| row.contains("Takes filmed:")).expect("summary");
    assert!(summary.contains("Kept: 4."), "{summary}");
}

#[test]
fn the_court_counts_two_tapes_side_by_side() {
    let shown = finished("/cave court --scenes 3");
    assert_eq!(count(&shown, "Mick's tape: called"), 3, "{shown}");
    for expected in [
        "What the judge can count",
        "came out on the called side",
        "Averaged over 100 tapes of each kind",
        "✓ The judge cannot tell the tapes apart.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    let matching = shown.lines().find(|row| row.contains("came out on the called side")).unwrap();
    assert_eq!(matching.split_whitespace().rev().take(2).collect::<Vec<_>>(), ["3", "3"]);
}

#[test]
fn prior_agreement_and_the_apartment_building_tell_their_own_story() {
    let shown = finished("/cave prior-agreement --scenes 3");
    assert!(shown.contains("Calls agreed before filming:"), "{shown}");
    assert_eq!(count(&shown, "· no coin ·"), 3, "{shown}");
    let shown = finished("/cave apartment --scenes 3");
    assert_eq!(count(&shown, "✓ Floor "), 3, "{shown}");
    assert!(shown.contains("All 3 floors came out on the called side in one round."));
}

#[test]
fn esc_stops_the_cave_between_scenes_and_says_where() {
    let mut app = app(Museum::empty());
    enter(&mut app, "/cave --scenes 40");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !screen(&draw(&mut app, 120, 60)).contains("✓ Scene 1 ") {
        assert!(std::time::Instant::now() < deadline, "no scene arrived");
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.pump();
    }
    let working = screen(&draw(&mut app, 120, 60));
    assert!(working.contains("Scene ") && working.contains("esc to stop"), "{working}");
    press(&mut app, KeyCode::Esc);
    wait_idle(&mut app);
    let shown = screen(&draw(&mut app, 120, 60));
    assert!(shown.contains("⚠ Stopped after "), "{shown}");
    assert!(!shown.contains("After filming"), "a stopped filming is not reviewed:\n{shown}");
}

#[test]
fn trio_plays_rounds_with_every_check_and_catches_a_cheat() {
    let shown = finished("/trio --rounds 6");
    assert_eq!(count(&shown, "✓ Round "), 6, "{shown}");
    assert!(shown.contains("⚠ Homemade · not audited"));
    assert!(shown.contains("opened: ") && shown.contains("checks: fits the card ✓"), "{shown}");
    assert!(shown.contains("✓ The verifier accepts after 6 rounds."), "{shown}");
    assert!(shown.contains("(3/5)^6, about 1 in 21."), "{shown}");

    // Only a dealer card catches a rigged card, so a long session makes escaping all of it
    // vanishingly rare; the cheat stops at the first failed round anyway.
    let shown = finished("/trio cheat bad-card --rounds 200");
    assert!(shown.contains("⚠ The false claim is the true witness"), "{shown}");
    let caught = shown.lines().find(|row| row.contains("Caught in round")).expect("caught");
    assert!(caught.contains("does not satisfy c = a·b"), "{caught}");
    assert_eq!(count(&shown, "✗ Round "), 1, "{shown}");
}

#[test]
fn the_simulator_passes_every_check_without_a_secret() {
    let shown = finished("/trio simulate sudoku --rounds 5");
    assert!(shown.contains("Cards known in advance: "), "{shown}");
    assert_eq!(count(&shown, "✓ Round "), 5, "{shown}");
    assert!(shown.contains("All 5 simulated rounds passed the same checks"), "{shown}");
}

#[test]
fn the_ceremony_forges_with_toxic_waste_and_checks_every_turn() {
    let shown = finished("/ceremony toxic");
    for expected in [
        "⚠ Teaching implementation · not audited",
        "✗ The ordinary Groth16 verifier accepts the forged proof.",
        "✓ The same forgery with a guessed δ is rejected.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    let shown = finished("/ceremony tau --participants 3");
    assert_eq!(count(&shown, "did not keep it · fingerprint "), 3, "{shown}");
    assert_eq!(count(&shown, "✓ Turn "), 3, "{shown}");
    assert!(shown.contains("✓ Every turn checks out."), "{shown}");
    for check in ["builds on the previous string.", "consistent powers.", "shape."] {
        assert!(shown.contains(&format!("✓ Caught by the check: {check}")), "{check}:\n{shown}");
    }
    assert!(!shown.contains("✗"), "{shown}");
}

#[test]
fn colluders_forge_an_opening_until_one_secret_is_missing() {
    let shown = finished("/ceremony collude --participants 3");
    assert_eq!(count(&shown, "kept it · fingerprint"), 3, "{shown}");
    assert!(shown.contains("✓ A commitment to p(X)"), "{shown}");
    assert!(
        shown.contains("✗ With all 3 secrets, the colluders open the commitment to p(2) = 1109.")
    );
    assert!(shown.contains("✓ With 2 of 3 secrets, the same forgery is rejected."), "{shown}");
}

#[test]
fn the_bctv14_forgery_is_accepted_only_with_the_flawed_keys() {
    let shown = finished("/forge bctv14");
    for expected in [
        "Forging BCTV14: CVE-2019-7167",
        "⚠ Teaching implementation · not audited",
        "✓ An honest proof of the true claim",
        "✓ The same proof offered for the false claim, an envelope that holds 3: rejected.",
        "✗ The proof rewritten with the extra points: the ordinary verifier accepts it",
        "✓ The same rewrite with corrected keys, which leave the extra points out: rejected.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
}

#[test]
fn a_pool_receipt_shows_what_the_world_sees_and_what_only_the_wallet_sees() {
    let dir = ScratchDir::new("receipt");
    let mut app = instant_app(Museum::empty(), Some(dir.0.clone()));
    for line in ["/pool wallet new alice", "/pool faucet alice 500", "/pool shield alice 300"] {
        enter(&mut app, line);
        wait_idle(&mut app);
    }
    let shown = screen(&draw(&mut app, 120, 200));
    for expected in [
        "✓ alice has a new wallet",
        "nothing: a new key never reaches the ledger",
        "✓ The faucet gave alice 500 transparent coins · transaction 0",
        "✓ alice shielded 300 · transaction 1",
        "What the world sees",
        "alice pays 300",
        "What only the wallets see",
        "note 1      300 to alice · position 0 · handed to the wallet file on this machine",
        "✓ Groth16's verifier accepts the proof.",
        "✓ the value in the pool stays at zero or above: 0 → 300",
        "✓ the tree has room for two more notes",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    assert!(dir.0.join("pool").join("groth16").join("ledger.json").exists());
    for (width, height) in SIZES {
        assert_theme_respected(&draw(&mut app, width, height));
    }
}

#[test]
fn the_pool_ledger_errors_attacks_and_reset_confirmation() {
    let dir = ScratchDir::new("ledger");
    let mut app = instant_app(Museum::empty(), Some(dir.0.clone()));
    let lines = [
        "/pool wallet new bob",
        "/pool faucet bob 40",
        "/pool ledger",
        "/pool wallet carol",
        "/pool attack steal",
        "/pool reset",
    ];
    for line in lines {
        enter(&mut app, line);
        wait_idle(&mut app);
    }
    let shown = screen(&draw(&mut app, 120, 200));
    for expected in [
        "• The ledger · Groth16 · what the world sees",
        "bob 40",
        "✗ No wallet is named carol.",
        "• Attack: steal · Groth16",
        "✓ refused: the proof does not verify",
        "✓ The ledger running the honest circuit refused every forgery.",
        "⚠ This deletes the Groth16 pool's ledger and every wallet in it.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    enter(&mut app, "/pool reset --yes");
    wait_idle(&mut app);
    enter(&mut app, "/pool wallets");
    wait_idle(&mut app);
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(
        shown.contains("The Groth16 pool is empty again.") && shown.contains("No wallets yet.")
    );
}

#[test]
fn help_the_popup_and_mistakes_know_the_new_commands() {
    let mut app = app(Museum::empty());
    enter(&mut app, "/help");
    let shown = screen(&draw(&mut app, 120, 40));
    for usage in
        ["/cave [mode]", "/trio [mode]", "/pool [action]", "/ceremony [part]", "/forge bctv14"]
    {
        assert!(shown.contains(usage), "missing {usage:?} in:\n{shown}");
    }
    type_text(&mut app, "/cave ");
    let shown = screen(&draw(&mut app, 80, 24));
    assert!(shown.contains("jealous-edit") && shown.contains("the court"), "{shown}");
    press(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'c');
    type_text(&mut app, "/trio cheat ");
    assert!(screen(&draw(&mut app, 80, 24)).contains("rewrite-hidden"));
    ctrl(&mut app, 'c');
    type_text(&mut app, "/pool attack co");
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.input(), "/pool attack counterfeit ");
    ctrl(&mut app, 'c');
    enter(&mut app, "/cave ring");
    enter(&mut app, "/trio --rounds 0");
    enter(&mut app, "/pool shield alice lots");
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(shown.contains("✗ Unknown choice 'ring'. Choices: demonstration, impostor"), "{shown}");
    assert!(shown.contains("✗ --rounds takes a whole number from 1 to 1000, not '0'."), "{shown}");
    assert!(shown.contains("✗ 'lots' is not an amount."), "{shown}");
}
