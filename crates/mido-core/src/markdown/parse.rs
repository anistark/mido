use std::ops::Range;

use pulldown_cmark::{
    Alignment as PAlignment, BlockQuoteKind, CodeBlockKind, Event, LinkType, MetadataBlockKind,
    Options, Parser, Tag,
};

use super::document::*;

pub fn parse(text: &str) -> Document {
    let mut builder = Builder::new();
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        builder.event(event, range);
    }
    builder.finish()
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_WIKILINKS
}

enum Node {
    Root(Vec<Block>),
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    BlockQuote {
        alert: Option<Alert>,
        blocks: Vec<Block>,
    },
    List(List),
    Item {
        task: Option<bool>,
        blocks: Vec<Block>,
        pending: Vec<Inline>,
    },
    DefList(Vec<Definition>),
    DefTitle(Vec<Inline>),
    DefDetail {
        blocks: Vec<Block>,
        pending: Vec<Inline>,
    },
    Meta {
        format: MetaFormat,
        text: String,
    },
    Footnote {
        label: String,
        blocks: Vec<Block>,
    },
    Html(String),
    Table(Table),
    TableHead(Vec<Vec<Inline>>),
    TableRow(Vec<Vec<Inline>>),
    TableCell(Vec<Inline>),
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
    Transparent,
}

struct Frame {
    node: Node,
    start: usize,
}

struct Builder {
    stack: Vec<Frame>,
    footnotes: Vec<Footnote>,
    front_matter: Option<FrontMatter>,
}

impl Builder {
    fn new() -> Self {
        Self {
            stack: vec![Frame {
                node: Node::Root(Vec::new()),
                start: 0,
            }],
            footnotes: Vec::new(),
            front_matter: None,
        }
    }

    fn finish(mut self) -> Document {
        while self.stack.len() > 1 {
            self.end_frame(usize::MAX);
        }
        let blocks = match self.stack.pop().map(|f| f.node) {
            Some(Node::Root(blocks)) => blocks,
            _ => Vec::new(),
        };
        Document {
            front_matter: self.front_matter,
            blocks,
            footnotes: self.footnotes,
        }
    }

