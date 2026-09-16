//! What the screen looks like at 80×24 and 120×40.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use crossterm::event::KeyCode;
use ratatui::style::{Color, Modifier};
use support::*;
use zk_core::ExampleId;
use zk_tui::plain::Printer;
use zk_tui::views::Comparison;
use zk_tui::{Look, Museum};

const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];

#[test]
fn welcome_box_composer_and_footer_sit_where_the_design_puts_them() {
    for (width, height) in SIZES {
        let mut app = app(Museum::empty());
        let buffer = draw(&mut app, width, height);
        let text = rows(&buffer);
        assert!(text[0].starts_with("╭"), "{width}×{height}:\n{}", screen(&buffer));
        assert!(text[1].starts_with("│ nmtzk v"), "{}", screen(&buffer));
        for expected in [
            "/list",
            "/examples",
            "/run all one-plus-one",
            "/lang ko",
            "No proof systems are built into this binary yet.",
        ] {
            assert!(
                find(&buffer, expected).is_some(),
                "missing {expected} at {width}×{height}:\n{}",
                screen(&buffer)
            );
        }
        // Composer: three rows with a border, directly above the one-row footer.
        let composer_top = height - 4;
        assert!(text[usize::from(composer_top)].starts_with("╭"), "{}", screen(&buffer));
        assert!(
            text[usize::from(composer_top) + 1].starts_with("│ › Type / for commands"),
            "{}",
            screen(&buffer)
        );
        assert!(text[usize::from(height) - 1].contains("/ commands · ? shortcuts · ctrl+c quit"));
        assert_eq!(
            buffer[(2, composer_top + 1)].fg,
            Color::Cyan,
            "the prompt glyph is the accent colour"
        );
        assert!(buffer[(2, 1)].modifier.contains(Modifier::BOLD), "nmtzk is bold");
        assert_theme_respected(&buffer);
    }
}

#[test]
fn the_slash_popup_opens_directly_above_the_composer_with_eight_rows() {
    for (width, height) in SIZES {
        let mut app = app(Museum::empty());
        type_text(&mut app, "/");
        let buffer = draw(&mut app, width, height);
        let text = rows(&buffer);
        let composer_top = usize::from(height - 4);
        let first = composer_top - 8;
        assert!(text[first].contains("/help"), "{}", screen(&buffer));
        assert!(text[first + 1].contains("/list [shelf]"));
        assert!(text[composer_top - 1].contains("/quit"));
        assert!(!text[first - 1].contains("/"), "only eight rows: {}", text[first - 1]);
        assert_eq!(reversed_rows(&buffer), [first as u16], "only the selected row is reversed");
        assert_eq!(buffer[(1, first as u16)].fg, Color::Cyan);
        let (x, y) = find(&buffer, "show the commands").expect("description");
        assert_eq!(y as usize, first);
        let (dx, dy) = find(&buffer, "the systems on the shelves").expect("description");
        assert_eq!(buffer[(dx, dy)].fg, Color::DarkGray, "descriptions are dimmed");
        assert!(dx >= x, "descriptions line up in a column");
        assert_theme_respected_except_selection(&buffer, first as u16);
    }
}

fn assert_theme_respected_except_selection(buffer: &ratatui::buffer::Buffer, selected: u16) {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            assert_eq!(cell.bg, Color::Reset);
            if y != selected {
                assert!(!cell.modifier.contains(Modifier::REVERSED));
            }
        }
    }
}

#[test]
fn a_finished_run_cell_shows_verdict_cautions_timings_attacks_and_the_scan() {
    let mut app = app(museum(&TWO));
    enter(&mut app, "/run standin one-plus-one");
    wait_idle(&mut app);
    let buffer = draw(&mut app, 120, 40);
    let shown = screen(&buffer);
    for expected in [
        "› /run standin one-plus-one",
        "• ✓ Stand-in accepted the honest proof of one-plus-one",
        "⚠ Teaching implementation · not audited",
        "⚠ Not zero-knowledge as run here",
        "R1CS over toy field",
        "constraints 1,234 · variables 1,240",
        "Timings · measured on this machine, now",
        "setup material  4.00 KiB",
        "First bytes · 48 of 64 bytes",
        "07070707",
        "Attacks · 3 of 3 held",
        "✓ flip proof byte 32: rejected",
        "✓ prove a false claim: the prover refused: the answer is not 1 + 1",
        "⚠ found byte for byte in the proof: answer",
        "A smoke detector, not a proof of zero knowledge.",
    ] {
        assert!(shown.contains(expected), "missing {expected:?} in:\n{shown}");
    }
    let (x, y) = find(&buffer, "✓ Stand-in").expect("verdict");
    assert_eq!(buffer[(x, y)].fg, Color::Green);
    let (x, y) = find(&buffer, "⚠ Teaching").expect("caution");
    assert_eq!(buffer[(x, y)].fg, Color::Yellow);
    assert_theme_respected(&buffer);

    let small = draw(&mut app, 80, 24);
    assert!(
        screen(&small).contains("A smoke detector"),
        "the transcript follows the bottom:\n{}",
        screen(&small)
    );
    assert!(rows(&small).iter().all(|row| row.chars().count() == 80));
}

