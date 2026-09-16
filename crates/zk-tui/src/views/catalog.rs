//! `/list` and `/about`: what is on the shelves and what one system declares about itself.

use zk_core::catalog::{
    Assumption, Implementation, Mode, ProofSize, Recursion, Shelf, Status, SystemMeta,
    TrustedSetup, ZeroKnowledge,
};
use zk_i18n::Language;

use crate::doc::{Align, Block, Doc, Entry, Hue, Kind, Span, Table, Tone, plain, span};
use crate::museum::Museum;
use crate::phrases::catalog::Msg as C;
use crate::phrases::fill;
use crate::phrases::ui::Msg as U;

/// A full page to open in the pager.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    /// What the pager's top line says.
    pub title: String,
    /// The page source.
    pub markdown: &'static str,
}

/// The systems on every shelf, or on one.
pub fn list(museum: &Museum, shelf: Option<Shelf>, language: Language) -> Entry {
    if museum.systems.is_empty() {
        return no_systems(language);
    }
    let on = |meta: &SystemMeta| shelf.is_none_or(|wanted| meta.shelf == wanted);
    let count = museum.systems.iter().filter(|system| on(system.meta())).count();
    let mut doc = Doc::new();
    doc.line(vec![plain(fill(C::ListTitle.text(language), &[("count", &count.to_string())]))]);
    let shelves = match shelf {
        Some(shelf) => vec![shelf],
        None => Shelf::ALL.to_vec(),
    };
    for current in shelves {
        let metas: Vec<&SystemMeta> = museum
            .systems
            .iter()
            .map(|system| system.meta())
            .filter(|meta| meta.shelf == current)
            .collect();
        doc.blank();
        let name = super::shelf_name(current, language);
        if metas.is_empty() {
            if shelf.is_some() {
                let empty = fill(C::ShelfEmpty.text(language), &[("shelf", name)]);
                doc.line(vec![span(empty, Tone::of(Hue::Secondary))]);
            }
            continue;
        }
        doc.line(vec![span(name, Tone::of(Hue::Shelf).bold())]);
        doc.push(Block::Table(shelf_table(&metas, language)));
    }
    doc.blank();
    doc.line(vec![span(C::ListFooter.text(language), Tone::of(Hue::Secondary))]);
    Entry::new(Kind::Result, doc)
}

/// The first-class empty state: nothing is built in, and that is said plainly.
pub fn no_systems(language: Language) -> Entry {
    let mut doc = Doc::new();
    doc.line(vec![plain(C::NoSystems.text(language))]);
    doc.tree(vec![vec![span(C::NoSystemsDetail.text(language), Tone::of(Hue::Secondary))]]);
    Entry::new(Kind::Result, doc)
}

/// The error for a system ID this binary does not have.
pub fn unknown_system(id: &str, language: Language) -> Entry {
    Entry::text(Kind::Error, fill(U::UnknownSystem.text(language), &[("system", id)]))
}

fn shelf_table(metas: &[&SystemMeta], language: Language) -> Table {
    let header =
        [C::ColId, C::ColName, C::ColYear, C::ColSetup, C::ColZk, C::ColProofSize, C::ColCode]
            .iter()
            .map(|msg| vec![plain(msg.text(language))])
            .collect();
    let rows = metas
        .iter()
        .map(|meta| {
            vec![
                vec![plain(meta.id)],
                vec![plain(meta.name)],
                vec![plain(meta.year.to_string())],
                setup_cell(meta.trusted_setup, language),
                zk_cell(meta.zero_knowledge, language),
                vec![plain(proof_size(meta.proof_size, language))],
                code_cell(&meta.implementation, language),
            ]
        })
        .collect();
    let mut align = vec![Align::Left; 7];
    align[2] = Align::Right;
    Table { header, align, rows, wrap: false, optional: vec![2, 5] }
}

fn setup_cell(setup: TrustedSetup, language: Language) -> Vec<Span> {
    match setup {
        TrustedSetup::TrustedComponent => super::caution(trusted_setup(setup, language)),
        _ => vec![plain(trusted_setup(setup, language))],
    }
}

