//! Every word this program shows, one table per subject.
//!
//! English is written first and is never missing; another language is one more column in these
//! tables, added by the museum's editor. Engines return enums and numbers and never text, so this is
//! the only place a sentence can change.

pub(crate) mod catalog;
pub(crate) mod run;
pub(crate) mod ui;

/// Replaces `{name}` in `template` with the value given for `name`; unknown names are left as they are.
pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let replaced = after.find('}').and_then(|close| {
            let name = &after[..close];
            values.iter().find(|(key, _)| *key == name).map(|(_, value)| (close, *value))
        });
        match replaced {
            Some((close, value)) => {
                out.push_str(value);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A phrase key an engine returned (`"public-inputs"`) as readable words when no table has it.
pub(crate) fn humanise(key: &str) -> String {
    key.replace(['-', '_'], " ")
}

#[cfg(test)]
mod tests {
    use zk_i18n::Language;

    use super::*;

    /// Checks one table: English present and trimmed, placeholders well formed.
    fn check(name: &str, texts: Vec<&'static str>) {
        assert!(!texts.is_empty(), "the {name} table is empty");
        for text in texts {
            assert!(!text.trim().is_empty(), "a phrase in {name} has no English");
            assert_eq!(text, text.trim(), "{name}: {text:?} has surrounding whitespace");
            let mut depth = 0i32;
            for character in text.chars() {
                match character {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                assert!((0..=1).contains(&depth), "{name}: unbalanced braces in {text:?}");
            }
            assert_eq!(depth, 0, "{name}: unbalanced braces in {text:?}");
        }
    }

    #[test]
    fn every_phrase_has_english() {
        let english = Language::ENGLISH;
        check("ui", ui::Msg::ALL.iter().map(|msg| msg.text(english)).collect());
        check("run", run::Msg::ALL.iter().map(|msg| msg.text(english)).collect());
        check("catalog", catalog::Msg::ALL.iter().map(|msg| msg.text(english)).collect());
    }

    #[test]
    fn every_phrase_reads_in_every_language() {
        for language in zk_i18n::Language::ALL {
            for msg in ui::Msg::ALL {
                assert!(!msg.text(*language).is_empty());
            }
            for msg in run::Msg::ALL {
                assert!(!msg.text(*language).is_empty());
            }
            for msg in catalog::Msg::ALL {
                assert!(!msg.text(*language).is_empty());
            }
        }
    }

    #[test]
    fn fill_replaces_named_values_only() {
        assert_eq!(fill("Proving with {system}", &[("system", "Groth16")]), "Proving with Groth16");
        assert_eq!(fill("{a}{b} {c}", &[("a", "1"), ("b", "{c}")]), "1{c} {c}");
        assert_eq!(humanise("public-inputs"), "public inputs");
    }
}
