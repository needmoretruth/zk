//! The languages this program is read in, and how a screen declares what it says.
//!
//! English is the source of truth: it is written first and it is never missing. Korean is a
//! choice the reader makes, and any line that has no Korean yet falls back to the English rather
//! than to a blank — a half-translated screen still reads.
//!
//! Engines never reach into a phrase table. They return numbers and enums, and the screen decides
//! how to say them. That is why a new language costs one column in each table and nothing else.

mod language;

pub use language::Language;

/// Declares a phrase table, one column per language.
///
/// Every crate that shows text calls this for its own phrases, so two parts being written at the
/// same time never edit the same file. `en` is required and is what a missing column falls back
/// to; every other column is keyed by the language's code.
///
/// ```
/// mod phrases {
///     zk_i18n::messages! {
///         Title { en: "Zero-knowledge proofs", ko: "영지식 증명" },
///         Verified { en: "Verified" },
///     }
/// }
/// use phrases::Msg;
/// use zk_i18n::Language;
///
/// assert_eq!(Msg::Title.text(Language::KOREAN), "영지식 증명");
/// assert_eq!(Msg::Verified.text(Language::KOREAN), "Verified");
/// ```
#[macro_export]
macro_rules! messages {
    ($( $(#[$doc:meta])* $key:ident { en: $en:literal $(, $lang:ident : $text:literal )* $(,)? } ),* $(,)?) => {
        /// One line of text, named by what it says rather than where it appears.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Msg { $( $(#[$doc])* $key, )* }

        impl Msg {
            /// The line in the chosen language, falling back to English.
            pub fn text(self, language: $crate::Language) -> &'static str {
                let code = language.code();
                match self {
                    $(
                        Msg::$key => {
                            $( if code == stringify!($lang) { return $text; } )*
                            $en
                        }
                    )*
                }
            }

            /// Every phrase, so a test can walk the whole table.
            pub const ALL: &'static [Msg] = &[ $( Msg::$key, )* ];
        }
    };
}

#[cfg(test)]
mod tests {
    use super::Language;

    messages! {
        Both { en: "Proof", ko: "증명" },
        EnglishOnly { en: "Verifier" },
    }

    #[test]
    fn a_translated_line_is_read_in_the_chosen_language() {
        assert_eq!(Msg::Both.text(Language::ENGLISH), "Proof");
        assert_eq!(Msg::Both.text(Language::KOREAN), "증명");
    }

    #[test]
    fn a_missing_translation_falls_back_to_english_not_to_a_blank() {
        assert_eq!(Msg::EnglishOnly.text(Language::KOREAN), "Verifier");
    }

    #[test]
    fn the_table_lists_every_phrase() {
        assert_eq!(Msg::ALL, &[Msg::Both, Msg::EnglishOnly]);
    }
}