fn zk_cell(zk: ZeroKnowledge, language: Language) -> Vec<Span> {
    match zk {
        ZeroKnowledge::No => super::caution(zero_knowledge(zk, language)),
        _ => vec![plain(zero_knowledge(zk, language))],
    }
}

fn code_cell(implementation: &Implementation, language: Language) -> Vec<Span> {
    match implementation {
        Implementation::Upstream { .. } => vec![plain(C::CodeUpstream.text(language))],
        Implementation::Teaching { .. } => super::caution(C::CodeTeaching.text(language)),
        Implementation::Companion { .. } => vec![plain(C::CodeCompanion.text(language))],
        Implementation::Homemade => vec![plain(C::CodeHomemade.text(language))],
    }
}

/// What `/about <id>` shows in the transcript, and the page to open when there is one.
pub fn about(museum: &Museum, id: &str, language: Language) -> (Entry, Option<Page>) {
    let markdown = (museum.page)(id, language);
    let Some(system) = museum.system(id) else {
        return match markdown {
            Some(markdown) => {
                let text = fill(C::AboutNotBuilt.text(language), &[("system", id)]);
                (Entry::text(Kind::Result, text), Some(Page { title: id.to_string(), markdown }))
            }
            None => (unknown_system(id, language), None),
        };
    };
    let meta = system.meta();
    let mut doc = summary(meta, language);
    doc.blank();
    let note = match markdown {
        Some(_) => C::AboutPage.text(language).to_string(),
        None => fill(C::AboutNoPage.text(language), &[("system", meta.name)]),
    };
    doc.line(vec![span(note, Tone::of(Hue::Secondary))]);
    let page = markdown.map(|markdown| Page { title: meta.name.to_string(), markdown });
    (Entry::new(Kind::Result, doc), page)
}

fn summary(meta: &SystemMeta, language: Language) -> Doc {
    let secondary = Tone::of(Hue::Secondary);
    let mut doc = Doc::new();
    doc.line(vec![
        span(meta.name, Tone::BODY.bold()),
        plain("  "),
        span(super::shelf_name(meta.shelf, language), Tone::of(Hue::Shelf)),
        span(format!(" · {}", meta.year), secondary),
    ]);
    if !meta.authors.is_empty() {
        doc.line(vec![span(meta.authors.join(", "), secondary)]);
    }
    for line in super::cautions(meta, language) {
        doc.line(line);
    }
    doc.pairs(facts(meta, language));
    doc
}

fn facts(meta: &SystemMeta, language: Language) -> Vec<(String, Vec<Span>)> {
    let label = |msg: C| msg.text(language).to_string();
    let mut pairs = Vec::new();
    if let Some(paper) = &meta.paper {
        let title =
            fill(C::PaperLine.text(language), &[("title", paper.title), ("venue", paper.venue)]);
        pairs.push((
            label(C::LabelPaper),
            vec![plain(title), span(format!(" ({})", paper.url), Tone::of(Hue::Accent))],
        ));
    }
    let assumptions: Vec<&str> =
        meta.assumptions.iter().map(|a| assumption(*a, language)).collect();
    let post_quantum = if meta.post_quantum() { C::Yes } else { C::No };
    pairs.extend([
        (label(C::LabelSetup), setup_cell(meta.trusted_setup, language)),
        (label(C::LabelZk), zk_cell(meta.zero_knowledge, language)),
        (label(C::LabelAssumptions), vec![plain(assumptions.join(", "))]),
        (label(C::LabelPostQuantum), vec![plain(post_quantum.text(language))]),
        (label(C::LabelProofSize), vec![plain(proof_size(meta.proof_size, language))]),
        (label(C::LabelRecursion), vec![plain(recursion(meta.recursion, language))]),
        (label(C::LabelMode), vec![plain(mode(meta.mode, language))]),
        (label(C::LabelField), vec![plain(meta.field)]),
        (label(C::LabelCode), implementation(meta, language)),
        (label(C::LabelStatus), vec![plain(status(meta, language))]),
    ]);
    if !meta.deployments.is_empty() {
        let used: Vec<String> = meta
            .deployments
            .iter()
            .map(|d| match d.until {
                Some(until) => fill(
                    C::DeploymentRange.text(language),
                    &[("project", d.project), ("since", d.since), ("until", until)],
                ),
                None => fill(
                    C::DeploymentSince.text(language),
                    &[("project", d.project), ("since", d.since)],
                ),
            })
            .collect();
        pairs.push((label(C::LabelUsedIn), vec![plain(used.join("; "))]));
    }
    pairs
}

