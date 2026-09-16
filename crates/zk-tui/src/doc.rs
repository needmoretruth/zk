//! What a cell or a page says, before anyone decides how wide the screen is.
//!
//! Views build a [`Doc`] out of styled spans, tree lines and tables; the terminal screen and the
//! plain-text printer both lay the same `Doc` out at their own width. That is how `nmtzk run` and
//! the TUI's `/run` show the same content, and how a finished cell rewraps when the window resizes.

/// A colour role. The look turns it into one of the 16 named ANSI colours, or into no colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Hue {
    /// The terminal's own foreground.
    #[default]
    Body,
    /// Cyan: the prompt glyph, selection and links.
    Accent,
    /// Green: something held or verified.
    Success,
    /// Red: something failed or was broken.
    Failure,
    /// Yellow: not zero-knowledge, teaching implementation, trusted component.
    Caution,
    /// Magenta: shelf names.
    Shelf,
    /// Dark gray: secondary information.
    Secondary,
}

/// Colour role plus the few text attributes a terminal reliably draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tone {
    /// Colour role.
    pub hue: Hue,
    /// Bold weight.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
}

impl Tone {
    /// Default foreground, no attributes.
    pub const BODY: Tone = Tone { hue: Hue::Body, bold: false, italic: false, underline: false };

    /// A plain tone of one colour role.
    pub const fn of(hue: Hue) -> Tone {
        Tone { hue, ..Tone::BODY }
    }

    /// The same tone in bold.
    pub const fn bold(self) -> Tone {
        Tone { bold: true, ..self }
    }

    /// The same tone in italics.
    pub const fn italic(self) -> Tone {
        Tone { italic: true, ..self }
    }

    /// The same tone underlined.
    pub const fn underline(self) -> Tone {
        Tone { underline: true, ..self }
    }
}

/// A run of text in one tone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The text; `\n` forces a new row.
    pub text: String,
    /// How it is drawn.
    pub tone: Tone,
}

/// A span in `tone`.
pub fn span(text: impl Into<String>, tone: Tone) -> Span {
    Span { text: text.into(), tone }
}

/// A span in the default tone.
pub fn plain(text: impl Into<String>) -> Span {
    span(text, Tone::BODY)
}

/// How a table column lines up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// Text.
    Left,
    /// Numbers.
    Right,
}

/// A borderless table: two spaces between columns, a bold header row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// Header cells; empty for a table without a header row.
    pub header: Vec<Vec<Span>>,
    /// Alignment per column; missing entries are [`Align::Left`].
    pub align: Vec<Align>,
    /// Body rows of cells.
    pub rows: Vec<Vec<Vec<Span>>>,
    /// Whether a too-wide cell wraps onto more rows (prose) or is cut with an ellipsis (data).
    pub wrap: bool,
    /// Columns that may be left out when the table is too wide, in the order they go.
    pub optional: Vec<usize>,
}

/// One vertical piece of a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Wrapped text. `lead` starts the first row; later rows start with `hang`, or with spaces as
    /// wide as `lead` when `hang` is `None`, so tree and list items keep their indentation.
    Text {
        /// First-row prefix.
        lead: Vec<Span>,
        /// Continuation-row prefix.
        hang: Option<Vec<Span>>,
        /// The text that wraps.
        body: Vec<Span>,
    },
    /// Rows drawn as given and cut at the edge, never wrapped: code and drawings.
    Pre(Vec<Vec<Span>>),
    /// A table.
    Table(Table),
    /// Rows inside a rounded box sized to its content.
    Boxed(Vec<Vec<Span>>),
    /// A horizontal rule across the width.
    Rule,
    /// An empty row.
    Blank,
}

/// A width-independent document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doc {
    /// Blocks from top to bottom.
    pub blocks: Vec<Block>,
}

impl Doc {
    /// An empty document.
    pub fn new() -> Doc {
        Doc::default()
    }

    /// Adds a block.
    pub fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Adds a wrapped line of text.
    pub fn line(&mut self, body: Vec<Span>) {
        self.push(Block::Text { lead: Vec::new(), hang: None, body });
    }

    /// Adds a wrapped line whose continuation rows are indented under `lead`.
    pub fn led(&mut self, lead: Vec<Span>, body: Vec<Span>) {
        self.push(Block::Text { lead, hang: None, body });
    }

    /// Adds tree lines: `├` before every item but the last, `└` before the last.
    pub fn tree(&mut self, items: Vec<Vec<Span>>) {
        let last = items.len().saturating_sub(1);
        for (index, body) in items.into_iter().enumerate() {
            let glyph = if index == last { "└ " } else { "├ " };
            self.led(vec![span(glyph, Tone::of(Hue::Secondary))], body);
        }
    }

    /// Adds tree lines of `label  value`, with labels padded to one column.
    pub fn pairs(&mut self, pairs: Vec<(String, Vec<Span>)>) {
        let wide = pairs.iter().map(|(label, _)| crate::text::width(label)).max().unwrap_or(0);
        let items = pairs
            .into_iter()
            .map(|(label, mut value)| {
                let pad = wide - crate::text::width(&label) + 2;
                let mut item = vec![plain(format!("{label}{}", " ".repeat(pad)))];
                item.append(&mut value);
                item
            })
            .collect();
        self.tree(items);
    }

    /// Adds an empty row.
    pub fn blank(&mut self) {
        self.push(Block::Blank);
    }

    /// Adds every block of `other`.
    pub fn append(&mut self, other: Doc) {
        self.blocks.extend(other.blocks);
    }
}

/// What kind of cell a document is shown in; the kind decides the head glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The start screen box; no head glyph.
    Welcome,
    /// What the user typed, after `›`.
    Command,
    /// A result, after `•`.
    Result,
    /// Work in progress, after a spinner; becomes a result when it ends.
    Running,
    /// A caution, after `⚠`.
    Warning,
    /// A failure, after `✗`.
    Error,
    /// Lines that arrive one beat at a time, after `•`.
    Story,
}

/// A document together with the kind of cell it belongs in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Cell kind.
    pub kind: Kind,
    /// Content.
    pub doc: Doc,
}

impl Entry {
    /// An entry of `kind` holding `doc`.
    pub fn new(kind: Kind, doc: Doc) -> Entry {
        Entry { kind, doc }
    }

    /// A one-line entry of `kind`.
    pub fn text(kind: Kind, text: impl Into<String>) -> Entry {
        let mut doc = Doc::new();
        doc.line(vec![plain(text)]);
        Entry { kind, doc }
    }
}
