use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use super::mermaid::{self, Kind};
use super::syntax;
use super::theme::{ColorMode, Theme};
use super::wrap::{LinkSpan, Piece, Wrapped, width, wrap};
use crate::markdown::{
    Alignment, Block, BlockKind, Document, Footnote, Inline, List, Slugger, Table, plain_text, slug,
};

const BULLETS: [&str; 3] = ["•", "◦", "▪"];

#[derive(Debug, Default)]
pub struct Layout {
    pub lines: Vec<Line<'static>>,
    pub sources: Vec<Option<usize>>,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub line: usize,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub line: usize,
    pub start: u16,
    pub end: u16,
    pub url: String,
}

impl Layout {
    pub fn line_for_source(&self, offset: usize) -> usize {
        self.sources
            .iter()
            .position(|s| s.is_some_and(|o| o >= offset))
            .unwrap_or(0)
    }

    pub fn line_for_anchor(&self, anchor: &str) -> Option<usize> {
        let wanted = slug(anchor);
        self.headings
            .iter()
            .find(|h| h.slug == wanted)
            .map(|h| h.line)
    }

    pub fn link_at(&self, line: usize, col: u16) -> Option<&Link> {
        self.links
            .iter()
            .find(|l| l.line == line && l.start <= col && col < l.end)
    }

    pub fn source_at(&self, line: usize) -> Option<usize> {
        self.sources.iter().skip(line).find_map(|s| *s)
    }

    pub fn plain_lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }
}

pub fn layout(doc: &Document, theme: &Theme, width: usize) -> Layout {
    let mut r = Renderer {
        theme,
        width: width.max(4),
        out: Layout::default(),
        prefix: Vec::new(),
        first: None,
        base: theme.text(),
        list_depth: 0,
        tight: false,
        source: None,
        slugger: Slugger::default(),
    };
    r.blocks(&doc.blocks);
    r.footnotes(&doc.footnotes);
    r.out
}

struct Renderer<'a> {
    theme: &'a Theme,
    width: usize,
    out: Layout,
    prefix: Vec<Span<'static>>,
    first: Option<Vec<Span<'static>>>,
    base: Style,
    list_depth: usize,
    tight: bool,
    source: Option<usize>,
    slugger: Slugger,
}

