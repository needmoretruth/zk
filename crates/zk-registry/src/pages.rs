//! The catalog pages compiled into this binary; `build.rs` writes the table from what is on disk.

include!(concat!(env!("OUT_DIR"), "/pages.rs"));

/// The page for `id` in exactly the language `code`, without falling back.
pub(crate) fn find(id: &str, code: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(page_id, page_code, _)| *page_id == id && *page_code == code)
        .map(|(_, _, text)| *text)
}
