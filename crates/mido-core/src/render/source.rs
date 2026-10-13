//! Styles for Markdown source as the editor shows it: the syntax colored with the theme's
//! tokens, from pulldown-cmark's offsets, and fenced code through syntect.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Parser, Tag, TagEnd};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::Span;

use super::syntax;
use super::theme::Theme;
use crate::markdown::options;

/// A style over a byte range of one line. Later runs patch earlier ones.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub range: Range<usize>,
    pub style: Style,
}

type Highlighted = Vec<Vec<Span<'static>>>;

/// Colors Markdown source line by line, keeping syntect's work on code blocks that did not
/// change since the last call.
#[derive(Default)]
pub struct SourceHighlighter {
    code: HashMap<(String, String), Option<Rc<Highlighted>>>,
}

impl SourceHighlighter {
    pub fn highlight(&mut self, lines: &[String], theme: &Theme) -> Vec<Vec<Run>> {
        let text = lines.join("\n");
        let mut starts = Vec::with_capacity(lines.len());
        let mut at = 0;
        for line in lines {
            starts.push(at);
            at += line.len() + 1;
        }
        let mut out = Painter {
            text: &text,
            starts,
            runs: vec![Vec::new(); lines.len()],
        };
        let mut used = HashMap::new();
        let mut lists: Vec<usize> = Vec::new();
        let mut code: Option<(Option<String>, Option<usize>, String)> = None;
        for (event, range) in Parser::new_ext(&text, options()).into_offset_iter() {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => out.heading(range, level as u8, theme),
                    Tag::BlockQuote(_) => out.quote(range, theme),
                    Tag::CodeBlock(kind) => {
                        out.paint(range.clone(), Style::new().fg(theme.code_fg));
                        let lang = match &kind {
                            CodeBlockKind::Fenced(info) => {
                                out.fences(range, theme);
                                info.split_whitespace().next().map(str::to_string)
                            }
                            CodeBlockKind::Indented => None,
                        };
                        code = Some((lang, None, String::new()));
                    }
                    Tag::List(_) => lists.push(0),
                    Tag::Item => out.item(range, lists.len().saturating_sub(1), theme),
                    Tag::FootnoteDefinition(_) => out.label(range, theme.footnote()),
                    Tag::Table(_) => out.table(range, theme),
                    Tag::TableHead => out.paint(range, Style::new().add_modifier(Modifier::BOLD)),
                    Tag::Emphasis => {
                        out.paint(range.clone(), Style::new().add_modifier(Modifier::ITALIC));
                        out.delimiters(range, theme);
                    }
                    Tag::Strong => {
                        out.paint(range.clone(), Style::new().add_modifier(Modifier::BOLD));
                        out.delimiters(range, theme);
                    }
                    Tag::Strikethrough => {
                        out.paint(
                            range.clone(),
                            Style::new().add_modifier(Modifier::CROSSED_OUT),
                        );
                        out.delimiters(range, theme);
                    }
                    Tag::Link { link_type, .. } => out.link(range, link_type, false, theme),
                    Tag::Image { link_type, .. } => out.link(range, link_type, true, theme),
                    Tag::MetadataBlock(_) => out.front_matter(range, theme),
                    Tag::HtmlBlock => out.paint(range, theme.faint()),
                    Tag::DefinitionListDefinition => out.marker(range, ':', theme.list_marker()),
                    _ => {}
                },
                Event::End(TagEnd::List(_)) => {
                    lists.pop();
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((Some(lang), Some(start), body)) = code.take() {
                        let key = (lang, body);
                        let spans = match used.get(&key).or_else(|| self.code.get(&key)) {
                            Some(spans) => spans.clone(),
                            None => syntax::highlight(&key.0, &key.1, theme).map(Rc::new),
                        };
                        if let Some(spans) = &spans {
                            out.code(start, spans);
                        }
                        used.insert(key, spans);
                    }
                }
                Event::Text(text) => {
                    if let Some((_, start, body)) = &mut code {
                        start.get_or_insert(range.start);
                        body.push_str(&text);
                    }
                }
                Event::Code(_) => out.paint(range, theme.code_inline()),
                Event::InlineMath(_) | Event::DisplayMath(_) => out.paint(range, theme.math()),
                Event::Html(_) | Event::InlineHtml(_) => out.paint(range, theme.faint()),
                Event::FootnoteReference(_) => out.paint(range, theme.footnote()),
                Event::TaskListMarker(_) => out.paint(range, theme.accent()),
                Event::Rule => out.paint(range, theme.rule()),
                _ => {}
            }
        }
        self.code = used;
        out.runs
    }
}

struct Painter<'a> {
    text: &'a str,
    starts: Vec<usize>,
    runs: Vec<Vec<Run>>,
}

