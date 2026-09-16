//! Compiles every catalog page that exists into the binary, and nothing for pages that do not.
//!
//! Pages live at `catalog/<id>/README.md` (English) and `catalog/<id>/README.<code>.md` (another
//! language). A page written tomorrow has to appear without anyone editing a table, and a page not
//! written yet must not break the build, so the table of `include_str!` entries is generated from
//! what is on disk when this crate is compiled.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());
    let catalog = manifest.join("..").join("..").join("catalog");

    println!("cargo::rerun-if-changed=build.rs");
    let mut pages = Vec::new();
    if catalog.is_dir() {
        // Cargo rescans a watched directory recursively, so a new page or language triggers a rerun.
        println!("cargo::rerun-if-changed={}", catalog.display());
        pages = find_pages(&catalog);
    } else {
        // A missing path would make cargo rerun this script on every build. Until the directory
        // exists, rerun when this crate changes instead, which is what wiring in a system does.
        println!("cargo::rerun-if-changed=src");
        println!("cargo::rerun-if-changed=Cargo.toml");
    }
    pages.sort();

    let mut code = String::from(
        "/// Catalog pages found when this crate was built: (system id, language code, markdown).\n\
         pub(crate) static PAGES: &[(&str, &str, &str)] = &[\n",
    );
    for (id, language, path) in &pages {
        let _ = writeln!(code, "    ({id:?}, {language:?}, include_str!({path:?})),");
    }
    code.push_str("];\n");
    if let Err(error) = fs::write(out_dir.join("pages.rs"), code) {
        panic!("could not write the generated page table: {error}");
    }
}

/// `(id, language code, absolute path)` for every page file under `catalog`.
fn find_pages(catalog: &Path) -> Vec<(String, String, String)> {
    let mut pages = Vec::new();
    for system in read_dir(catalog) {
        let Some(id) = file_name(&system).filter(|name| is_id(name)) else { continue };
        if !system.is_dir() {
            continue;
        }
        for file in read_dir(&system) {
            let Some(language) = file_name(&file).and_then(|name| language_of(&name)) else {
                continue;
            };
            let Some(path) =
                fs::canonicalize(&file).ok().and_then(|p| p.to_str().map(String::from))
            else {
                continue;
            };
            pages.push((id.clone(), language, path));
        }
    }
    pages
}

fn read_dir(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
        .unwrap_or_default()
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name().and_then(|name| name.to_str()).map(String::from)
}

/// System IDs are lowercase ASCII letters, digits and hyphens; anything else is not a page folder.
fn is_id(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// `README.md` is English; `README.<code>.md` is the language with that two- or three-letter code.
fn language_of(name: &str) -> Option<String> {
    if name == "README.md" {
        return Some("en".to_string());
    }
    let code = name.strip_prefix("README.")?.strip_suffix(".md")?;
    let valid = (2..=3).contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_lowercase());
    valid.then(|| code.to_string())
}