    fn event(&mut self, event: Event, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(tag, range.start),
            Event::End(_) => self.end_frame(range.end),
            Event::Text(s) => self.text(&s),
            Event::Code(s) => self.inline(Inline::Code(s.into_string())),
            Event::InlineMath(s) => self.inline(Inline::Math {
                display: false,
                source: s.into_string(),
            }),
            Event::DisplayMath(s) => self.inline(Inline::Math {
                display: true,
                source: s.into_string(),
            }),
            Event::Html(s) => self.html(&s),
            Event::InlineHtml(s) => self.inline(Inline::Html(s.into_string())),
            Event::FootnoteReference(s) => self.inline(Inline::FootnoteRef(s.into_string())),
            Event::SoftBreak => self.inline(Inline::SoftBreak),
            Event::HardBreak => self.inline(Inline::HardBreak),
            Event::Rule => self.block(BlockKind::Rule, range),
            Event::TaskListMarker(checked) => {
                for frame in self.stack.iter_mut().rev() {
                    if let Node::Item { task, .. } = &mut frame.node {
                        *task = Some(checked);
                        break;
                    }
                }
            }
        }
    }

    fn top(&mut self) -> Option<&mut Node> {
        self.stack.last_mut().map(|f| &mut f.node)
    }

    fn push(&mut self, node: Node, start: usize) {
        self.stack.push(Frame { node, start });
    }

    fn start(&mut self, tag: Tag, pos: usize) {
        let node = match tag {
            Tag::Paragraph => {
                self.flush_item(pos);
                self.mark_loose();
                Node::Paragraph(Vec::new())
            }
            Tag::Heading { level, .. } => Node::Heading {
                level: level as u8,
                content: Vec::new(),
            },
            Tag::BlockQuote(kind) => {
                self.flush_item(pos);
                Node::BlockQuote {
                    alert: kind.map(alert),
                    blocks: Vec::new(),
                }
            }
            Tag::CodeBlock(kind) => {
                self.flush_item(pos);
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => language(&info),
                    CodeBlockKind::Indented => None,
                };
                Node::CodeBlock {
                    lang,
                    code: String::new(),
                }
            }
            Tag::HtmlBlock => {
                self.flush_item(pos);
                Node::Html(String::new())
            }
            Tag::List(start) => {
                self.flush_item(pos);
                Node::List(List {
                    start,
                    tight: true,
                    items: Vec::new(),
                })
            }
            Tag::Item => Node::Item {
                task: None,
                blocks: Vec::new(),
                pending: Vec::new(),
            },
            Tag::FootnoteDefinition(label) => Node::Footnote {
                label: label.into_string(),
                blocks: Vec::new(),
            },
            Tag::Table(alignments) => {
                self.flush_item(pos);
                let alignments = alignments.into_iter().map(alignment).collect();
                Node::Table(Table {
                    alignments,
                    header: Vec::new(),
                    rows: Vec::new(),
                })
            }
            Tag::TableHead => Node::TableHead(Vec::new()),
            Tag::TableRow => Node::TableRow(Vec::new()),
            Tag::TableCell => Node::TableCell(Vec::new()),
            Tag::Emphasis => Node::Emphasis(Vec::new()),
            Tag::Strong => Node::Strong(Vec::new()),
            Tag::Strikethrough => Node::Strikethrough(Vec::new()),
            Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            } => Node::WikiLink {
                target: dest_url.into_string(),
                content: Vec::new(),
            },
            Tag::Link {
                dest_url, title, ..
            } => Node::Link {
                content: Vec::new(),
                url: dest_url.into_string(),
                title: title.into_string(),
            },
            Tag::Image { dest_url, .. } => Node::Image {
                alt: Vec::new(),
                url: dest_url.into_string(),
            },
            Tag::MetadataBlock(kind) => Node::Meta {
                format: match kind {
                    MetadataBlockKind::YamlStyle => MetaFormat::Yaml,
                    MetadataBlockKind::PlusesStyle => MetaFormat::Toml,
                },
                text: String::new(),
            },
            Tag::DefinitionList => {
                self.flush_item(pos);
                Node::DefList(Vec::new())
            }
            Tag::DefinitionListTitle => Node::DefTitle(Vec::new()),
            Tag::DefinitionListDefinition => Node::DefDetail {
                blocks: Vec::new(),
                pending: Vec::new(),
            },
            Tag::Superscript | Tag::Subscript => Node::Transparent,
        };
        self.push(node, pos);
    }

    fn end_frame(&mut self, pos: usize) {
        if self.stack.len() < 2 {
            return;
        }
        let Frame { node, start } = self.stack.pop().unwrap();
        let span = start..pos;
        match node {
            Node::Root(_) => unreachable!(),
            Node::Paragraph(content) => self.block(BlockKind::Paragraph(content), span),
            Node::Heading { level, content } => {
                self.block(BlockKind::Heading { level, content }, span)
            }
            Node::CodeBlock { lang, mut code } => {
                if code.ends_with('\n') {
                    code.pop();
                }
                self.block(BlockKind::CodeBlock { lang, code }, span);
            }
            Node::BlockQuote { alert, blocks } => {
                self.block(BlockKind::BlockQuote { alert, blocks }, span)
            }
            Node::List(list) => self.block(BlockKind::List(list), span),
            Node::DefList(definitions) => self.block(BlockKind::DefinitionList(definitions), span),
            Node::DefTitle(term) => {
                if let Some(Node::DefList(definitions)) = self.top() {
                    definitions.push(Definition {
                        term,
                        details: Vec::new(),
                    });
                }
            }
            Node::DefDetail {
                mut blocks,
                pending,
            } => {
                if !pending.is_empty() {
                    blocks.push(Block {
                        kind: BlockKind::Paragraph(pending),
                        span: span.clone(),
                    });
                }
                if let Some(Node::DefList(definitions)) = self.top()
                    && let Some(last) = definitions.last_mut()
                {
                    last.details.push(blocks);
                }
            }
            Node::Meta { format, mut text } => {
                while text.ends_with('\n') {
                    text.pop();
                }
                self.front_matter = Some(FrontMatter { format, text });
            }
            Node::Item {
                task,
                mut blocks,
                pending,
            } => {
                if !pending.is_empty() {
                    blocks.push(Block {
                        kind: BlockKind::Paragraph(pending),
                        span: span.clone(),
                    });
                }
                if let Some(Node::List(list)) = self.top() {
                    list.items.push(ListItem { task, blocks });
                }
            }
            Node::Footnote { label, blocks } => self.footnotes.push(Footnote { label, blocks }),
            Node::Html(mut html) => {
                while html.ends_with('\n') {
                    html.pop();
                }
                self.block(BlockKind::Html(html), span);
            }
            Node::Table(table) => self.block(BlockKind::Table(table), span),
            Node::TableHead(cells) => {
                if let Some(Node::Table(table)) = self.top() {
                    table.header = cells;
                }
            }
            Node::TableRow(cells) => {
                if let Some(Node::Table(table)) = self.top() {
                    table.rows.push(cells);
                }
            }
            Node::TableCell(content) => {
                if let Some(Node::TableHead(cells) | Node::TableRow(cells)) = self.top() {
                    cells.push(content);
                }
            }
            Node::Emphasis(content) => self.inline(Inline::Emphasis(content)),
            Node::Strong(content) => self.inline(Inline::Strong(content)),
            Node::Strikethrough(content) => self.inline(Inline::Strikethrough(content)),
            Node::Link {
                content,
                url,
                title,
            } => self.inline(Inline::Link {
                content,
                url,
                title,
            }),
            Node::WikiLink { target, content } => self.inline(Inline::WikiLink { target, content }),
            Node::Image { alt, url } => self.inline(Inline::Image { alt, url }),
            Node::Transparent => {}
        }
    }

    fn text(&mut self, s: &str) {
        match self.top() {
            Some(Node::CodeBlock { code, .. }) => code.push_str(s),
            Some(Node::Html(html)) => html.push_str(s),
            Some(Node::Meta { text, .. }) => text.push_str(s),
            _ => self.inline(Inline::Text(emojify(s))),
        }
    }

    fn html(&mut self, s: &str) {
        match self.top() {
            Some(Node::Html(html)) => html.push_str(s),
            Some(Node::Meta { text, .. }) => text.push_str(s),
            _ => self.inline(Inline::Html(s.to_string())),
        }
    }

    fn inline(&mut self, inline: Inline) {
        for frame in self.stack.iter_mut().rev() {
            let target = match &mut frame.node {
                Node::Paragraph(c)
                | Node::Heading { content: c, .. }
                | Node::TableCell(c)
                | Node::Emphasis(c)
                | Node::Strong(c)
                | Node::Strikethrough(c)
                | Node::Link { content: c, .. }
                | Node::WikiLink { content: c, .. }
                | Node::DefTitle(c)
                | Node::Image { alt: c, .. } => c,
                Node::Item { pending, .. } | Node::DefDetail { pending, .. } => pending,
                Node::Transparent => continue,
                _ => return,
            };
            if let (Inline::Text(new), Some(Inline::Text(last))) = (&inline, target.last_mut()) {
                last.push_str(new);
            } else {
                target.push(inline);
            }
            return;
        }
    }

    fn block(&mut self, kind: BlockKind, span: Range<usize>) {
        let block = Block { kind, span };
        for frame in self.stack.iter_mut().rev() {
            match &mut frame.node {
                Node::Root(blocks)
                | Node::BlockQuote { blocks, .. }
                | Node::Footnote { blocks, .. } => {
                    blocks.push(block);
                    return;
                }
                Node::Item {
                    blocks, pending, ..
                }
                | Node::DefDetail { blocks, pending } => {
                    if !pending.is_empty() {
                        let span = frame.start..block.span.start;
                        blocks.push(Block {
                            kind: BlockKind::Paragraph(std::mem::take(pending)),
                            span,
                        });
                    }
                    blocks.push(block);
                    return;
                }
                _ => continue,
            }
        }
    }

    fn flush_item(&mut self, pos: usize) {
        if let Some(Frame {
            node:
                Node::Item {
                    blocks, pending, ..
                }
                | Node::DefDetail { blocks, pending },
            start,
        }) = self.stack.last_mut()
            && !pending.is_empty()
        {
            let span = *start..pos;
            blocks.push(Block {
                kind: BlockKind::Paragraph(std::mem::take(pending)),
                span,
            });
        }
    }

    fn mark_loose(&mut self) {
        let n = self.stack.len();
        if n < 2 || !matches!(self.stack[n - 1].node, Node::Item { .. }) {
            return;
        }
        if let Node::List(list) = &mut self.stack[n - 2].node {
            list.tight = false;
        }
    }
}