impl Painter<'_> {
    fn line_of(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1)
    }

    fn line_end(&self, line: usize) -> usize {
        self.starts
            .get(line + 1)
            .map_or(self.text.len(), |next| next - 1)
    }

    fn line_text(&self, line: usize) -> &str {
        &self.text[self.starts[line]..self.line_end(line)]
    }

    fn lines(&self, range: &Range<usize>) -> Range<usize> {
        if self.starts.is_empty() || range.start >= range.end {
            return 0..0;
        }
        self.line_of(range.start)..self.line_of(range.end - 1) + 1
    }

    fn paint(&mut self, range: Range<usize>, style: Style) {
        for line in self.lines(&range) {
            let start = self.starts[line];
            let from = range.start.max(start) - start;
            let to = range.end.min(self.line_end(line)).saturating_sub(start);
            if from < to {
                self.runs[line].push(Run {
                    range: from..to,
                    style,
                });
            }
        }
    }

    fn paint_line(&mut self, line: usize, range: Range<usize>, style: Style) {
        let start = self.starts[line];
        self.paint(start + range.start..start + range.end, style);
    }

    fn heading(&mut self, range: Range<usize>, level: u8, theme: &Theme) {
        self.paint(range.clone(), theme.heading_text(level));
        let lines = self.lines(&range);
        let first = self.line_text(lines.start);
        let indent = first.len() - first.trim_start().len();
        let hashes = first[indent..].bytes().take_while(|&b| b == b'#').count();
        if hashes > 0 {
            self.paint_line(lines.start, indent..indent + hashes, theme.faint());
        } else if lines.len() > 1 {
            let last = lines.end - 1;
            let len = self.line_text(last).len();
            self.paint_line(last, 0..len, theme.faint());
        }
    }

    fn quote(&mut self, range: Range<usize>, theme: &Theme) {
        self.paint(range.clone(), theme.quote());
        for line in self.lines(&range) {
            let text = self.line_text(line);
            let from = range.start.max(self.starts[line]) - self.starts[line];
            let rest = &text[from..];
            let indent = rest.len() - rest.trim_start().len();
            if rest[indent..].starts_with('>') {
                let at = from + indent;
                self.paint_line(line, at..at + 1, theme.quote_bar());
            }
        }
    }

    fn fences(&mut self, range: Range<usize>, theme: &Theme) {
        let lines = self.lines(&range);
        for line in [lines.start, lines.end.saturating_sub(1)] {
            let text = self.line_text(line).trim_start();
            if text.starts_with("```") || text.starts_with("~~~") {
                let len = self.line_text(line).len();
                self.paint_line(line, 0..len, theme.faint());
            }
        }
    }

    fn code(&mut self, start: usize, spans: &[Vec<Span<'static>>]) {
        let first = self.line_of(start);
        for (i, spans) in spans.iter().enumerate() {
            let line = first + i;
            if line >= self.starts.len() {
                break;
            }
            let width: usize = spans.iter().map(|s| s.content.len()).sum();
            let mut at = self.line_text(line).len().saturating_sub(width);
            for span in spans {
                let len = span.content.len();
                let mut style = span.style;
                style.bg = None;
                self.paint_line(line, at..at + len, style);
                at += len;
            }
        }
    }

    fn item(&mut self, range: Range<usize>, depth: usize, theme: &Theme) {
        let rest = &self.text[range.start..];
        let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        self.paint(range.start..range.start + len, theme.list_marker_at(depth));
    }

    fn marker(&mut self, range: Range<usize>, marker: char, style: Style) {
        let rest = &self.text[range.start..range.end];
        let indent = rest.len() - rest.trim_start().len();
        if rest[indent..].starts_with(marker) {
            let at = range.start + indent;
            self.paint(at..at + marker.len_utf8(), style);
        }
    }

    fn label(&mut self, range: Range<usize>, style: Style) {
        let rest = &self.text[range.start..range.end];
        if let Some(end) = rest.find("]:") {
            self.paint(range.start..range.start + end + 2, style);
        }
    }

    fn delimiters(&mut self, range: Range<usize>, theme: &Theme) {
        let text = &self.text[range.clone()];
        let Some(first) = text.chars().next() else {
            return;
        };
        let open = text.chars().take_while(|&c| c == first).count();
        let close = text.chars().rev().take_while(|&c| c == first).count();
        if open + close > text.chars().count() {
            return;
        }
        self.paint(range.start..range.start + open, theme.faint());
        self.paint(range.end - close..range.end, theme.faint());
    }

    fn link(&mut self, range: Range<usize>, kind: LinkType, image: bool, theme: &Theme) {
        self.paint(range.clone(), theme.link());
        let text = &self.text[range.clone()];
        if image {
            self.paint(range.start..range.start + 1, theme.accent());
        }
        let target = match kind {
            LinkType::Inline if text.ends_with(')') => text.rfind("](").map(|i| i + 1),
            LinkType::Reference if text.ends_with(']') => text.rfind("][").map(|i| i + 1),
            LinkType::WikiLink { .. } => {
                self.paint(range.start..range.start + 2, theme.faint());
                self.paint(range.end.saturating_sub(2)..range.end, theme.faint());
                None
            }
            _ => None,
        };
        if let Some(at) = target {
            self.paint(range.start + at..range.end, theme.link_url());
        }
    }

    fn table(&mut self, range: Range<usize>, theme: &Theme) {
        let lines = self.lines(&range);
        for (i, line) in lines.enumerate() {
            let text = self.line_text(line);
            if i == 1 {
                let len = text.len();
                self.paint_line(line, 0..len, theme.table_border());
                continue;
            }
            let pipes: Vec<usize> = text
                .char_indices()
                .filter(|&(j, c)| c == '|' && !text[..j].ends_with('\\'))
                .map(|(j, _)| j)
                .collect();
            for j in pipes {
                self.paint_line(line, j..j + 1, theme.table_border());
            }
        }
    }

    fn front_matter(&mut self, range: Range<usize>, theme: &Theme) {
        self.paint(range.clone(), theme.muted());
        let lines = self.lines(&range);
        let start = lines.start.saturating_sub(1);
        for line in [start, lines.end] {
            if line < self.starts.len() {
                let text = self.line_text(line).trim_end();
                if text == "---" || text == "+++" {
                    let len = text.len();
                    self.paint_line(line, 0..len, theme.faint());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(source: &str) -> (Vec<Vec<Run>>, Theme) {
        let theme = Theme::dark();
        let lines: Vec<String> = source.split('\n').map(str::to_string).collect();
        let runs = SourceHighlighter::default().highlight(&lines, &theme);
        (runs, theme)
    }

    fn style_at(runs: &[Run], byte: usize) -> Style {
        runs.iter()
            .filter(|r| r.range.contains(&byte))
            .fold(Style::new(), |acc, r| acc.patch(r.style))
    }

    #[test]
    fn headings_take_their_level_color_with_faint_hashes() {
        let (runs, theme) = runs("# Title\n\ntext\n\n### Third");
        assert_eq!(style_at(&runs[0], 0).fg, Some(theme.fg_faint));
        assert_eq!(style_at(&runs[0], 3).fg, Some(theme.h1));
        assert!(runs[2].is_empty());
        assert_eq!(style_at(&runs[4], 5).fg, Some(theme.h3));
    }

    #[test]
    fn inline_markup_patches_over_the_block() {
        let (runs, theme) = runs("Some *em* and `code` and [a](b.md)");
        let line = &runs[0];
        assert_eq!(style_at(line, 5).fg, Some(theme.fg_faint));
        assert!(style_at(line, 6).add_modifier.contains(Modifier::ITALIC));
        assert_eq!(style_at(line, 15).fg, Some(theme.code_fg));
        assert_eq!(style_at(line, 26).fg, Some(theme.link));
        assert_eq!(style_at(line, 29).fg, Some(theme.link_url));
    }

    #[test]
    fn lists_quotes_and_tasks_mark_their_markers() {
        let (runs, theme) = runs("- [ ] one\n  - two\n\n> quoted");
        assert_eq!(style_at(&runs[0], 0).fg, Some(theme.list_marker));
        assert_eq!(style_at(&runs[0], 3).fg, Some(theme.accent));
        assert_eq!(style_at(&runs[1], 2).fg, Some(theme.h2));
        assert_eq!(style_at(&runs[3], 0).fg, Some(theme.quote_bar));
        assert_eq!(style_at(&runs[3], 3).fg, Some(theme.quote_fg));
    }

    #[test]
    fn fenced_code_is_highlighted_and_cached() {
        let theme = Theme::dark();
        let lines: Vec<String> = "```rust\nfn main() {}\n```"
            .split('\n')
            .map(str::to_string)
            .collect();
        let mut highlighter = SourceHighlighter::default();
        let first = highlighter.highlight(&lines, &theme);
        assert_eq!(style_at(&first[0], 0).fg, Some(theme.fg_faint));
        assert_eq!(style_at(&first[2], 0).fg, Some(theme.fg_faint));
        let keyword = style_at(&first[1], 0);
        assert_ne!(keyword.fg, Some(theme.code_fg));
        assert_eq!(keyword.bg, None);
        assert_eq!(highlighter.code.len(), 1);
        assert_eq!(highlighter.highlight(&lines, &theme), first);
    }

    #[test]
    fn tables_and_front_matter() {
        let (runs, theme) = runs("---\ntitle: x\n---\n\n| a | b |\n| - | - |\n| 1 | 2 |");
        assert_eq!(style_at(&runs[0], 0).fg, Some(theme.fg_faint));
        assert_eq!(style_at(&runs[1], 0).fg, Some(theme.fg_muted));
        assert_eq!(style_at(&runs[4], 0).fg, Some(theme.table_border));
        assert!(style_at(&runs[4], 2).add_modifier.contains(Modifier::BOLD));
        assert_eq!(style_at(&runs[5], 2).fg, Some(theme.table_border));
        assert_eq!(style_at(&runs[6], 4).fg, Some(theme.table_border));
    }
}
