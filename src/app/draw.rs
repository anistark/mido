use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
};

use super::keys::{self, SECTIONS};
use super::{App, Focus, Mode, Panel};
use crate::render::theme::ColorMode;
use crate::render::wrap::{Piece, width, wrap};

const MIN_CONTENT_AUTO: u16 = 60;
const MIN_CONTENT_FORCED: u16 = 40;

impl App {
    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        if area.height < 2 || area.width < 8 {
            return;
        }
        let body = Rect {
            height: area.height - 1,
            ..area
        };
        let status_area = Rect {
            y: area.y + area.height - 1,
            height: 1,
            ..area
        };
        if self.layout_width == 0 {
            self.ensure_layout(self.content_width(body.width));
        }
        let (files_area, outline_area, content_area) = self.plan_panels(body);
        self.files.area = files_area;
        self.outline.area = outline_area;
        if (self.focus == Focus::Files && files_area.is_none())
            || (self.focus == Focus::Outline && outline_area.is_none())
        {
            self.focus = Focus::Content;
        }
        let content_width = self.content_width(content_area.width);
        self.ensure_layout(content_width);
        self.view_height = content_area.height as usize;
        self.clamp_scroll();
        if let Some(anchor) = self.pending_anchor.take() {
            match self.layout.line_for_anchor(&anchor) {
                Some(line) => self.scroll = line.min(self.max_scroll()),
                None => self.notice = Some(format!("no heading #{anchor}")),
            }
        }
        if self.focus != Focus::Outline {
            let row = self.active_heading().and_then(|h| self.outline.row_for(h));
            self.outline.list.select(row);
        }

        let (left, right) = gutters(content_area.width);
        let available = (content_area.width - left - right) as usize;
        let x = content_area.x + left + ((available - content_width) / 2) as u16;
        self.content_origin = (x, content_area.y);
        self.content_cols = content_width as u16;
        let buf = frame.buffer_mut();
        if self.layout.lines.is_empty() {
            let notice = if self.project.is_some() && self.file.is_none() {
                "No Markdown files here yet."
            } else {
                "Nothing to show, the file is empty."
            };
            buf.set_stringn(x, content_area.y, notice, content_width, self.theme.faint());
        }
        for (row, line) in self
            .layout
            .lines
            .iter()
            .skip(self.scroll)
            .take(self.view_height)
            .enumerate()
        {
            let y = content_area.y + row as u16;
            buf.set_line(x, y, line, content_width as u16);
            if line.width() > content_width {
                buf[(x + content_width as u16 - 1, y)]
                    .set_symbol("…")
                    .set_style(self.theme.faint());
            }
        }
        self.draw_matches(buf, x, content_area.y, content_width as u16);
        if let Some(link) = self.link.and_then(|i| self.layout.links.get(i))
            && link.line >= self.scroll
            && link.line < self.scroll + self.view_height
        {
            let row = content_area.y + (link.line - self.scroll) as u16;
            for col in link.start..link.end.min(content_width as u16) {
                buf[(x + col, row)].set_style(self.theme.status_chip());
            }
        }

        if self.layout.lines.len() > self.view_height {
            let mut state = ScrollbarState::new(self.max_scroll() + 1)
                .position(self.scroll)
                .viewport_content_length(self.view_height);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(self.theme.rule())
                .thumb_style(self.theme.faint())
                .render(
                    Rect {
                        x: body.x,
                        width: body.width,
                        ..content_area
                    },
                    buf,
                    &mut state,
                );
        }

