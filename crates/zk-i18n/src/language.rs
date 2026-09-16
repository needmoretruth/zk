//! The languages this program speaks.
//!
//! A language is a code and the name that language calls itself by. Adding one is an entry in
//! [`Language::ALL`] and one more column in each phrase table; nothing else in the program changes.

use std::fmt;

/// One language the program can be read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Language {
    code: &'static str,
    endonym: &'static str,
}

impl Language {
    /// The source of truth. Never missing a line.
    pub const ENGLISH: Language = Language { code: "en", endonym: "English" };
    /// 한국어.
    pub const KOREAN: Language = Language { code: "ko", endonym: "한국어" };

    /// Every language, in the order a reader picks from. English first because it is complete;
    /// the rest alphabetically by code, so the list stays predictable as it grows.
    pub const ALL: &'static [Language] = &[Language::ENGLISH, Language::KOREAN];

    /// The BCP-47 language subtag, e.g. `ko`. Phrase tables and the `--lang` flag match on it.
    pub const fn code(self) -> &'static str {
        self.code
    }

    /// What this language calls itself. A reader finds their own language by looking for a word
    /// they recognise, so this is never translated.
    pub const fn endonym(self) -> &'static str {
        self.endonym
    }

    /// A language by code, or `None` when this build does not speak it.
    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL.iter().copied().find(|language| language.code == code)
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::ENGLISH
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.endonym)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_default_and_comes_first() {
        assert_eq!(Language::default(), Language::ENGLISH);
        assert_eq!(Language::ALL[0], Language::ENGLISH);
    }

    #[test]
    fn codes_survive_a_round_trip_and_unknown_codes_are_refused() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), Some(*language));
        }
        assert_eq!(Language::from_code("xx"), None);
    }

    #[test]
    fn every_language_has_a_distinct_code() {
        let mut codes: Vec<&str> = Language::ALL.iter().map(|l| l.code()).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), total, "two languages share a code");
    }
}