impl Renderer<'_> {
    fn avail(&self) -> usize {
        let used: usize = self.prefix.iter().map(|s| width(&s.content)).sum();
        self.width.saturating_sub(used).max(1)
    }

    fn emit(&mut self, spans: Vec<Span<'static>>) {
        self.emit_with(spans, Vec::new());
    }

    fn emit_line(&mut self, line: Wrapped) {
        self.emit_with(line.spans, line.links);
    }

    fn emit_with(&mut self, spans: Vec<Span<'static>>, links: Vec<LinkSpan>) {
        let mut line = self.first.take().unwrap_or_else(|| self.prefix.clone());
        let offset: usize = line.iter().map(|s| width(&s.content)).sum();
        let number = self.out.lines.len();
        for l in links {
            self.out.links.push(Link {
                line: number,
                start: (l.start + offset) as u16,
                end: (l.end + offset) as u16,
                url: l.url,
            });
        }
        line.extend(spans);
        while line
            .last()
            .is_some_and(|s| s.content.trim().is_empty() && s.style.bg.is_none())
        {
            line.pop();
        }
        self.out.lines.push(Line::from(line));
        self.out.sources.push(self.source);
    }

    fn blank(&mut self) {
        self.emit(Vec::new());
    }

    fn push_prefix(&mut self, span: Span<'static>) {
        if let Some(first) = &mut self.first {
            first.push(span.clone());
        }
        self.prefix.push(span);
    }

    fn pop_prefix(&mut self) {
        if let Some(first) = &mut self.first {
            first.pop();
        }
        self.prefix.pop();
    }

    fn blocks(&mut self, blocks: &[Block]) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 && !self.tight {
                self.blank();
                if matches!(block.kind, BlockKind::Heading { level: 1 | 2, .. }) {
                    self.blank();
                }
            }
            self.source = Some(block.span.start);
            self.block(block);
        }
    }

    fn block(&mut self, block: &Block) {
        match &block.kind {
            BlockKind::Heading { level, content } => self.heading(*level, content),
            BlockKind::Paragraph(content) => self.paragraph(content),
            BlockKind::CodeBlock { lang, code } => self.code_block(lang.as_deref(), code),
            BlockKind::BlockQuote(blocks) => self.quote(blocks),
            BlockKind::List(list) => self.list(list),
            BlockKind::Table(table) => self.table(table),
            BlockKind::Rule => self.rule(),
            BlockKind::Html(html) => self.html(html),
        }
    }

    fn heading(&mut self, level: u8, content: &[Inline]) {
        let text = plain_text(content);
        self.out.headings.push(Heading {
            level,
            slug: self.slugger.unique(&text),
            text,
            line: self.out.lines.len(),
        });
        let style = self.theme.heading(level);
        match level {
            1 if self.theme.mode == ColorMode::Mono => {
                for line in wrap(&self.inlines(content, style), self.avail()) {
                    self.emit_line(line);
                }
                let rule = "━".repeat(self.avail());
                self.emit(vec![Span::styled(rule, self.theme.rule())]);
            }
            1 => {
                let pieces = self.inlines(content, style);
                for mut line in wrap(&pieces, self.avail().saturating_sub(2).max(1)) {
                    let mut row = vec![Span::styled(" ", style)];
                    row.extend(line.spans);
                    row.push(Span::styled(" ", style));
                    for l in &mut line.links {
                        l.start += 1;
                        l.end += 1;
                    }
                    self.emit_with(row, line.links);
                }
            }
            2 => {
                let lines = wrap(&self.inlines(content, style), self.avail());
                let widest = lines
                    .iter()
                    .map(|l| l.spans.iter().map(|s| width(&s.content)).sum::<usize>())
                    .max()
                    .unwrap_or(1);
                for line in lines {
                    self.emit_line(line);
                }
                let rule = "─".repeat(widest.max(1));
                self.emit(vec![Span::styled(rule, self.theme.heading_rule())]);
            }
            _ => {
                let marker = format!("{} ", "#".repeat(level as usize));
                let indent = " ".repeat(marker.len());
                let mut first = self.prefix.clone();
                first.push(Span::styled(marker, self.theme.faint()));
                self.first = Some(first);
                self.prefix.push(Span::raw(indent));
                for line in wrap(&self.inlines(content, style), self.avail()) {
                    self.emit_line(line);
                }
                self.first = None;
                self.prefix.pop();
            }
        }
    }

    fn paragraph(&mut self, content: &[Inline]) {
        let pieces = self.inlines(content, self.base);
        for line in wrap(&pieces, self.avail()) {
            self.emit_line(line);
        }
    }

    fn inlines(&self, inlines: &[Inline], style: Style) -> Vec<Piece> {
        let mut out = Vec::new();
        self.push_inlines(inlines, style, None, &mut out);
        out
    }

    fn push_inlines(
        &self,
        inlines: &[Inline],
        style: Style,
        link: Option<&str>,
        out: &mut Vec<Piece>,
    ) {
        for inline in inlines {
            match inline {
                Inline::Text(s) => {
                    out.push(Piece::text(s.replace('\t', "    "), style).linked(link))
                }
                Inline::Code(s) => {
                    let text = if self.theme.mode == ColorMode::Mono {
                        format!("`{s}`")
                    } else {
                        format!(" {s} ")
                    };
                    out.push(Piece::atomic(text, self.theme.code_inline()).linked(link));
                }
                Inline::Emphasis(c) => {
                    self.push_inlines(c, style.add_modifier(Modifier::ITALIC), link, out)
                }
                Inline::Strong(c) => {
                    self.push_inlines(c, style.add_modifier(Modifier::BOLD), link, out)
                }
                Inline::Strikethrough(c) => {
                    self.push_inlines(c, style.add_modifier(Modifier::CROSSED_OUT), link, out)
                }
                Inline::Link { content, url, .. } => {
                    let target = Some(url.as_str());
                    self.push_inlines(content, style.patch(self.theme.link()), target, out);
                    if !url.is_empty() && plain_text(content) != *url {
                        out.push(Piece::text(" ", style).linked(target));
                        out.push(
                            Piece::text(format!("({url})"), self.theme.link_url()).linked(target),
                        );
                    }
                }
                Inline::Image { alt, url } => {
                    out.push(Piece::text("▣ ", Style::new().fg(self.theme.accent)).linked(link));
                    self.push_inlines(alt, style.add_modifier(Modifier::ITALIC), link, out);
                    out.push(Piece::text(format!(" ({url})"), self.theme.link_url()).linked(link));
                }
                Inline::FootnoteRef(label) => out
                    .push(Piece::atomic(footnote_label(label), self.theme.footnote()).linked(link)),
                Inline::Html(s) => {
                    out.push(Piece::text(s.clone(), self.theme.faint()).linked(link))
                }
                Inline::SoftBreak => out.push(Piece::text(" ", style).linked(link)),
                Inline::HardBreak => out.push(Piece::Break),
            }
        }
    }

    fn code_block(&mut self, lang: Option<&str>, code: &str) {
        let code = code.replace('\t', "    ");
        let inner = self.avail().saturating_sub(2).max(1);
        if lang == Some("mermaid")
            && let Some(drawing) = mermaid::render(&code, inner.saturating_sub(8))
            && drawing.width() + 8 <= inner
        {
            self.diagram(&drawing, inner);
            return;
        }
        let block_style = self.theme.code_block();
        let mut lines = lang
            .and_then(|l| syntax::highlight(l, &code, self.theme))
            .unwrap_or_else(|| {
                code.lines()
                    .map(|l| vec![Span::styled(l.to_string(), block_style)])
                    .collect()
            });
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        for (i, mut spans) in lines.into_iter().enumerate() {
            for span in spans.iter_mut() {
                span.style = span.style.bg(self.theme.code_bg);
            }
            let w: usize = spans.iter().map(|s| width(&s.content)).sum();
            let mut row = vec![Span::styled("▎ ", self.theme.code_border())];
            if w > inner {
                row.extend(truncate(spans, inner - 1));
                row.push(Span::styled("…", self.theme.faint().bg(self.theme.code_bg)));
            } else {
                row.extend(spans);
                let mut pad = inner - w;
                if i == 0
                    && let Some(label) = lang
                    && pad >= width(label) + 2
                {
                    row.push(Span::styled(" ".repeat(pad - width(label)), block_style));
                    row.push(Span::styled(
                        label.to_string(),
                        self.theme.faint().bg(self.theme.code_bg),
                    ));
                    pad = 0;
                }
                if pad > 0 {
                    row.push(Span::styled(" ".repeat(pad), block_style));
                }
            }
            self.emit(row);
        }
    }

    fn diagram(&mut self, drawing: &mermaid::Drawing, inner: usize) {
        let bg = self.theme.code_bg;
        let block_style = self.theme.code_block();
        let style = |kind: Kind| match kind {
            Kind::Text => self.theme.text().bg(bg),
            Kind::Border => self.theme.muted().bg(bg),
            Kind::Edge => Style::new().fg(self.theme.accent).bg(bg),
        };
        let rows: Vec<Vec<Span<'static>>> = std::iter::once(Vec::new())
            .chain(drawing.rows.iter().map(|runs| {
                runs.iter()
                    .map(|(kind, text)| Span::styled(text.clone(), style(*kind)))
                    .collect()
            }))
            .chain(std::iter::once(Vec::new()))
            .collect();
        for (i, spans) in rows.into_iter().enumerate() {
            let used: usize = spans.iter().map(|s| width(&s.content)).sum();
            let mut row = vec![
                Span::styled("▎ ", self.theme.code_border()),
                Span::styled(" ", block_style),
            ];
            row.extend(spans);
            let mut pad = inner.saturating_sub(used + 1);
            if i == 0 && pad >= 9 {
                row.push(Span::styled(" ".repeat(pad - 7), block_style));
                row.push(Span::styled("mermaid", self.theme.faint().bg(bg)));
                pad = 0;
            }
            if pad > 0 {
                row.push(Span::styled(" ".repeat(pad), block_style));
            }
            self.emit(row);
        }
    }

    fn quote(&mut self, blocks: &[Block]) {
        let base = self.base;
        let tight = self.tight;
        self.push_prefix(Span::styled("▎ ", self.theme.quote_bar()));
        self.base = self.theme.quote();
        self.tight = false;
        self.blocks(blocks);
        self.tight = tight;
        self.base = base;
        self.pop_prefix();
    }

    fn list(&mut self, list: &List) {
        let depth = self.list_depth;
        let tight = self.tight;
        self.list_depth += 1;
        self.tight = list.tight;
        let last = list
            .start
            .map(|s| s + list.items.len().saturating_sub(1) as u64);
        let num_width = last.map(|n| n.to_string().len() + 1).unwrap_or(0);
        for (i, item) in list.items.iter().enumerate() {
            if i > 0 && !list.tight {
                self.blank();
            }
            let marker = match (item.task, list.start) {
                (Some(true), _) => "☑ ".to_string(),
                (Some(false), _) => "☐ ".to_string(),
                (None, Some(start)) => format!("{:>num_width$} ", format!("{}.", start + i as u64)),
                (None, None) => format!("{} ", BULLETS[depth % BULLETS.len()]),
            };
            let indent = " ".repeat(width(&marker));
            let marker_style = if item.task == Some(true) {
                self.theme.faint()
            } else {
                self.theme.list_marker_at(depth)
            };
            let mut first = self.prefix.clone();
            first.push(Span::styled(marker, marker_style));
            self.first = Some(first);
            self.prefix.push(Span::raw(indent));
            let base = self.base;
            if item.task == Some(true) {
                self.base = self.theme.muted();
            }
            if item.blocks.is_empty() {
                self.emit(Vec::new());
            } else {
                self.blocks(&item.blocks);
            }
            self.base = base;
            self.first = None;
            self.prefix.pop();
        }
        self.tight = tight;
        self.list_depth = depth;
    }

    fn table(&mut self, table: &Table) {
        let ncols = table
            .header
            .len()
            .max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
        if ncols == 0 {
            return;
        }
        let to_pieces = |r: &Renderer, cells: &[Vec<Inline>], style: Style| -> Vec<Vec<Piece>> {
            (0..ncols)
                .map(|c| {
                    cells
                        .get(c)
                        .map(|cell| r.inlines(cell, style))
                        .unwrap_or_default()
                })
                .collect()
        };
        let header = to_pieces(self, &table.header, self.theme.table_header());
        let rows: Vec<Vec<Vec<Piece>>> = table
            .rows
            .iter()
            .map(|r| to_pieces(self, r, self.base))
            .collect();

        let mut max_w = vec![1usize; ncols];
        let mut min_w = vec![1usize; ncols];
        for row in std::iter::once(&header).chain(rows.iter()) {
            for (c, cell) in row.iter().enumerate() {
                max_w[c] = max_w[c].max(pieces_width(cell));
                min_w[c] = min_w[c].max(pieces_min_width(cell));
            }
        }
        let inner = self.avail().saturating_sub(3 * ncols + 1);
        let mut widths = max_w;
        let mut excess = widths.iter().sum::<usize>().saturating_sub(inner);
        while excess > 0 {
            let candidate = (0..ncols)
                .filter(|&c| widths[c] > min_w[c])
                .max_by_key(|&c| widths[c])
                .or_else(|| {
                    (0..ncols)
                        .filter(|&c| widths[c] > 1)
                        .max_by_key(|&c| widths[c])
                });
            let Some(c) = candidate else { break };
            widths[c] -= 1;
            excess -= 1;
        }

        let border = self.theme.table_border();
        let rule = |l: &str, fill: &str, m: &str, r: &str| -> Vec<Span<'static>> {
            let mut s = String::from(l);
            for (c, w) in widths.iter().enumerate() {
                if c > 0 {
                    s.push_str(m);
                }
                s.push_str(&fill.repeat(w + 2));
            }
            s.push_str(r);
            vec![Span::styled(s, border)]
        };
        let top = rule("┌", "─", "┬", "┐");
        let mid = rule("┝", "━", "┿", "┥");
        let bottom = rule("└", "─", "┴", "┘");

        self.emit(top);
        self.table_row(
            &header,
            &widths,
            &table.alignments,
            Some(self.theme.table_band()),
        );
        self.emit(mid);
        for row in &rows {
            self.table_row(row, &widths, &table.alignments, None);
        }
        self.emit(bottom);
    }

    fn table_row(
        &mut self,
        cells: &[Vec<Piece>],
        widths: &[usize],
        aligns: &[Alignment],
        band: Option<Style>,
    ) {
        let border = self.theme.table_border();
        let wrapped: Vec<Vec<Wrapped>> = cells
            .iter()
            .zip(widths)
            .map(|(cell, w)| wrap(cell, *w))
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(0).max(1);
        for i in 0..height {
            let mut spans = vec![Span::styled("│", border)];
            let mut links: Vec<LinkSpan> = Vec::new();
            let mut col = 1;
            for (c, w) in widths.iter().enumerate() {
                let line = wrapped[c].get(i);
                let used = line
                    .map(|l| l.spans.iter().map(|s| width(&s.content)).sum())
                    .unwrap_or(0);
                let pad = w.saturating_sub(used);
                let (left, right) = match aligns.get(c).copied().unwrap_or_default() {
                    Alignment::Left => (0, pad),
                    Alignment::Right => (pad, 0),
                    Alignment::Center => (pad / 2, pad - pad / 2),
                };
                spans.push(Span::raw(" ".repeat(left + 1)));
                col += left + 1;
                if let Some(line) = line {
                    spans.extend(line.spans.iter().cloned());
                    links.extend(line.links.iter().map(|l| LinkSpan {
                        start: l.start + col,
                        end: l.end + col,
                        url: l.url.clone(),
                    }));
                }
                col += used;
                spans.push(Span::raw(" ".repeat(right + 1)));
                col += right + 1;
                spans.push(Span::styled("│", border));
                col += 1;
            }
            if let Some(band) = band {
                for span in spans.iter_mut() {
                    span.style = span.style.patch(band);
                }
            }
            self.emit_with(spans, links);
        }
    }

    fn rule(&mut self) {
        let line = "─".repeat(self.avail());
        self.emit(vec![Span::styled(line, self.theme.rule())]);
    }

    fn html(&mut self, html: &str) {
        let style = self.theme.faint();
        let mut pieces = Vec::new();
        for (i, line) in html.lines().enumerate() {
            if i > 0 {
                pieces.push(Piece::Break);
            }
            pieces.push(Piece::text(line.replace('\t', "    "), style));
        }
        for line in wrap(&pieces, self.avail()) {
            self.emit_line(line);
        }
    }

    fn footnotes(&mut self, footnotes: &[Footnote]) {
        if footnotes.is_empty() {
            return;
        }
        self.source = None;
        self.blank();
        self.rule();
        for note in footnotes {
            self.blank();
            let marker = format!("{} ", footnote_label(&note.label));
            let indent = " ".repeat(width(&marker));
            let mut first = self.prefix.clone();
            first.push(Span::styled(marker, self.theme.footnote()));
            self.first = Some(first);
            self.prefix.push(Span::raw(indent));
            let base = self.base;
            self.base = self.theme.muted();
            self.blocks(&note.blocks);
            self.base = base;
            self.first = None;
            self.prefix.pop();
        }
    }
}

