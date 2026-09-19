use std::ops::Range;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Document {
    pub front_matter: Option<FrontMatter>,
    pub blocks: Vec<Block>,
    pub footnotes: Vec<Footnote>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatter {
    pub format: MetaFormat,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaFormat {
    Yaml,
    Toml,
}

impl FrontMatter {
    pub fn language(&self) -> &'static str {
        match self.format {
            MetaFormat::Yaml => "yaml",
            MetaFormat::Toml => "toml",
        }
    }

    pub fn keys(&self) -> Vec<String> {
        let sep = match self.format {
            MetaFormat::Yaml => ':',
            MetaFormat::Toml => '=',
        };
        self.text
            .lines()
            .filter(|line| !line.starts_with([' ', '\t', '#', '-', '[']))
            .filter_map(|line| line.split_once(sep))
            .map(|(key, _)| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    BlockQuote {
        alert: Option<Alert>,
        blocks: Vec<Block>,
    },
    List(List),
    DefinitionList(Vec<Definition>),
    Table(Table),
    Rule,
    Html(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl Alert {
    pub fn label(self) -> &'static str {
        match self {
            Alert::Note => "Note",
            Alert::Tip => "Tip",
            Alert::Important => "Important",
            Alert::Warning => "Warning",
            Alert::Caution => "Caution",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Alert::Note => "●",
            Alert::Tip => "✦",
            Alert::Important => "◆",
            Alert::Warning => "▲",
            Alert::Caution => "■",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub term: Vec<Inline>,
    pub details: Vec<Vec<Block>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub start: Option<u64>,
    pub tight: bool,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub alignments: Vec<Alignment>,
    pub header: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Footnote {
    pub label: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Math {
        display: bool,
        source: String,
    },
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        content: Vec<Inline>,
        url: String,
        title: String,
    },
    WikiLink {
        target: String,
        content: Vec<Inline>,
    },
    Image {
        alt: Vec<Inline>,
        url: String,
    },
    FootnoteRef(String),
    Html(String),
    SoftBreak,
    HardBreak,
}

pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_plain(inlines, &mut out);
    out
}

fn push_plain(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(s) | Inline::Code(s) | Inline::Html(s) => out.push_str(s),
            Inline::Math { source, .. } => out.push_str(source),
            Inline::Emphasis(c) | Inline::Strong(c) | Inline::Strikethrough(c) => {
                push_plain(c, out)
            }
            Inline::Link { content, .. } | Inline::WikiLink { content, .. } => {
                push_plain(content, out)
            }
            Inline::Image { alt, .. } => push_plain(alt, out),
            Inline::FootnoteRef(label) => {
                out.push('[');
                out.push_str(label);
                out.push(']');
            }
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
        }
    }
}