        if let Some(panel) = files_area {
            self.draw_files(buf, panel);
        }
        if let Some(panel) = outline_area {
            self.draw_outline(buf, panel);
        }
        self.draw_status(buf, status_area);
        match self.mode {
            Mode::Help => self.draw_help(buf, body),
            Mode::Toc => self.draw_toc(buf, area),
            Mode::Finder => self.draw_finder(buf, area),
            _ => {}
        }
    }

    fn draw_finder(&mut self, buf: &mut Buffer, area: Rect) {
        let w = area.width.saturating_sub(6).clamp(20, 90);
        let rows = self.finder.hits.len().max(1) as u16;
        let h = (rows + 4).min(area.height.saturating_sub(2).max(5));
        let popup = centered(area, w, h);
        Clear.render(popup, buf);
        let block = self
            .popup_block(" Find file ")
            .title_bottom(self.hint(" Enter open · Esc close "));
        let inner = block.inner(popup);
        block.render(popup, buf);
        let prompt = format!(" > {}▏", self.finder.query);
        let prompt_style = Style::new().fg(self.theme.accent);
        buf.set_stringn(
            inner.x,
            inner.y,
            &prompt,
            inner.width as usize,
            prompt_style,
        );
        let list_area = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(2),
            ..inner
        };
        if self.finder.hits.is_empty() {
            let room = list_area.width.saturating_sub(3) as usize;
            buf.set_stringn(
                list_area.x + 3,
                list_area.y,
                "No matches",
                room,
                self.theme.faint(),
            );
            return;
        }
        let items: Vec<ListItem> = self
            .finder
            .hits
            .iter()
            .map(|hit| {
                let mut spans = Vec::new();
                let mut run = String::new();
                let mut run_hit = false;
                for (i, c) in hit.label.chars().enumerate() {
                    let is_hit = hit.indices.binary_search(&(i as u32)).is_ok();
                    if is_hit != run_hit && !run.is_empty() {
                        spans.push(self.finder_span(std::mem::take(&mut run), run_hit));
                    }
                    run_hit = is_hit;
                    run.push(c);
                }
                if !run.is_empty() {
                    spans.push(self.finder_span(run, run_hit));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let mut state = ListState::default().with_selected(Some(self.finder.selected));
        let list = List::new(items)
            .highlight_style(self.theme.selected())
            .highlight_symbol("▸ ");
        StatefulWidget::render(list, list_area, buf, &mut state);
    }

    fn finder_span(&self, text: String, matched: bool) -> Span<'static> {
        if matched {
            Span::styled(
                text,
                Style::new()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(text, self.theme.text())
        }
    }

    fn content_width(&self, area_width: u16) -> usize {
        let (left, right) = gutters(area_width);
        let available = area_width.saturating_sub(left + right) as usize;
        self.max_width
            .map_or(available, |cap| available.min(cap))
            .max(4)
    }

    fn plan_panels(&self, body: Rect) -> (Option<Rect>, Option<Rect>, Rect) {
        let margin: u16 = match body.height {
            0..=5 => 0,
            6..=9 => 1,
            _ => 2,
        };
        let mut left = body.x;
        let mut right = body.right();
        let mut files = None;
        let mut outline = None;
        if body.height >= 7 && !self.focus_mode {
            let (wanted, forced) = match self.show_files {
                Some(v) => (v, true),
                None => (self.project.is_some(), false),
            };
            let min = if forced {
                MIN_CONTENT_FORCED
            } else {
                MIN_CONTENT_AUTO
            };
            let w = (body.width / 4).clamp(22, 32);
            if wanted && (right - left).saturating_sub(w) >= min {
                files = Some(Rect {
                    x: left,
                    y: body.y,
                    width: w,
                    height: body.height,
                });
                left += w;
            }
            let (wanted, forced) = match self.show_outline {
                Some(v) => (v, true),
                None => (true, false),
            };
            let min = if forced {
                MIN_CONTENT_FORCED
            } else {
                MIN_CONTENT_AUTO
            };
            let widest = self.outline_widest.max(11) as u16;
            let w = (widest + 7).clamp(18, 32).min(body.width / 3).max(14);
            if wanted && (right - left).saturating_sub(w + 2) >= min {
                let list_width = w.saturating_sub(4);
                let rows = self
                    .outline
                    .rows
                    .iter()
                    .map(|r| {
                        let label = &self.layout.headings[r.index].text;
                        wrap_label(label, label_width(list_width, &r.prefix, r.marker)).len()
                    })
                    .sum::<usize>()
                    .max(1) as u16;
                let h = (rows + 4)
                    .clamp(5, body.height.saturating_sub(2 * margin.max(1)).max(5))
                    .min(body.height);
                outline = Some(Rect {
                    x: right - 2 - w,
                    y: body.y + margin.max(1).min(body.height.saturating_sub(h)),
                    width: w,
                    height: h,
                });
                right -= w + 2;
            }
        }
        let content = Rect {
            x: left,
            y: body.y + margin,
            width: right - left,
            height: body.height - 2 * margin,
        };
        (files, outline, content)
    }

    fn card(&self, buf: &mut Buffer, panel: Rect, title: &str, focused: bool) -> Rect {
        let glass = self
            .theme
            .faint()
            .bg(self.theme.surface)
            .remove_modifier(Modifier::all());
        for y in panel.y..panel.bottom() {
            for x in panel.x..panel.right() {
                let cell = &mut buf[(x, y)];
                if self.theme.mode == ColorMode::Mono {
                    cell.reset();
                } else {
                    cell.set_style(glass);
                }
            }
        }
        let (border, title_style) = if focused {
            (
                self.theme.overlay_border(),
                Style::new()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                self.theme.table_border(),
                self.theme.muted().add_modifier(Modifier::BOLD),
            )
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(border)
            .title(Span::styled(format!(" {title} "), title_style));
        let inner = block.inner(panel);
        block.render(panel, buf);
        Rect {
            x: inner.x + 1,
            y: inner.y + 1,
            width: inner.width.saturating_sub(2),
            height: inner.height.saturating_sub(2),
        }
    }

    fn sidebar_frame(&self, buf: &mut Buffer, panel: Rect, title: &str, focused: bool) -> Rect {
        let glass = self
            .theme
            .faint()
            .bg(self.theme.surface)
            .remove_modifier(Modifier::all());
        for y in panel.y..panel.bottom() {
            for x in panel.x..panel.right() {
                let cell = &mut buf[(x, y)];
                if self.theme.mode == ColorMode::Mono {
                    cell.reset();
                } else {
                    cell.set_style(glass);
                }
            }
        }
        let (rule, title_style) = if focused {
            (
                self.theme.overlay_border(),
                Style::new()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                self.theme.table_border(),
                self.theme.muted().add_modifier(Modifier::BOLD),
            )
        };
        let edge = panel.right() - 1;
        for y in panel.y..panel.bottom() {
            buf[(edge, y)].set_symbol("│").set_style(rule);
        }
        let room = panel.width.saturating_sub(3) as usize;
        buf.set_stringn(panel.x + 1, panel.y, title, room, title_style);
        Rect {
            x: panel.x + 1,
            y: panel.y + 2,
            width: panel.width.saturating_sub(3),
            height: panel.height.saturating_sub(2),
        }
    }

    fn panel_item(
        &self,
        list_width: u16,
        prefix: &str,
        marker: &str,
        marker_style: Style,
        label: &str,
        label_style: Style,
    ) -> (ListItem<'static>, usize) {
        let lines = wrap_label(label, label_width(list_width, prefix, marker));
        let continuation = continuation_prefix(prefix);
        let text: Vec<Line<'static>> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::styled(prefix.to_string(), self.theme.muted()),
                        Span::styled(marker.to_string(), marker_style),
                        Span::styled(line.clone(), label_style),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(continuation.clone(), self.theme.muted()),
                        Span::raw("  "),
                        Span::styled(line.clone(), label_style),
                    ])
                }
            })
            .collect();
        let height = text.len().max(1);
        (ListItem::new(Text::from(text)), height)
    }

    fn draw_panel_list(
        buf: &mut Buffer,
        panel: &mut Panel,
        items: Vec<ListItem<'static>>,
        highlight: Style,
        focused: bool,
    ) {
        let list = List::new(items)
            .highlight_style(highlight)
            .highlight_symbol(if focused { "▎" } else { " " });
        StatefulWidget::render(list, panel.list_area, buf, &mut panel.list);
    }

    fn draw_outline(&mut self, buf: &mut Buffer, panel: Rect) {
        let focused = self.focus == Focus::Outline;
        let list_area = self.card(buf, panel, "Outline", focused);
        self.outline.list_area = list_area;
        if list_area.width == 0 || list_area.height == 0 {
            return;
        }
        if self.outline.rows.is_empty() {
            let room = list_area.width.saturating_sub(1) as usize;
            buf.set_stringn(
                list_area.x + 1,
                list_area.y,
                "No headings",
                room,
                self.theme.faint(),
            );
            return;
        }
        let (items, heights): (Vec<ListItem>, Vec<usize>) = self
            .outline
            .rows
            .iter()
            .map(|row| {
                let heading = &self.layout.headings[row.index];
                let marker_style = if row.collapsed {
                    Style::new().fg(self.theme.accent)
                } else {
                    self.theme.muted()
                };
                self.panel_item(
                    list_area.width,
                    &row.prefix,
                    row.marker,
                    marker_style,
                    &heading.text,
                    self.theme.heading_text(heading.level),
                )
            })
            .unzip();
        self.outline.heights = heights;
        let highlight = if focused {
            self.theme.selected().fg(self.theme.accent)
        } else {
            self.theme.selected()
        };
        Self::draw_panel_list(buf, &mut self.outline, items, highlight, focused);
    }

    fn draw_files(&mut self, buf: &mut Buffer, panel: Rect) {
        let focused = self.focus == Focus::Files;
        let name = self
            .project
            .as_ref()
            .and_then(|p| p.entries.first())
            .map(|root| root.name.clone())
            .unwrap_or_else(|| "Files".to_string());
        let list_area = self.sidebar_frame(buf, panel, &name, focused);
        self.files.list_area = list_area;
        if list_area.width == 0 || list_area.height == 0 {
            return;
        }
        let Some(project) = &self.project else { return };
        if self.files.rows.is_empty() {
            let room = list_area.width.saturating_sub(1) as usize;
            buf.set_stringn(
                list_area.x + 1,
                list_area.y,
                "No Markdown files",
                room,
                self.theme.faint(),
            );
            return;
        }
        let current = self.current_entry();
        let (items, heights): (Vec<ListItem>, Vec<usize>) = self
            .files
            .rows
            .iter()
            .map(|row| {
                let entry = &project.entries[row.index];
                let label = if self.files_titles && !entry.is_dir {
                    entry.title.clone().unwrap_or_else(|| entry.name.clone())
                } else {
                    entry.name.clone()
                };
                let style = if current == Some(row.index) {
                    Style::new()
                        .fg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if entry.is_dir {
                    self.theme.muted().add_modifier(Modifier::BOLD)
                } else {
                    self.theme.text()
                };
                let marker_style = if row.collapsed {
                    Style::new().fg(self.theme.accent)
                } else {
                    self.theme.muted()
                };
                self.panel_item(
                    list_area.width,
                    &row.prefix,
                    row.marker,
                    marker_style,
                    &label,
                    style,
                )
            })
            .unzip();
        self.files.heights = heights;
        let highlight = if focused {
            self.theme.selected().fg(self.theme.accent)
        } else {
            self.theme.selected()
        };
        Self::draw_panel_list(buf, &mut self.files, items, highlight, focused);
    }

    fn draw_matches(&self, buf: &mut Buffer, x: u16, y: u16, width: u16) {
        for (i, m) in self.matches.iter().enumerate() {
            if m.line < self.scroll || m.line >= self.scroll + self.view_height {
                continue;
            }
            let row = y + (m.line - self.scroll) as u16;
            let style = self.theme.search(self.current == Some(i));
            for col in m.start..m.end.min(width) {
                buf[(x + col, row)].set_style(style);
            }
        }
    }

    fn draw_status(&self, buf: &mut Buffer, area: Rect) {
        let style = self.theme.status();
        buf.set_style(area, style);
        let (chip, chip_style) = match self.mode {
            Mode::Search => (format!(" /{}▏ ", self.query), self.theme.search(false)),
            _ => (format!(" {} ", self.title()), self.theme.status_chip()),
        };
        let position = if self.layout.lines.len() <= self.view_height {
            "All".to_string()
        } else if self.scroll == 0 {
            "Top".to_string()
        } else if self.scroll >= self.max_scroll() {
            "Bot".to_string()
        } else {
            format!("{}%", self.scroll * 100 / self.max_scroll())
        };
        let middle = if let Some(notice) = &self.notice {
            notice.clone()
        } else if !self.query.is_empty() && self.mode != Mode::Search {
            let n = self.current.map(|i| i + 1).unwrap_or(0);
            format!("/{}  {}/{}", self.query, n, self.matches.len())
        } else {
            match self.focus {
                Focus::Outline => "outline: j/k follow  space fold  Tab back".to_string(),
                Focus::Files => "files: j/k move  Enter open  space fold  T titles".to_string(),
                Focus::Content => String::new(),
            }
        };
        let hints = if area.width >= 48 {
            "? help  q quit  "
        } else {
            ""
        };
        let right_width = (width(hints) + width(&position) + 1) as u16;
        let right_x = area.x + area.width.saturating_sub(right_width);
        let chip_room = right_x.saturating_sub(area.x + 1) as usize;
        let (after_chip, _) = buf.set_stringn(area.x, area.y, &chip, chip_room, chip_style);
        buf.set_string(right_x, area.y, hints, style.patch(self.theme.muted()));
        let position_x = right_x + width(hints) as u16;
        buf.set_string(
            position_x,
            area.y,
            &position,
            style.add_modifier(Modifier::BOLD),
        );
        if !middle.is_empty() {
            let mid_x = after_chip + 2;
            let free = right_x.saturating_sub(mid_x + 1) as usize;
            buf.set_stringn(
                mid_x,
                area.y,
                &middle,
                free,
                style.patch(self.theme.muted()),
            );
        }
    }

    fn draw_help(&mut self, buf: &mut Buffer, area: Rect) {
        let key_width = SECTIONS
            .iter()
            .flat_map(|s| s.bindings)
            .map(|(k, _)| width(&keys::plain(k)))
            .max()
            .unwrap_or(0);
        let mut lines: Vec<Line> = Vec::with_capacity(keys::help_rows());
        for (i, section) in SECTIONS.iter().enumerate() {
            if i > 0 {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(
                format!(" {}", section.title),
                self.theme.text().add_modifier(Modifier::BOLD),
            )));
            for (k, action) in section.bindings {
                let k = keys::plain(k);
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {k:<key_width$}  "),
                        Style::new().fg(self.theme.accent),
                    ),
                    Span::styled((*action).to_string(), self.theme.text()),
                ]));
            }
        }
        let w = (lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 3).min(area.width);
        let h = (lines.len() as u16 + 2).min(area.height);
        let popup = centered(area, w, h);
        let visible = popup.height.saturating_sub(2) as usize;
        let max_scroll = lines.len().saturating_sub(visible);
        self.help_scroll = self.help_scroll.min(max_scroll);
        let hint = if max_scroll > 0 {
            " j/k scroll · any other key closes "
        } else {
            " any key closes "
        };
        Clear.render(popup, buf);
        let block = self.popup_block(" Keys ").title_bottom(self.hint(hint));
        Paragraph::new(lines)
            .block(block)
            .scroll((self.help_scroll as u16, 0))
            .render(popup, buf);
    }

    fn draw_toc(&mut self, buf: &mut Buffer, area: Rect) {
        let items: Vec<ListItem> = self
            .layout
            .headings
            .iter()
            .map(|h| {
                let indent = "  ".repeat(h.level.saturating_sub(1) as usize);
                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(h.text.clone(), self.theme.heading_text(h.level)),
                ]))
            })
            .collect();
        let widest = self
            .layout
            .headings
            .iter()
            .map(|h| width(&h.text) + 2 * h.level as usize + 6)
            .max()
            .unwrap_or(20) as u16;
        let w = widest.max(32).min(area.width.saturating_sub(4).max(10));
        let h = (items.len() as u16 + 2).min(area.height.saturating_sub(2).max(3));
        let popup = centered(area, w, h);
        Clear.render(popup, buf);
        let block = self
            .popup_block(" Contents ")
            .title_bottom(self.hint(" Enter jump · Esc close "));
        let list = List::new(items)
            .block(block)
            .highlight_style(self.theme.selected())
            .highlight_symbol("▸ ");
        StatefulWidget::render(list, popup, buf, &mut self.toc);
    }

    fn popup_block(&self, title: &str) -> Block<'static> {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.theme.overlay_border())
            .title(Span::styled(
                title.to_string(),
                self.theme.text().add_modifier(Modifier::BOLD),
            ))
    }

    fn hint(&self, text: &str) -> Line<'static> {
        Line::from(Span::styled(text.to_string(), self.theme.faint())).right_aligned()
    }
}

fn label_width(list_width: u16, prefix: &str, marker: &str) -> usize {
    (list_width as usize)
        .saturating_sub(1 + width(prefix) + width(marker))
        .max(4)
}

fn wrap_label(label: &str, width: usize) -> Vec<String> {
    let lines: Vec<String> = wrap(&[Piece::text(label, Style::new())], width)
        .into_iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect();
    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

fn continuation_prefix(prefix: &str) -> String {
    if let Some(head) = prefix.strip_suffix("├ ") {
        format!("{head}│ ")
    } else if let Some(head) = prefix.strip_suffix("└ ") {
        format!("{head}  ")
    } else {
        prefix.to_string()
    }
}

fn gutters(width: u16) -> (u16, u16) {
    match width {
        0..=29 => (0, 1),
        30..=59 => (1, 1),
        _ => (2, 2),
    }
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}