fn pieces_width(pieces: &[Piece]) -> usize {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text { text, .. } => width(text),
            Piece::Break => 0,
        })
        .sum()
}

fn pieces_min_width(pieces: &[Piece]) -> usize {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text {
                text, atomic: true, ..
            } => width(text),
            Piece::Text { text, .. } => text.split(' ').map(width).max().unwrap_or(0),
            Piece::Break => 0,
        })
        .max()
        .unwrap_or(0)
}

fn truncate(spans: Vec<Span<'static>>, max: usize) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut used = 0;
    for span in spans {
        let w = width(&span.content);
        if used + w <= max {
            used += w;
            out.push(span);
            continue;
        }
        let mut text = String::new();
        for ch in span.content.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cw > max {
                break;
            }
            used += cw;
            text.push(ch);
        }
        if !text.is_empty() {
            out.push(Span::styled(text, span.style));
        }
        break;
    }
    out
}

fn footnote_label(label: &str) -> String {
    const SUPERSCRIPT: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    if !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit()) {
        label
            .bytes()
            .map(|b| SUPERSCRIPT[(b - b'0') as usize])
            .collect()
    } else {
        format!("[{label}]")
    }
}

#[cfg(test)]
mod tests {
    use super::footnote_label;

    #[test]
    fn numeric_footnotes_become_superscripts() {
        assert_eq!(footnote_label("1"), "¹");
        assert_eq!(footnote_label("12"), "¹²");
        assert_eq!(footnote_label("note"), "[note]");
    }
}