fn alert(kind: BlockQuoteKind) -> Alert {
    match kind {
        BlockQuoteKind::Note => Alert::Note,
        BlockQuoteKind::Tip => Alert::Tip,
        BlockQuoteKind::Important => Alert::Important,
        BlockQuoteKind::Warning => Alert::Warning,
        BlockQuoteKind::Caution => Alert::Caution,
    }
}

fn emojify(text: &str) -> String {
    if !text.contains(':') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find(':') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-')))
            .unwrap_or(after.len());
        match after[end..]
            .starts_with(':')
            .then(|| emojis::get_by_shortcode(&after[..end]))
        {
            Some(Some(emoji)) if end > 0 => {
                out.push_str(emoji.as_str());
                rest = &after[end + 1..];
            }
            _ => {
                out.push(':');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn language(info: &str) -> Option<String> {
    let lang: String = info
        .trim_start()
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != ',' && *c != '{')
        .collect();
    (!lang.is_empty()).then_some(lang)
}

fn alignment(a: PAlignment) -> Alignment {
    match a {
        PAlignment::None | PAlignment::Left => Alignment::Left,
        PAlignment::Center => Alignment::Center,
        PAlignment::Right => Alignment::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Inline {
        Inline::Text(s.to_string())
    }

    #[test]
    fn headings_and_paragraphs_keep_spans() {
        let doc = parse("# Title\n\nHello *world*.\n");
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].span, 0..8);
        assert_eq!(
            doc.blocks[0].kind,
            BlockKind::Heading {
                level: 1,
                content: vec![text("Title")]
            }
        );
        assert_eq!(doc.blocks[1].span, 9..24);
        assert_eq!(
            doc.blocks[1].kind,
            BlockKind::Paragraph(vec![
                text("Hello "),
                Inline::Emphasis(vec![text("world")]),
                text(".")
            ])
        );
    }

    #[test]
    fn tight_list_gets_implicit_paragraphs() {
        let doc = parse("- one\n- two\n  - nested\n");
        let BlockKind::List(list) = &doc.blocks[0].kind else {
            panic!()
        };
        assert!(list.tight);
        assert_eq!(list.items.len(), 2);
        assert_eq!(
            list.items[0].blocks[0].kind,
            BlockKind::Paragraph(vec![text("one")])
        );
        assert_eq!(list.items[1].blocks.len(), 2);
        assert!(matches!(list.items[1].blocks[1].kind, BlockKind::List(_)));
    }

    #[test]
    fn loose_list_and_tasks() {
        let doc = parse("1. [x] done\n\n2. [ ] todo\n");
        let BlockKind::List(list) = &doc.blocks[0].kind else {
            panic!()
        };
        assert!(!list.tight);
        assert_eq!(list.start, Some(1));
        assert_eq!(list.items[0].task, Some(true));
        assert_eq!(list.items[1].task, Some(false));
    }

    #[test]
    fn table_with_alignments() {
        let doc = parse("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n");
        let BlockKind::Table(table) = &doc.blocks[0].kind else {
            panic!()
        };
        assert_eq!(
            table.alignments,
            vec![Alignment::Left, Alignment::Center, Alignment::Right]
        );
        assert_eq!(
            table.header,
            vec![vec![text("a")], vec![text("b")], vec![text("c")]]
        );
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][2], vec![text("3")]);
    }

    #[test]
    fn code_block_language_and_trailing_newline() {
        let doc = parse("```rust,ignore\nfn main() {}\n```\n");
        assert_eq!(
            doc.blocks[0].kind,
            BlockKind::CodeBlock {
                lang: Some("rust".into()),
                code: "fn main() {}".into()
            }
        );
        let doc = parse("    indented\n");
        assert_eq!(
            doc.blocks[0].kind,
            BlockKind::CodeBlock {
                lang: None,
                code: "indented".into()
            }
        );
    }

    #[test]
    fn footnotes_are_collected_separately() {
        let doc = parse("Text[^1].\n\n[^1]: The note.\n");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.footnotes[0].label, "1");
        assert_eq!(
            doc.footnotes[0].blocks[0].kind,
            BlockKind::Paragraph(vec![text("The note.")])
        );
    }

    #[test]
    fn front_matter_is_kept_aside() {
        let doc = parse("---\ntitle: x\ntags:\n  - a\n---\n\nBody\n");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].kind, BlockKind::Paragraph(vec![text("Body")]));
        let meta = doc.front_matter.expect("front matter captured");
        assert_eq!(meta.format, MetaFormat::Yaml);
        assert_eq!(meta.text, "title: x\ntags:\n  - a");
        assert_eq!(meta.keys(), ["title", "tags"]);
        let doc = parse("+++\ntitle = \"x\"\n+++\n\nBody\n");
        assert_eq!(doc.front_matter.unwrap().format, MetaFormat::Toml);
    }

    #[test]
    fn alerts_math_wikilinks_and_emoji() {
        let doc = parse(
            "> [!WARNING]\n> Careful.\n\nSee [[Getting Started]] and $x^2$ :tada:\n\n$$\na = b\n$$\n",
        );
        let BlockKind::BlockQuote { alert, blocks } = &doc.blocks[0].kind else {
            panic!()
        };
        assert_eq!(*alert, Some(Alert::Warning));
        assert_eq!(blocks[0].kind, BlockKind::Paragraph(vec![text("Careful.")]));
        let BlockKind::Paragraph(inlines) = &doc.blocks[1].kind else {
            panic!()
        };
        assert_eq!(
            inlines,
            &[
                text("See "),
                Inline::WikiLink {
                    target: "Getting Started".into(),
                    content: vec![text("Getting Started")]
                },
                text(" and "),
                Inline::Math {
                    display: false,
                    source: "x^2".into()
                },
                text(" 🎉"),
            ]
        );
        assert!(matches!(
            &doc.blocks[2].kind,
            BlockKind::Paragraph(p) if matches!(&p[0], Inline::Math { display: true, .. })
        ));
    }

    #[test]
    fn definition_lists() {
        let doc = parse("Term\n: First meaning\n: Second meaning\n\nOther\n: Meaning\n");
        let BlockKind::DefinitionList(defs) = &doc.blocks[0].kind else {
            panic!("{:?}", doc.blocks[0].kind)
        };
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].term, vec![text("Term")]);
        assert_eq!(defs[0].details.len(), 2);
        assert_eq!(
            defs[0].details[1][0].kind,
            BlockKind::Paragraph(vec![text("Second meaning")])
        );
        assert_eq!(defs[1].details.len(), 1);
    }

    #[test]
    fn shortcodes_only_replace_known_emoji() {
        assert_eq!(emojify("at 10:30:45 :rocket: go"), "at 10:30:45 🚀 go");
        assert_eq!(emojify(":not_an_emoji_xyz:"), ":not_an_emoji_xyz:");
        assert_eq!(emojify("a:b"), "a:b");
    }

    #[test]
    fn quotes_nest_and_links_keep_urls() {
        let doc = parse("> quoted [link](https://x.y \"T\")\n>\n> > deeper\n");
        let BlockKind::BlockQuote {
            alert: None,
            blocks,
        } = &doc.blocks[0].kind
        else {
            panic!()
        };
        assert_eq!(blocks.len(), 2);
        let BlockKind::Paragraph(inlines) = &blocks[0].kind else {
            panic!()
        };
        assert_eq!(
            inlines[1],
            Inline::Link {
                content: vec![text("link")],
                url: "https://x.y".into(),
                title: "T".into()
            }
        );
        assert!(matches!(blocks[1].kind, BlockKind::BlockQuote { .. }));
    }
}
