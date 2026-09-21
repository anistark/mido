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
        self.fields().into_iter().map(|(key, _)| key).collect()
    }

    pub fn fields(&self) -> Vec<(String, String)> {
        let lines: Vec<&str> = self.text.lines().collect();
        let mut out = Vec::new();
        match self.format {
            MetaFormat::Yaml => yaml_fields(&lines, 0, "", &mut out),
            MetaFormat::Toml => toml_fields(&lines, &mut out),
        }
        out
    }
}

fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

fn yaml_pair(body: &str) -> Option<(&str, &str)> {
    if body.starts_with("- ") || body == "-" {
        return None;
    }
    let mut at = 0;
    for (i, c) in body.char_indices() {
        if c == ':' && body[i + 1..].chars().next().is_none_or(|n| n == ' ') {
            at = i;
            break;
        }
    }
    let (key, rest) = body.split_at(at);
    let rest = rest.strip_prefix(':')?;
    let key = key.trim();
    (!key.is_empty()).then_some((key, rest))
}

fn yaml_fields(lines: &[&str], indent: usize, prefix: &str, out: &mut Vec<(String, String)>) {
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let body = line.trim();
        if body.is_empty() || body.starts_with('#') || indent_of(line) != indent {
            i += 1;
            continue;
        }
        let Some((key, rest)) = yaml_pair(body) else {
            i += 1;
            continue;
        };
        let key = format!("{prefix}{}", unquote(key));
        let mut end = i + 1;
        while end < lines.len() {
            let next = lines[end];
            if !next.trim().is_empty() && indent_of(next) <= indent {
                break;
            }
            end += 1;
        }
        let children = &lines[i + 1..end];
        let value = rest.trim();
        let block_scalar = value.starts_with('|') || value.starts_with('>');
        if !value.is_empty() && !block_scalar {
            out.push((key, scalar(value)));
        } else if block_scalar {
            let text: Vec<&str> = children
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            out.push((key, text.join(" ")));
        } else {
            let child_indent = children
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| indent_of(l))
                .min();
            match child_indent {
                None => out.push((key, String::new())),
                Some(child_indent) => {
                    let direct: Vec<&str> = children
                        .iter()
                        .filter(|l| !l.trim().is_empty() && indent_of(l) == child_indent)
                        .map(|l| l.trim())
                        .collect();
                    if direct.iter().all(|l| l.starts_with("- ") || *l == "-") {
                        let items: Vec<String> = direct
                            .iter()
                            .map(|l| scalar(l.trim_start_matches('-').trim()))
                            .filter(|l| !l.is_empty())
                            .collect();
                        out.push((key, items.join(", ")));
                    } else {
                        yaml_fields(children, child_indent, &format!("{key}."), out);
                    }
                }
            }
        }
        i = end;
    }
}

fn toml_fields(lines: &[&str], out: &mut Vec<(String, String)>) {
    let mut prefix = String::new();
    let mut i = 0;
    while i < lines.len() {
        let body = lines[i].trim();
        i += 1;
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        if body.starts_with('[') && body.ends_with(']') {
            let name = body.trim_matches(['[', ']']).trim();
            prefix = if name.is_empty() {
                String::new()
            } else {
                format!("{name}.")
            };
            continue;
        }
        let Some((key, value)) = body.split_once('=') else {
            continue;
        };
        let mut value = value.trim().to_string();
        while value.starts_with('[') && !value.ends_with(']') && i < lines.len() {
            value.push(' ');
            value.push_str(lines[i].trim());
            i += 1;
        }
        out.push((format!("{prefix}{}", unquote(key)), scalar(&value)));
    }
}

fn scalar(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('[') && value.ends_with(']') {
        return value[1..value.len() - 1]
            .split(',')
            .map(unquote)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }
    unquote(value)
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    let quoted = value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')));
    if quoted {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(format: MetaFormat, text: &str) -> Vec<(String, String)> {
        FrontMatter {
            format,
            text: text.to_string(),
        }
        .fields()
    }

    fn pairs(fields: &[(String, String)]) -> Vec<(&str, &str)> {
        fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    #[test]
    fn yaml_scalars_lists_and_nested_maps() {
        let text = "title: \"Rich content\"\n# a comment\ndraft: false\ntags:\n  - demo\n  - 'two words'\nlangs: [rust, \"toml\"]\nauthor:\n  name: Ani\n  links:\n    - a\n    - b\nsummary: >\n  folded over\n  two lines\nempty:\n";
        let got = fields(MetaFormat::Yaml, text);
        assert_eq!(
            pairs(&got),
            [
                ("title", "Rich content"),
                ("draft", "false"),
                ("tags", "demo, two words"),
                ("langs", "rust, toml"),
                ("author.name", "Ani"),
                ("author.links", "a, b"),
                ("summary", "folded over two lines"),
                ("empty", ""),
            ]
        );
    }

    #[test]
    fn yaml_urls_and_list_items_keep_their_colons() {
        let text = "url: https://example.com:8080/x\nsteps:\n  - name: build\n    run: cargo build\n  - name: test\n";
        let got = fields(MetaFormat::Yaml, text);
        assert_eq!(
            pairs(&got),
            [
                ("url", "https://example.com:8080/x"),
                ("steps", "name: build, name: test"),
            ]
        );
    }

    #[test]
    fn toml_tables_and_arrays() {
        let text = "title = \"Docs\"\norder = 1\ntags = [\n  \"a\",\n  \"b\",\n]\n\n[extra]\ntheme = 'dark'\n";
        let got = fields(MetaFormat::Toml, text);
        assert_eq!(
            pairs(&got),
            [
                ("title", "Docs"),
                ("order", "1"),
                ("tags", "a, b"),
                ("extra.theme", "dark"),
            ]
        );
    }
}