fn implementation(meta: &SystemMeta, language: Language) -> Vec<Span> {
    let text = match meta.implementation {
        Implementation::Upstream { name, version, license, .. } => fill(
            C::ImplUpstream.text(language),
            &[("name", name), ("version", version), ("license", license)],
        ),
        Implementation::Teaching { built_on } => {
            let text = fill(C::ImplTeaching.text(language), &[("crates", &built_on.join(", "))]);
            return super::caution(text);
        }
        Implementation::Companion { upstream, .. } => {
            fill(C::ImplCompanion.text(language), &[("id", meta.id), ("upstream", upstream)])
        }
        Implementation::Homemade => C::ImplHomemade.text(language).to_string(),
    };
    vec![plain(text)]
}

fn status(meta: &SystemMeta, language: Language) -> String {
    let status = match meta.status {
        Status::Active => C::StatusActive.text(language).to_string(),
        Status::Superseded { by, since } => {
            fill(C::StatusSuperseded.text(language), &[("by", by), ("since", since)])
        }
        Status::Discontinued { since } => {
            fill(C::StatusDiscontinued.text(language), &[("since", since)])
        }
        Status::Experimental => C::StatusExperimental.text(language).to_string(),
        Status::Historical => C::StatusHistorical.text(language).to_string(),
    };
    fill(C::StatusAsOf.text(language), &[("status", &status), ("date", meta.status_as_of)])
}

fn trusted_setup(setup: TrustedSetup, language: Language) -> &'static str {
    match setup {
        TrustedSetup::None => C::SetupNone,
        TrustedSetup::Universal => C::SetupUniversal,
        TrustedSetup::PerCircuit => C::SetupPerCircuit,
        TrustedSetup::TrustedComponent => C::SetupTrusted,
    }
    .text(language)
}

fn zero_knowledge(zk: ZeroKnowledge, language: Language) -> &'static str {
    match zk {
        ZeroKnowledge::Yes => C::ZkYes,
        ZeroKnowledge::No => C::ZkNo,
        ZeroKnowledge::Optional => C::ZkOptional,
    }
    .text(language)
}

fn assumption(assumption: Assumption, language: Language) -> &'static str {
    match assumption {
        Assumption::Pairing => C::AssumePairing,
        Assumption::DiscreteLog => C::AssumeDiscreteLog,
        Assumption::Hash => C::AssumeHash,
        Assumption::Lattice => C::AssumeLattice,
        Assumption::TrustedComponent => C::AssumeTrusted,
    }
    .text(language)
}

fn proof_size(size: ProofSize, language: Language) -> &'static str {
    match size {
        ProofSize::Constant => C::SizeConstant,
        ProofSize::Logarithmic => C::SizeLog,
        ProofSize::Polylogarithmic => C::SizePolylog,
        ProofSize::SquareRoot => C::SizeSqrt,
        ProofSize::Linear => C::SizeLinear,
    }
    .text(language)
}

fn recursion(recursion: Recursion, language: Language) -> &'static str {
    match recursion {
        Recursion::None => C::RecNone,
        Recursion::Recursion => C::RecRecursion,
        Recursion::Accumulation => C::RecAccumulation,
        Recursion::Folding => C::RecFolding,
    }
    .text(language)
}

fn mode(mode: Mode, language: Language) -> &'static str {
    match mode {
        Mode::NonInteractive => C::ModeNonInteractive,
        Mode::Interactive => C::ModeInteractive,
    }
    .text(language)
}
