mod keys;
mod outline;
mod search;
mod watch;

use std::collections::HashSet;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
};
use ratatui::{DefaultTerminal, Frame};

use crate::markdown::{Document, parse};
use crate::render::layout::{Layout, layout};
use crate::render::theme::{ColorMode, Theme};
use crate::render::wrap::width;
use keys::KEYS;
use search::Match;
use watch::FileWatcher;

const SIDEBAR_AUTO_MIN: u16 = 100;
const SIDEBAR_FORCED_MIN: u16 = 60;
const TAB_CHORD: Duration = Duration::from_secs(1);

pub enum Source {
    File(PathBuf),
    Stdin,
}

impl Source {
    pub fn read(&self) -> io::Result<String> {
        match self {
            Source::File(path) => std::fs::read_to_string(path),
            Source::Stdin => {
                let mut text = String::new();
                io::stdin().read_to_string(&mut text)?;
                Ok(text)
            }
        }
    }

    fn name(&self) -> String {
        match self {
            Source::File(path) => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Source::Stdin => "stdin".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    View,
    Help,
    Toc,
    Search,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Content,
    Sidebar,
}

pub struct App {
    source: Source,
    doc: Document,
    theme: Theme,
    max_width: Option<usize>,
    layout: Layout,
    layout_width: usize,
    scroll: usize,
    view_height: usize,
    mode: Mode,
    focus: Focus,
    tab_chord: Option<(Focus, Instant)>,
    sidebar: Option<bool>,
    sidebar_area: Option<Rect>,
    outline_list: Rect,
    outline: ListState,
    tree: outline::Tree,
    collapsed: HashSet<usize>,
    rows: Vec<outline::Row>,
    outline_widest: usize,
    query: String,
    search_origin: usize,
    matches: Vec<Match>,
    current: Option<usize>,
    toc: ListState,
    watcher: Option<FileWatcher>,
    notice: Option<String>,
    quit: bool,
}

impl App {
    pub fn new(source: Source, text: &str, max_width: Option<usize>) -> Self {
        let watcher = match &source {
            Source::File(path) => FileWatcher::new(path).ok(),
            Source::Stdin => None,
        };
        Self {
            source,
            doc: parse(text),
            theme: Theme::for_terminal(),
            max_width,
            layout: Layout::default(),
            layout_width: 0,
            scroll: 0,
            view_height: 1,
            mode: Mode::View,
            focus: Focus::Content,
            tab_chord: None,
            sidebar: None,
            sidebar_area: None,
            outline_list: Rect::default(),
            outline: ListState::default(),
            tree: outline::Tree::default(),
            collapsed: HashSet::new(),
            rows: Vec::new(),
            outline_widest: 0,
            query: String::new(),
            search_origin: 0,
            matches: Vec::new(),
            current: None,
            toc: ListState::default(),
            watcher,
            notice: None,
            quit: false,
        }
    }

    pub fn set_sidebar(&mut self, sidebar: Option<bool>) {
        self.sidebar = sidebar;
    }

    pub fn press(&mut self, code: KeyCode) {
        self.key(KeyEvent::from(code));
    }

    pub fn run(mut self) -> Result<()> {
        let mut terminal = ratatui::try_init()?;
        execute!(io::stdout(), EnableMouseCapture)?;
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            hook(info);
        }));
        let result = self.event_loop(&mut terminal);
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
        result
    }

    fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            if crossterm::event::poll(Duration::from_millis(100))? {
                match crossterm::event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => self.key(key),
                    Event::Mouse(mouse) => self.mouse(mouse),
                    _ => {}
                }
            }
            if self
                .tab_chord
                .is_some_and(|(_, at)| at.elapsed() > TAB_CHORD)
            {
                self.tab_chord = None;
            }
            if self.watcher.as_ref().is_some_and(FileWatcher::changed) {
                self.reload();
            }
            if self.quit {
                return Ok(());
            }
        }
    }

    fn reload(&mut self) {
        match self.source.read() {
            Ok(text) => {
                self.doc = parse(&text);
                self.layout_width = 0;
                self.notice = Some("reloaded".to_string());
            }
            Err(err) => self.notice = Some(format!("reload failed: {err}")),
        }
    }

    fn ensure_layout(&mut self, content_width: usize) {
        if content_width == self.layout_width {
            return;
        }
        let anchor = self.layout.source_at(self.scroll);
        self.layout = layout(&self.doc, &self.theme, content_width);
        self.layout_width = content_width;
        self.tree = outline::Tree::new(&self.layout.headings);
        self.collapsed.retain(|&i| self.tree.has_children(i));
        self.outline_widest = self
            .tree
            .rows(&HashSet::new())
            .iter()
            .map(|r| width(&r.prefix) + 2 + width(&self.layout.headings[r.index].text))
            .max()
            .unwrap_or(0);
        self.refresh_rows();
        if let Some(anchor) = anchor {
            self.scroll = self.layout.line_for_source(anchor);
        }
        if !self.query.is_empty() {
            self.matches = search::find(&self.layout, &self.query);
            self.current = self.current.filter(|&i| i < self.matches.len());
        }
    }

    fn refresh_rows(&mut self) {
        self.rows = self.tree.rows(&self.collapsed);
    }

    fn row_for_heading(&self, heading: usize) -> Option<usize> {
        let shown = self.tree.visible_ancestor(heading, &self.collapsed);
        self.rows.iter().position(|r| r.index == shown)
    }

    fn max_scroll(&self) -> usize {
        self.layout.lines.len().saturating_sub(self.view_height)
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.max_scroll());
    }

    fn scroll_by(&mut self, delta: isize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(delta)
            .min(self.max_scroll());
    }

    fn scroll_to(&mut self, line: usize) {
        self.scroll = line
            .saturating_sub(self.view_height / 3)
            .min(self.max_scroll());
    }

    fn active_heading(&self) -> Option<usize> {
        let cutoff = if self.scroll >= self.max_scroll() {
            self.layout.lines.len()
        } else {
            self.scroll + 1
        };
        self.layout.headings.iter().rposition(|h| h.line < cutoff)
    }

    fn sidebar_visible(&self, body_width: u16) -> bool {
        match self.sidebar {
            Some(false) => false,
            Some(true) => body_width >= SIDEBAR_FORCED_MIN,
            None => body_width >= SIDEBAR_AUTO_MIN,
        }
    }

    fn toggle_sidebar(&mut self) {
        let visible = self.sidebar_area.is_some();
        self.sidebar = Some(!visible);
        if visible {
            self.focus = Focus::Content;
        } else if self.layout.headings.is_empty() {
            self.notice = Some("no headings to outline".to_string());
        }
    }

    fn focus_sidebar(&mut self) {
        self.focus = Focus::Sidebar;
        if self.outline.selected().is_none() {
            let row = self.active_heading().and_then(|h| self.row_for_heading(h));
            self.outline.select(row.or(Some(0)));
        }
    }

    fn panes(&self) -> Vec<Focus> {
        let mut panes = vec![Focus::Content];
        if self.sidebar_area.is_some() {
            panes.push(Focus::Sidebar);
        }
        panes
    }

    fn set_focus(&mut self, focus: Focus) {
        match focus {
            Focus::Sidebar if self.sidebar_area.is_some() => self.focus_sidebar(),
            _ => self.focus = Focus::Content,
        }
    }

    fn cycle_focus(&mut self, delta: isize) {
        let panes = self.panes();
        if panes.len() < 2 {
            self.notice = Some("no other pane".to_string());
            return;
        }
        let from = self.focus;
        let index = panes.iter().position(|&p| p == from).unwrap_or(0) as isize;
        let next = (index + delta).rem_euclid(panes.len() as isize) as usize;
        self.set_focus(panes[next]);
        self.tab_chord = Some((from, Instant::now()));
    }

    fn focus_direction(&mut self, from: Focus, code: KeyCode) {
        let target = match (from, code) {
            (Focus::Content, KeyCode::Right) => Focus::Sidebar,
            (Focus::Sidebar, KeyCode::Left) => Focus::Content,
            (pane, _) => pane,
        };
        self.set_focus(target);
    }

    fn selected_row(&self) -> Option<usize> {
        self.outline.selected().filter(|&r| r < self.rows.len())
    }

    fn outline_move(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let current = self
            .selected_row()
            .or_else(|| self.active_heading().and_then(|h| self.row_for_heading(h)))
            .unwrap_or(0);
        let next = current
            .saturating_add_signed(delta)
            .min(self.rows.len() - 1);
        self.outline_set(next);
    }

    fn outline_set(&mut self, row: usize) {
        if let Some(r) = self.rows.get(row) {
            let line = self.layout.headings[r.index].line;
            self.outline.select(Some(row));
            self.scroll = line.min(self.max_scroll());
        }
    }

    fn outline_toggle(&mut self, row: usize) {
        let Some(r) = self.rows.get(row) else { return };
        if !r.has_children {
            return;
        }
        let heading = r.index;
        if !self.collapsed.remove(&heading) {
            self.collapsed.insert(heading);
        }
        self.refresh_rows();
        self.outline.select(self.row_for_heading(heading));
    }

    fn outline_collapse(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let r = &self.rows[row];
        if r.has_children && !r.collapsed {
            self.outline_toggle(row);
        } else if let Some(parent) = self.tree.parent(r.index)
            && let Some(parent_row) = self.row_for_heading(parent)
        {
            self.outline_set(parent_row);
        }
    }

    fn outline_expand(&mut self) -> bool {
        let Some(row) = self.selected_row() else {
            return false;
        };
        if self.rows[row].collapsed {
            self.outline_toggle(row);
            true
        } else {
            false
        }
    }

    fn outline_fold_all(&mut self, fold: bool) {
        let keep = self.selected_row().map(|r| self.rows[r].index);
        self.collapsed = if fold {
            (0..self.tree.len())
                .filter(|&i| self.tree.has_children(i))
                .collect()
        } else {
            HashSet::new()
        };
        self.refresh_rows();
        if let Some(heading) = keep {
            self.outline.select(self.row_for_heading(heading));
        }
    }

    fn key(&mut self, key: KeyEvent) {
        self.notice = None;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        let chord = self
            .tab_chord
            .take()
            .filter(|(_, at)| at.elapsed() <= TAB_CHORD);
        match self.mode {
            Mode::View if key.code == KeyCode::Tab => self.cycle_focus(1),
            Mode::View if key.code == KeyCode::BackTab => self.cycle_focus(-1),
            Mode::View if matches!(key.code, KeyCode::Left | KeyCode::Right) && chord.is_some() => {
                self.focus_direction(chord.unwrap().0, key.code)
            }
            Mode::View if self.focus == Focus::Sidebar => self.key_sidebar(key, ctrl),
            Mode::View => self.key_view(key, ctrl),
            Mode::Help => self.mode = Mode::View,
            Mode::Toc => self.key_toc(key),
            Mode::Search => self.key_search(key),
        }
    }

    fn key_view(&mut self, key: KeyEvent, ctrl: bool) {
        let page = self.view_height.max(1) as isize;
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc if !self.query.is_empty() => self.clear_search(),
            KeyCode::Esc => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => self.scroll_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_by(-1),
            KeyCode::Char('d') if ctrl => self.scroll_by(page / 2),
            KeyCode::Char('u') if ctrl => self.scroll_by(-page / 2),
            KeyCode::Char('f') if ctrl => self.scroll_by(page),
            KeyCode::Char('b') if ctrl => self.scroll_by(-page),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll_by(page),
            KeyCode::PageUp => self.scroll_by(-page),
            KeyCode::Char('g') | KeyCode::Home => self.scroll = 0,
            KeyCode::Char('G') | KeyCode::End => self.scroll = self.max_scroll(),
            KeyCode::Char('b') => self.toggle_sidebar(),
            KeyCode::Char('h') if self.sidebar_area.is_some() => self.focus_sidebar(),
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.search_origin = self.scroll;
                self.query.clear();
                self.matches.clear();
                self.current = None;
            }
            KeyCode::Char('n') => self.step_match(1),
            KeyCode::Char('N') => self.step_match(-1),
            KeyCode::Char('t') => self.open_toc(),
            KeyCode::Char('r') => self.reload(),
            KeyCode::Char('?') => self.mode = Mode::Help,
            _ => {}
        }
    }

    fn key_sidebar(&mut self, key: KeyEvent, ctrl: bool) {
        let last = self.rows.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.outline_move(1),
            KeyCode::Char('k') | KeyCode::Up => self.outline_move(-1),
            KeyCode::Char('g') | KeyCode::Home => self.outline_set(0),
            KeyCode::Char('G') | KeyCode::End => self.outline_set(last),
            KeyCode::Char(' ') | KeyCode::Char('o') => {
                if let Some(row) = self.selected_row() {
                    self.outline_toggle(row);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => self.outline_collapse(),
            KeyCode::Char('l') | KeyCode::Right => {
                if !self.outline_expand() {
                    self.focus = Focus::Content;
                }
            }
            KeyCode::Char('-') => self.outline_fold_all(true),
            KeyCode::Char('+') | KeyCode::Char('=') => self.outline_fold_all(false),
            KeyCode::Enter | KeyCode::Esc => self.focus = Focus::Content,
            _ => self.key_view(key, ctrl),
        }
    }

    fn key_toc(&mut self, key: KeyEvent) {
        let count = self.layout.headings.len();
        let selected = self.toc.selected().unwrap_or(0);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('t') => self.mode = Mode::View,
            KeyCode::Char('j') | KeyCode::Down => {
                self.toc.select(Some((selected + 1).min(count - 1)))
            }
            KeyCode::Char('k') | KeyCode::Up => self.toc.select(Some(selected.saturating_sub(1))),
            KeyCode::Char('g') | KeyCode::Home => self.toc.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End => self.toc.select(Some(count - 1)),
            KeyCode::Enter => {
                let line = self.layout.headings[selected].line;
                self.scroll = line.min(self.max_scroll());
                self.mode = Mode::View;
            }
            _ => {}
        }
    }

    fn key_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.clear_search();
                self.scroll = self.search_origin;
                self.mode = Mode::View;
            }
            KeyCode::Enter => {
                self.mode = Mode::View;
                if !self.query.is_empty() && self.matches.is_empty() {
                    self.notice = Some("no matches".to_string());
                }
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.update_search();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.push(c);
                self.update_search();
            }
            _ => {}
        }
    }

    fn update_search(&mut self) {
        self.matches = if self.query.is_empty() {
            Vec::new()
        } else {
            search::find(&self.layout, &self.query)
        };
        self.current = self
            .matches
            .iter()
            .position(|m| m.line >= self.search_origin)
            .or(if self.matches.is_empty() {
                None
            } else {
                Some(0)
            });
        match self.current {
            Some(i) => self.scroll_to(self.matches[i].line),
            None => self.scroll = self.search_origin,
        }
    }

    fn clear_search(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.current = None;
    }

    fn step_match(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as isize;
        let next = match self.current {
            Some(i) => (i as isize + delta).rem_euclid(len),
            None => 0,
        } as usize;
        self.current = Some(next);
        self.scroll_to(self.matches[next].line);
    }

    fn open_toc(&mut self) {
        if self.layout.headings.is_empty() {
            self.notice = Some("no headings".to_string());
            return;
        }
        self.toc.select(self.active_heading().or(Some(0)));
        self.mode = Mode::Toc;
    }

    fn mouse(&mut self, mouse: MouseEvent) {
        let Some(panel) = self.sidebar_area else {
            return self.mouse_content(mouse);
        };
        if !panel.contains(Position::new(mouse.column, mouse.row)) {
            return self.mouse_content(mouse);
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => self.outline_move(1),
            MouseEventKind::ScrollUp => self.outline_move(-1),
            MouseEventKind::Down(MouseButton::Left) => {
                let list = self.outline_list;
                if mouse.row < list.y || mouse.row >= list.bottom() {
                    return;
                }
                let row = (mouse.row - list.y) as usize + self.outline.offset();
                let Some(r) = self.rows.get(row) else { return };
                let toggle_col = list.x + 1 + width(&r.prefix) as u16;
                if r.has_children && (toggle_col..toggle_col + 2).contains(&mouse.column) {
                    self.outline_toggle(row);
                } else {
                    self.outline_set(row);
                }
                self.focus = Focus::Sidebar;
            }
            _ => {}
        }
    }

    fn mouse_content(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.scroll_by(3),
            MouseEventKind::ScrollUp => self.scroll_by(-3),
            _ => {}
        }
    }

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
        let panel =
            (self.sidebar_visible(body.width) && body.height >= 5).then(|| self.panel_rect(body));
        self.sidebar_area = panel;
        if panel.is_none() {
            self.focus = Focus::Content;
        }
        let content_area = match panel {
            Some(p) => Rect {
                width: p.x - body.x,
                ..body
            },
            None => body,
        };
        let content_width = self.content_width(content_area.width);
        self.ensure_layout(content_width);
        self.view_height = content_area.height as usize;
        self.clamp_scroll();
        if self.focus == Focus::Content {
            let row = self.active_heading().and_then(|h| self.row_for_heading(h));
            self.outline.select(row);
        }

        let (left, right) = gutters(content_area.width);
        let available = (content_area.width - left - right) as usize;
        let x = content_area.x + left + ((available - content_width) / 2) as u16;
        let buf = frame.buffer_mut();
        if self.layout.lines.is_empty() {
            let notice = "Nothing to show, the file is empty.";
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

        if self.layout.lines.len() > self.view_height {
            let mut state = ScrollbarState::new(self.layout.lines.len())
                .position(self.scroll)
                .viewport_content_length(self.view_height);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(self.theme.rule())
                .thumb_style(self.theme.faint())
                .render(body, buf, &mut state);
        }

        if let Some(panel) = panel {
            self.draw_outline(buf, panel);
        }
        self.draw_status(buf, status_area);
        match self.mode {
            Mode::Help => self.draw_help(buf, area),
            Mode::Toc => self.draw_toc(buf, area),
            _ => {}
        }
    }

    fn content_width(&self, area_width: u16) -> usize {
        let (left, right) = gutters(area_width);
        let available = area_width.saturating_sub(left + right) as usize;
        self.max_width
            .map_or(available, |cap| available.min(cap))
            .max(4)
    }

    fn panel_rect(&self, body: Rect) -> Rect {
        let widest = self.outline_widest.max(11) as u16;
        let w = (widest + 7).clamp(18, 46).min(body.width / 3).max(14);
        let rows = self.rows.len().max(1) as u16;
        let h = (rows + 4)
            .clamp(5, body.height.saturating_sub(2).max(5))
            .min(body.height);
        Rect {
            x: body.right().saturating_sub(w + 2),
            y: body.y + 1,
            width: w,
            height: h,
        }
    }

    fn draw_outline(&mut self, buf: &mut Buffer, panel: Rect) {
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
        let focused = self.focus == Focus::Sidebar;
        let (border, title) = if focused {
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
            .title(Span::styled(" Outline ", title));
        let inner = block.inner(panel);
        block.render(panel, buf);
        let list_area = Rect {
            x: inner.x + 1,
            y: inner.y + 1,
            width: inner.width.saturating_sub(2),
            height: inner.height.saturating_sub(2),
        };
        self.outline_list = list_area;
        if list_area.width == 0 || list_area.height == 0 {
            return;
        }
        if self.rows.is_empty() {
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
        let items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|row| {
                let heading = &self.layout.headings[row.index];
                let marker_style = if row.collapsed {
                    Style::new().fg(self.theme.accent)
                } else {
                    self.theme.muted()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(row.prefix.clone(), self.theme.muted()),
                    Span::styled(row.marker, marker_style),
                    Span::styled(heading.text.clone(), self.theme.heading_text(heading.level)),
                ]))
            })
            .collect();
        let highlight = if focused {
            self.theme.selected().fg(self.theme.accent)
        } else {
            self.theme.selected()
        };
        let list = List::new(items)
            .highlight_style(highlight)
            .highlight_symbol(if focused { "▎" } else { " " });
        StatefulWidget::render(list, list_area, buf, &mut self.outline);
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
            _ => (
                format!(" {} ", self.source.name()),
                self.theme.status_chip(),
            ),
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
        } else if self.focus == Focus::Sidebar {
            "outline: j/k follow  space fold  Tab back".to_string()
        } else {
            String::new()
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

    fn draw_help(&self, buf: &mut Buffer, area: Rect) {
        let key_width = KEYS.iter().map(|(k, _)| width(k)).max().unwrap_or(0);
        let lines: Vec<Line> = KEYS
            .iter()
            .map(|(k, action)| {
                Line::from(vec![
                    Span::styled(
                        format!(" {k:<key_width$}  "),
                        Style::new().fg(self.theme.accent),
                    ),
                    Span::styled((*action).to_string(), self.theme.text()),
                ])
            })
            .collect();
        let w = (lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 3).min(area.width);
        let h = (lines.len() as u16 + 2).min(area.height);
        let popup = centered(area, w, h);
        Clear.render(popup, buf);
        let block = self
            .popup_block(" Keys ")
            .title_bottom(self.hint(" any key closes "));
        Paragraph::new(lines).block(block).render(popup, buf);
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