#[test]
fn the_comparison_table_fills_in_row_by_row() {
    let printer = Printer::new(120, false, Look::default());
    let mut table = Comparison::new(&TWO, ExampleId::OnePlusOne);
    table.start(0);
    let running = printer.entry(&table.entry(zk_i18n::Language::ENGLISH));
    assert!(running.contains("running"), "{running}");
    assert!(running.contains("waiting"), "{running}");

    let mut app = app(museum(&TWO));
    enter(&mut app, "/run all one-plus-one");
    wait_idle(&mut app);
    for (width, height) in SIZES {
        let buffer = draw(&mut app, width, height);
        let text = rows(&buffer);
        let header = text.iter().position(|row| row.starts_with("  System")).expect("header row");
        let order =
            ["System", "Setup", "Prove", "Verify", "Proof", "Verdict", "Attacks", "Secrets"];
        let positions: Vec<usize> =
            order.iter().map(|word| text[header].find(word).expect(word)).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{}", text[header]);
        assert!(buffer[(2, header as u16)].modifier.contains(Modifier::BOLD), "the header is bold");
        assert!(
            text[header + 1].contains("Stand-in") && text[header + 1].contains("✓ accepted"),
            "{}",
            screen(&buffer)
        );
        assert!(text[header + 2].contains("Stand-in B"));
        assert!(
            !text[header + 1].contains('…'),
            "nothing is cut at {width} columns: {}",
            text[header + 1]
        );
        assert!(
            screen(&buffer).contains("2 of 2 accepted the honest proof and held every attack."),
            "{}",
            screen(&buffer)
        );
        assert_theme_respected(&buffer);
    }
    let wide = draw(&mut app, 120, 40);
    let text = rows(&wide);
    let header = text.iter().position(|row| row.starts_with("  System")).expect("header");
    assert!(
        text[header].contains("Shelf"),
        "wide terminals keep the shelf column: {}",
        text[header]
    );
    assert!(text[header + 2].contains("Homemade"));
    assert!(text[header + 1].contains("3/3"));
}

#[test]
fn about_opens_the_page_in_a_full_screen_pager() {
    for (width, height) in SIZES {
        let mut app = app(museum(&TWO));
        enter(&mut app, "/about standin");
        assert!(app.pager_open());
        let buffer = draw(&mut app, width, height);
        let text = rows(&buffer);
        assert!(
            text[0].starts_with("Stand-in") && text[0].trim_end().ends_with("q to close"),
            "{}",
            screen(&buffer)
        );
        assert!(text[usize::from(height) - 1].contains("q close"));
        let (x, y) = find(&buffer, "History").expect("heading");
        assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
        let (x, y) = find(&buffer, "toy").expect("strong");
        assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
        assert!(find(&buffer, "• first").is_some(), "{}", screen(&buffer));
        assert!(find(&buffer, "2026  written").is_some(), "{}", screen(&buffer));
        assert!(find(&buffer, "    fn prove() {}").is_some());
        let (x, y) = find(&buffer, "source (https://example.org/standin)").expect("link");
        assert_eq!(buffer[(x, y)].fg, Color::Cyan);
        assert_theme_respected(&buffer);

        for key in [
            KeyCode::Char('j'),
            KeyCode::Char('G'),
            KeyCode::Char('g'),
            KeyCode::PageDown,
            KeyCode::Char('k'),
        ] {
            press(&mut app, key);
            draw(&mut app, width, height);
        }
        press(&mut app, KeyCode::Char('q'));
        assert!(!app.pager_open());
        let buffer = draw(&mut app, width, height);
        assert!(screen(&buffer).contains("ctrl+o opens the page again."), "{}", screen(&buffer));
        ctrl(&mut app, 'o');
        assert!(app.pager_open(), "ctrl+o reopens the last page");
        press(&mut app, KeyCode::Esc);
        assert!(!app.pager_open());
    }
}

#[test]
fn a_missing_page_is_said_plainly_and_opens_nothing() {
    let mut app = app(museum(&TWO));
    enter(&mut app, "/about standin-b");
    assert!(!app.pager_open());
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(shown.contains("No page has been written for Stand-in B yet."), "{shown}");
    enter(&mut app, "/about groth16");
    let shown = screen(&draw(&mut app, 120, 40));
    assert!(shown.contains("✗ Unknown system 'groth16'."), "{shown}");
}

#[test]
fn a_too_small_terminal_gets_one_line_asking_for_more_room() {
    for (width, height) in [(59, 24), (80, 15), (40, 10)] {
        let mut app = app(Museum::empty());
        let buffer = draw(&mut app, width, height);
        let text = rows(&buffer);
        let filled: Vec<&String> = text.iter().filter(|row| !row.trim().is_empty()).collect();
        assert_eq!(filled.len(), 1, "{width}×{height}:\n{}", screen(&buffer));
        assert!(filled[0].starts_with("Please enlarge the terminal"), "{}", filled[0]);
    }
    let mut app = app(Museum::empty());
    assert!(screen(&draw(&mut app, 60, 16)).contains("nmtzk"), "60×16 is big enough");
}

#[test]
fn ascii_mode_swaps_every_symbol() {
    let mut app = zk_tui::App::new(
        Museum::empty(),
        zk_tui::Settings {
            language: zk_i18n::Language::ENGLISH,
            look: Look { color: false, ascii: true },
        },
    );
    let buffer = draw(&mut app, 80, 24);
    let shown = screen(&buffer);
    assert!(shown.starts_with("+-"), "{shown}");
    assert!(shown.contains("| > Type / for commands"), "{shown}");
    assert!(shown.contains("! No proof systems"), "{shown}");
    assert!(shown.is_ascii(), "{shown}");
    assert!(buffer.content.iter().all(|cell| cell.fg == Color::Reset), "NO_COLOR paints nothing");
}
