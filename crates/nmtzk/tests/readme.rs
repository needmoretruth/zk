//! The systems tables in `README.md` and `README.ko.md` are generated from what each system
//! declares about itself, and must not drift from it.
//!
//! When a system is added or its metadata changes, regenerate them with
//! `NMTZK_BLESS=1 cargo test -p nmtzk --test readme`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use zk_core::catalog::SystemMeta;
use zk_i18n::Language;

const BEGIN: &str = "<!-- systems:begin -->";
const END: &str = "<!-- systems:end -->";
const BLESS: &str = "NMTZK_BLESS=1 cargo test -p nmtzk --test readme";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(file: &str, language: Language) {
    let path = workspace_root().join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let begin =
        text.find(BEGIN).unwrap_or_else(|| panic!("{file} has no `{BEGIN}` marker")) + BEGIN.len();
    let end = begin
        + text[begin..]
            .find(END)
            .unwrap_or_else(|| panic!("{file} has no `{END}` after `{BEGIN}`"));

    let metas: Vec<&SystemMeta> =
        zk_registry::systems().iter().map(|system| system.meta()).collect();
    let expected = zk_tui::views::readme_table(&metas, language);

    if std::env::var("NMTZK_BLESS").as_deref() == Ok("1") {
        let blessed = format!("{}\n\n{}\n\n{}", &text[..begin], expected.trim_end(), &text[end..]);
        std::fs::write(&path, blessed)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        return;
    }

    let normalise = |table: &str| -> String {
        table.trim().lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
    };
    assert!(
        normalise(&text[begin..end]) == normalise(&expected),
        "the systems table in {file} is out of date with the systems' metadata; run `{BLESS}` \
         to regenerate it.\n\nexpected:\n{expected}"
    );
}

#[test]
fn english_readme_table_matches_the_metadata() {
    check("README.md", Language::ENGLISH);
}

#[test]
fn korean_readme_table_matches_the_metadata() {
    check("README.ko.md", Language::KOREAN);
}
