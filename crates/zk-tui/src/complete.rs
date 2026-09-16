//! What the popup offers for the line being typed: command names first, then each argument.

use zk_core::ExampleId;
use zk_core::catalog::Shelf;
use zk_i18n::Language;

use crate::commands::{Arg, SPECS};
use crate::museum::Museum;
use crate::phrases::ui::Msg;

/// One row of the popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// Left column.
    pub(crate) label: String,
    /// Right column, drawn dimmed.
    pub(crate) detail: String,
    /// The whole line after choosing this row.
    pub(crate) line: String,
}

/// Candidates for `input`, best first; empty when the line is not a single-line slash command.
pub(crate) fn candidates(input: &str, museum: &Museum, language: Language) -> Vec<Candidate> {
    if !input.starts_with('/') || input.contains('\n') {
        return Vec::new();
    }
    let words: Vec<&str> = input.split_whitespace().collect();
    let open = input.ends_with(char::is_whitespace);
    if words.len() == 1 && !open {
        return commands(words[0], language);
    }
    let Some(spec) = SPECS.iter().find(|spec| Some(&spec.name) == words.first()) else {
        return Vec::new();
    };
    let (done, partial) = if open {
        (words.len() - 1, "")
    } else {
        (words.len() - 2, words.last().copied().unwrap_or_default())
    };
    let Some((arg, _)) = spec.args.get(done) else { return Vec::new() };
    let prefix = words[..=done].join(" ");
    let more = done + 1 < spec.args.len();
    let mut ranked: Vec<(u8, Candidate)> = options(*arg, museum, language)
        .into_iter()
        .filter_map(|(value, detail)| {
            let rank = rank(partial, &value)?;
            let line = format!("{prefix} {value}{}", if more { " " } else { "" });
            Some((rank, Candidate { label: value, detail, line }))
        })
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, candidate)| candidate).collect()
}

fn commands(typed: &str, language: Language) -> Vec<Candidate> {
    let query = typed.trim_start_matches('/');
    let mut ranked: Vec<(u8, Candidate)> = SPECS
        .iter()
        .filter_map(|spec| {
            let rank = rank(query, spec.name.trim_start_matches('/'))?;
            let line = format!("{}{}", spec.name, if spec.args.is_empty() { "" } else { " " });
            let detail = spec.summary.text(language).to_string();
            Some((rank, Candidate { label: spec.usage.to_string(), detail, line }))
        })
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, candidate)| candidate).collect()
}

/// `(value, description)` pairs an argument position accepts.
fn options(arg: Arg, museum: &Museum, language: Language) -> Vec<(String, String)> {
    let systems = || {
        museum.systems.iter().map(|system| {
            let meta = system.meta();
            (
                meta.id.to_string(),
                format!("{} · {}", meta.name, crate::views::shelf_name(meta.shelf, language)),
            )
        })
    };
    match arg {
        Arg::System => systems().collect(),
        Arg::SystemOrAll => {
            let mut all = vec![("all".to_string(), Msg::ArgAll.text(language).to_string())];
            all.extend(systems());
            all
        }
        Arg::Example => ExampleId::ALL
            .iter()
            .map(|example| {
                (example.id().to_string(), crate::views::statement(*example, language).to_string())
            })
            .collect(),
        Arg::Shelf => Shelf::ALL
            .iter()
            .map(|shelf| {
                (shelf.key().to_string(), crate::views::shelf_name(*shelf, language).to_string())
            })
            .collect(),
        Arg::Language => Language::ALL
            .iter()
            .map(|language| (language.code().to_string(), language.endonym().to_string()))
            .collect(),
    }
}

/// 0 for a prefix match, 1 for a subsequence match, `None` for no match. Case is ignored.
fn rank(query: &str, candidate: &str) -> Option<u8> {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    if candidate.starts_with(&query) {
        return Some(0);
    }
    let mut rest = candidate.chars();
    query.chars().all(|wanted| rest.any(|have| have == wanted)).then_some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &str) -> Vec<String> {
        candidates(input, &Museum::empty(), Language::ENGLISH).into_iter().map(|c| c.line).collect()
    }

    #[test]
    fn a_bare_slash_offers_every_command_in_order() {
        let all = lines("/");
        assert_eq!(all.len(), SPECS.len());
        assert_eq!(all[0], "/help");
        assert_eq!(all[4], "/run ");
    }

    #[test]
    fn typing_filters_by_subsequence_with_prefixes_first() {
        assert_eq!(lines("/ex"), ["/examples"]);
        let l = lines("/l");
        assert_eq!(&l[..2], ["/list ", "/lang "]);
        assert!(l.contains(&"/help".to_string()), "subsequence matches follow: {l:?}");
    }

    #[test]
    fn arguments_complete_in_their_position() {
        assert_eq!(lines("/run ")[0], "/run all ");
        assert_eq!(lines("/run all one"), ["/run all one-plus-one"]);
        assert_eq!(lines("/lang k"), ["/lang ko"]);
        assert_eq!(lines("/list zc"), ["/list zcash", "/list aztec"]);
        assert_eq!(lines("/list zca"), ["/list zcash"]);
        assert!(lines("/about ").is_empty(), "an empty museum has no systems to offer");
        assert!(lines("/help ").is_empty());
        assert!(lines("hello").is_empty());
    }
}
