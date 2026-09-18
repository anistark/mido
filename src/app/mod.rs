mod draw;
mod finder;
mod keys;
mod outline;
mod search;
mod watch;

use std::collections::HashSet;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use ratatui::DefaultTerminal;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;

use crate::markdown::{Document, parse};
use crate::project::{Project, is_markdown};
use crate::render::layout::{Layout, layout};
use crate::render::theme::Theme;
use crate::render::wrap::width;
use search::Match;
use watch::FileWatcher;

const TAB_CHORD: Duration = Duration::from_secs(1);

pub enum Source {
    Stdin,
    File(PathBuf),
    Project {
        project: Project,
        file: Option<PathBuf>,
    },
}

impl Source {
    pub fn read(&self) -> io::Result<String> {
        match self {
            Source::File(path) => std::fs::read_to_string(path),
            Source::Project { project, file } => match file {
                Some(file) => std::fs::read_to_string(project.absolute(file)),
                None => Ok(String::new()),
            },
            Source::Stdin => {
                let mut text = String::new();
                io::stdin().read_to_string(&mut text)?;
                Ok(text)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    View,
    Help,
    Toc,
    Search,
    Finder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    Files,
    Content,
    Outline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Visit {
    file: Option<PathBuf>,
    scroll: usize,
}

struct Panel {
    list: ListState,
    tree: outline::Tree,
    collapsed: HashSet<usize>,
    rows: Vec<outline::Row>,
    area: Option<Rect>,
    list_area: Rect,
    heights: Vec<usize>,
    hide_root: bool,
}

impl Panel {
    fn new(hide_root: bool) -> Self {
        Self {
            list: ListState::default(),
            tree: outline::Tree::default(),
            collapsed: HashSet::new(),
            rows: Vec::new(),
            area: None,
            list_area: Rect::default(),
            heights: Vec::new(),
            hide_root,
        }
    }

    fn rebuild(&mut self, tree: outline::Tree) {
        self.tree = tree;
        self.collapsed.retain(|&i| self.tree.has_children(i));
        self.refresh();
    }

    fn refresh(&mut self) {
        self.rows = self.tree.rows(&self.collapsed);
        if self.hide_root {
            self.rows.retain(|r| r.index != 0);
        }
    }

    fn selected(&self) -> Option<usize> {
        self.list.selected().filter(|&r| r < self.rows.len())
    }

    fn row_for(&self, node: usize) -> Option<usize> {
        let shown = self.tree.visible_ancestor(node, &self.collapsed);
        self.rows.iter().position(|r| r.index == shown)
    }

    fn toggle(&mut self, row: usize) -> bool {
        let Some(r) = self.rows.get(row) else {
            return false;
        };
        if !r.has_children {
            return false;
        }
        let node = r.index;
        if !self.collapsed.remove(&node) {
            self.collapsed.insert(node);
        }
        self.refresh();
        self.list.select(self.row_for(node));
        true
    }

    fn fold_all(&mut self, fold: bool) {
        let keep = self.selected().map(|r| self.rows[r].index);
        let first = usize::from(self.hide_root);
        self.collapsed = if fold {
            (first..self.tree.len())
                .filter(|&i| self.tree.has_children(i))
                .collect()
        } else {
            HashSet::new()
        };
        self.refresh();
        if let Some(node) = keep {
            self.list.select(self.row_for(node));
        }
    }

    fn row_at(&self, mouse: &MouseEvent) -> Option<(usize, bool)> {
        let list = self.list_area;
        if mouse.row < list.y || mouse.row >= list.bottom() {
            return None;
        }
        let mut y = list.y;
        for row in self.list.offset()..self.rows.len() {
            let height = self.heights.get(row).copied().unwrap_or(1).max(1) as u16;
            if mouse.row < y + height {
                let r = &self.rows[row];
                let toggle_col = list.x + 1 + width(&r.prefix) as u16;
                let on_marker = r.has_children
                    && mouse.row == y
                    && (toggle_col..toggle_col + 2).contains(&mouse.column);
                return Some((row, on_marker));
            }
            y += height;
        }
        None
    }
}

pub struct App {
    project: Option<Project>,
    file: Option<PathBuf>,
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
    show_files: Option<bool>,
    show_outline: Option<bool>,
    focus_mode: bool,
    files: Panel,
    files_titles: bool,
    outline: Panel,
    outline_widest: usize,
    link: Option<usize>,
    back: Vec<Visit>,
    forward: Vec<Visit>,
    pending_anchor: Option<String>,
    content_origin: (u16, u16),
    content_cols: u16,
    query: String,
    search_origin: usize,
    matches: Vec<Match>,
    current: Option<usize>,
    toc: ListState,
    help_scroll: usize,
    finder: finder::Finder,
    watcher: Option<FileWatcher>,
    edit_request: bool,
    notice: Option<String>,
    quit: bool,
}

impl App {
    pub fn new(source: Source, text: &str, max_width: Option<usize>) -> Self {
        let (project, file) = match source {
            Source::Stdin => (None, None),
            Source::File(path) => (None, Some(path)),
            Source::Project { project, file } => {
                let file = file.map(|f| project.absolute(&f));
                (Some(project), file)
            }
        };
        let watcher = match (&project, &file) {
            (Some(project), _) => FileWatcher::tree(&project.root).ok(),
            (None, Some(path)) => FileWatcher::file(path).ok(),
            (None, None) => None,
        };
        let mut app = Self {
            project,
            file,
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
            show_files: None,
            show_outline: None,
            focus_mode: false,
            files: Panel::new(true),
            files_titles: false,
            outline: Panel::new(false),
            outline_widest: 0,
            link: None,
            back: Vec::new(),
            forward: Vec::new(),
            pending_anchor: None,
            content_origin: (0, 0),
            content_cols: 0,
            query: String::new(),
            search_origin: 0,
            matches: Vec::new(),
            current: None,
            toc: ListState::default(),
            help_scroll: 0,
            finder: finder::Finder::new(),
            watcher,
            edit_request: false,
            notice: None,
            quit: false,
        };
        app.rebuild_files();
        app
    }

    pub fn set_sidebar(&mut self, outline: Option<bool>) {
        self.show_outline = outline;
    }

    pub fn set_files(&mut self, files: Option<bool>) {
        self.show_files = files;
    }

    pub fn press(&mut self, code: KeyCode) {
        self.key(KeyEvent::from(code));
    }

    pub fn mouse_event(&mut self, mouse: MouseEvent) {
        self.mouse(mouse);
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.file.as_deref()
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
            let changed = self
                .watcher
                .as_ref()
                .map(FileWatcher::drain)
                .unwrap_or_default();
            if !changed.is_empty() {
                self.on_fs_change(&changed);
            }
            if self.edit_request {
                self.edit_request = false;
                self.run_editor(terminal)?;
            }
            if self.quit {
                return Ok(());
            }
        }
    }

    fn on_fs_change(&mut self, paths: &[PathBuf]) {
        let current = self.file.as_deref();
        let in_project = self.project.is_some();
        let same_file = |p: &Path| match current {
            Some(cur) if in_project => p == cur,
            Some(cur) => p.file_name() == cur.file_name(),
            None => false,
        };
        let touches_current = paths.iter().any(|p| same_file(p));
        let tree_changed = in_project && paths.iter().any(|p| !same_file(p) && is_markdown(p));
        if touches_current {
            self.reload();
        }
        if tree_changed {
            self.rescan_project();
        }
    }

    fn rescan_project(&mut self) {
        let Some(project) = &self.project else { return };
        let folded: HashSet<PathBuf> = self
            .files
            .collapsed
            .iter()
            .filter_map(|&i| project.entries.get(i).map(|e| e.path.clone()))
            .collect();
        let selected = self
            .files
            .selected()
            .and_then(|r| self.files.rows.get(r))
            .and_then(|r| project.entries.get(r.index).map(|e| e.path.clone()));
        let fresh = Project::scan(&project.root);
        self.files.collapsed = fresh
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| folded.contains(&e.path))
            .map(|(i, _)| i)
            .collect();
        let depths: Vec<usize> = fresh.entries.iter().map(|e| e.depth).collect();
        self.project = Some(fresh);
        self.files.rebuild(outline::Tree::from_depths(depths));
        let row = selected
            .and_then(|path| self.project.as_ref().and_then(|p| p.index_of(&path)))
            .and_then(|i| self.files.row_for(i));
        match row {
            Some(row) => self.files.list.select(Some(row)),
            None => self.select_current_file(),
        }
    }

    fn run_editor(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let Some(path) = self.file.clone() else {
            self.notice = Some("no file to edit".to_string());
            return Ok(());
        };
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());
        let mut parts = editor.split_whitespace();
        let Some(program) = parts.next() else {
            self.notice = Some("EDITOR is empty".to_string());
            return Ok(());
        };
        let args: Vec<&str> = parts.collect();
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
        let status = std::process::Command::new(program)
            .args(&args)
            .arg(&path)
            .status();
        *terminal = ratatui::try_init()?;
        execute!(io::stdout(), EnableMouseCapture)?;
        terminal.clear()?;
        self.notice = Some(match status {
            Ok(s) if s.success() => {
                self.reload();
                format!("edited with {program}")
            }
            Ok(s) => format!("{program} exited with {s}"),
            Err(err) => format!("cannot run {program}: {err}"),
        });
        Ok(())
    }

    fn open_finder(&mut self) {
        let Some(project) = &self.project else {
            self.notice = Some("open a folder to find files".to_string());
            return;
        };
        self.finder.open(project);
        self.mode = Mode::Finder;
    }

    fn key_finder(&mut self, key: KeyEvent, ctrl: bool) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::View,
            KeyCode::Enter => {
                self.mode = Mode::View;
                let target = self.finder.chosen().and_then(|i| {
                    self.project
                        .as_ref()
                        .map(|p| p.absolute(&p.entries[i].path))
                });
                if let Some(path) = target {
                    self.navigate(Some(path), None);
                    self.focus = Focus::Content;
                }
            }
            KeyCode::Down => self.finder.step(1),
            KeyCode::Up => self.finder.step(-1),
            KeyCode::Char('j') | KeyCode::Char('n') if ctrl => self.finder.step(1),
            KeyCode::Char('k') | KeyCode::Char('p') if ctrl => self.finder.step(-1),
            KeyCode::Backspace => {
                self.finder.query.pop();
                if let Some(project) = &self.project {
                    self.finder.search(project);
                }
            }
            KeyCode::Char(c) if !ctrl => {
                self.finder.query.push(c);
                if let Some(project) = &self.project {
                    self.finder.search(project);
                }
            }
            _ => {}
        }
    }

    fn title(&self) -> String {
        match (&self.project, &self.file) {
            (Some(project), Some(file)) => file
                .strip_prefix(&project.root)
                .unwrap_or(file)
                .display()
                .to_string(),
            (Some(project), None) => project
                .root
                .file_name()
                .map(|n| format!("{}/", n.to_string_lossy()))
                .unwrap_or_default(),
            (None, Some(file)) => file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            (None, None) => "stdin".to_string(),
        }
    }

    fn rebuild_files(&mut self) {
        let depths: Vec<usize> = self
            .project
            .as_ref()
            .map(|p| p.entries.iter().map(|e| e.depth).collect())
            .unwrap_or_default();
        self.files.rebuild(outline::Tree::from_depths(depths));
        self.select_current_file();
    }

    fn select_current_file(&mut self) {
        let index = match (&self.project, &self.file) {
            (Some(project), Some(file)) => file
                .strip_prefix(&project.root)
                .ok()
                .and_then(|rel| project.index_of(rel)),
            _ => None,
        };
        let row = index.and_then(|i| self.files.row_for(i));
        self.files.list.select(row);
    }

    fn current_entry(&self) -> Option<usize> {
        let project = self.project.as_ref()?;
        let rel = self.file.as_ref()?.strip_prefix(&project.root).ok()?;
        project.index_of(rel)
    }

    fn reload(&mut self) {
        let Some(path) = self.file.clone() else {
            self.notice = Some("nothing to reload".to_string());
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.doc = parse(&text);
                self.layout_width = 0;
                self.notice = Some("reloaded".to_string());
            }
            Err(err) => self.notice = Some(format!("reload failed: {err}")),
        }
    }

    fn open_path(&mut self, path: PathBuf) -> bool {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.doc = parse(&text);
                self.file = Some(path.clone());
                self.layout = Layout::default();
                self.layout_width = 0;
                self.scroll = 0;
                self.link = None;
                self.clear_search();
                self.outline.collapsed.clear();
                if self.project.is_none() {
                    self.watcher = FileWatcher::file(&path).ok();
                }
                self.select_current_file();
                true
            }
            Err(err) => {
                self.notice = Some(format!("cannot open {}: {err}", path.display()));
                false
            }
        }
    }

    fn visit(&self) -> Visit {
        Visit {
            file: self.file.clone(),
            scroll: self.scroll,
        }
    }

    fn navigate(&mut self, file: Option<PathBuf>, anchor: Option<String>) -> bool {
        let here = self.visit();
        if let Some(path) = file
            && self.file.as_deref() != Some(path.as_path())
            && !self.open_path(path)
        {
            return false;
        }
        self.back.push(here);
        self.forward.clear();
        self.link = None;
        self.pending_anchor = anchor;
        true
    }

    fn restore(&mut self, visit: Visit) {
        if visit.file != self.file
            && let Some(path) = visit.file.clone()
            && !self.open_path(path)
        {
            return;
        }
        self.scroll = visit.scroll;
        self.link = None;
    }

    fn go_back(&mut self) {
        let Some(visit) = self.back.pop() else {
            self.notice = Some("nothing to go back to".to_string());
            return;
        };
        let here = self.visit();
        self.forward.push(here);
        self.restore(visit);
    }

    fn go_forward(&mut self) {
        let Some(visit) = self.forward.pop() else {
            self.notice = Some("nothing to go forward to".to_string());
            return;
        };
        let here = self.visit();
        self.back.push(here);
        self.restore(visit);
    }

    fn follow(&mut self, url: &str) {
        if let Some(anchor) = url.strip_prefix('#') {
            self.navigate(None, Some(anchor.to_string()));
            return;
        }
        if url.contains("://") || url.starts_with("mailto:") || url.starts_with("tel:") {
            self.notice = Some(match open::that_detached(url) {
                Ok(()) => format!("opened {url}"),
                Err(err) => format!("cannot open {url}: {err}"),
            });
            return;
        }
        let (path_part, anchor) = match url.split_once('#') {
            Some((p, a)) => (p, Some(a.to_string())),
            None => (url, None),
        };
        let base = self
            .file
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut target = base.join(path_part);
        if target.is_dir() {
            let readme = target.join("README.md");
            if readme.is_file() {
                target = readme;
            } else {
                self.notice = Some(format!("{} is a folder", path_part));
                return;
            }
        }
        if !target.exists() {
            self.notice = Some(format!("not found: {path_part}"));
            return;
        }
        if !is_markdown(&target) {
            self.notice = Some(match open::that_detached(&target) {
                Ok(()) => format!("opened {}", target.display()),
                Err(err) => format!("cannot open {}: {err}", target.display()),
            });
            return;
        }
        let target = target.canonicalize().unwrap_or(target);
        self.navigate(Some(target), anchor);
    }

    fn select_link(&mut self, delta: isize) {
        if self.layout.links.is_empty() {
            self.notice = Some("no links".to_string());
            return;
        }
        let count = self.layout.links.len();
        let next = match self.link {
            Some(i) => (i as isize + delta).rem_euclid(count as isize) as usize,
            None if delta > 0 => self
                .layout
                .links
                .iter()
                .position(|l| l.line >= self.scroll)
                .unwrap_or(0),
            None => self
                .layout
                .links
                .iter()
                .rposition(|l| l.line < self.scroll + self.view_height)
                .unwrap_or(count - 1),
        };
        self.link = Some(next);
        let line = self.layout.links[next].line;
        if line < self.scroll || line >= self.scroll + self.view_height {
            self.scroll_to(line);
        }
    }

    fn open_entry(&mut self, row: usize) -> bool {
        let Some(r) = self.files.rows.get(row) else {
            return false;
        };
        let Some(project) = &self.project else {
            return false;
        };
        let entry = &project.entries[r.index];
        if entry.is_dir {
            self.files.toggle(row);
            return false;
        }
        let path = project.absolute(&entry.path);
        if self.file.as_deref() != Some(path.as_path()) {
            return self.navigate(Some(path), None);
        }
        true
    }

    fn ensure_layout(&mut self, content_width: usize) {
        if content_width == self.layout_width {
            return;
        }
        let anchor = self.layout.source_at(self.scroll);
        self.layout = layout(&self.doc, &self.theme, content_width);
        self.layout_width = content_width;
        self.link = None;
        let tree = outline::Tree::new(&self.layout.headings);
        self.outline_widest = tree
            .rows(&HashSet::new())
            .iter()
            .map(|r| width(&r.prefix) + 2 + width(&self.layout.headings[r.index].text))
            .max()
            .unwrap_or(0);
        self.outline.rebuild(tree);
        if let Some(anchor) = anchor {
            self.scroll = self.layout.line_for_source(anchor);
        }
        if !self.query.is_empty() {
            self.matches = search::find(&self.layout, &self.query);
            self.current = self.current.filter(|&i| i < self.matches.len());
        }
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

    fn panes(&self) -> Vec<Focus> {
        let mut panes = Vec::new();
        if self.files.area.is_some() {
            panes.push(Focus::Files);
        }
        panes.push(Focus::Content);
        if self.outline.area.is_some() {
            panes.push(Focus::Outline);
        }
        panes
    }

    fn set_focus(&mut self, focus: Focus) {
        match focus {
            Focus::Outline if self.outline.area.is_some() => {
                self.focus = Focus::Outline;
                if self.outline.list.selected().is_none() {
                    let row = self.active_heading().and_then(|h| self.outline.row_for(h));
                    self.outline.list.select(row.or(Some(0)));
                }
            }
            Focus::Files if self.files.area.is_some() => {
                self.focus = Focus::Files;
                if self.files.list.selected().is_none() {
                    self.files.list.select(Some(0));
                }
            }
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
        let panes = self.panes();
        let index = panes.iter().position(|&p| p == from).unwrap_or(0);
        let target = match code {
            KeyCode::Left => panes.get(index.wrapping_sub(1)),
            KeyCode::Right => panes.get(index + 1),
            _ => None,
        };
        self.set_focus(target.copied().unwrap_or(from));
    }

    fn toggle_focus_mode(&mut self) {
        self.focus_mode = !self.focus_mode;
        if self.focus_mode {
            self.focus = Focus::Content;
        }
    }

    fn toggle_files(&mut self) {
        if self.project.is_none() {
            self.notice = Some("open a folder to browse files".to_string());
            return;
        }
        if std::mem::take(&mut self.focus_mode) {
            self.show_files = Some(true);
            return;
        }
        let visible = self.files.area.is_some();
        self.show_files = Some(!visible);
        if visible && self.focus == Focus::Files {
            self.focus = Focus::Content;
        }
    }

    fn toggle_outline(&mut self) {
        if std::mem::take(&mut self.focus_mode) {
            self.show_outline = Some(true);
            return;
        }
        let visible = self.outline.area.is_some();
        self.show_outline = Some(!visible);
        if visible && self.focus == Focus::Outline {
            self.focus = Focus::Content;
        } else if !visible && self.layout.headings.is_empty() {
            self.notice = Some("no headings to outline".to_string());
        }
    }

    fn outline_move(&mut self, delta: isize) {
        if self.outline.rows.is_empty() {
            return;
        }
        let current = self
            .outline
            .selected()
            .or_else(|| self.active_heading().and_then(|h| self.outline.row_for(h)))
            .unwrap_or(0);
        let next = current
            .saturating_add_signed(delta)
            .min(self.outline.rows.len() - 1);
        self.outline_set(next);
    }

    fn outline_set(&mut self, row: usize) {
        if let Some(r) = self.outline.rows.get(row) {
            let line = self.layout.headings[r.index].line;
            self.outline.list.select(Some(row));
            self.scroll = line.min(self.max_scroll());
        }
    }

    fn files_move(&mut self, delta: isize) {
        if self.files.rows.is_empty() {
            return;
        }
        let current = self.files.selected().unwrap_or(0);
        let next = current
            .saturating_add_signed(delta)
            .min(self.files.rows.len() - 1);
        self.files.list.select(Some(next));
    }

    fn panel_collapse(&mut self, focus: Focus) {
        let panel = if focus == Focus::Files {
            &mut self.files
        } else {
            &mut self.outline
        };
        let Some(row) = panel.selected() else { return };
        let r = &panel.rows[row];
        if r.has_children && !r.collapsed {
            panel.toggle(row);
        } else if let Some(parent) = panel.tree.parent(r.index)
            && let Some(parent_row) = panel.row_for(parent)
        {
            if focus == Focus::Files {
                panel.list.select(Some(parent_row));
            } else {
                self.outline_set(parent_row);
            }
        }
    }

    fn panel_expand(&mut self, focus: Focus) -> bool {
        let panel = if focus == Focus::Files {
            &mut self.files
        } else {
            &mut self.outline
        };
        match panel.selected() {
            Some(row) if panel.rows[row].collapsed => panel.toggle(row),
            _ => false,
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
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
            Mode::View => match self.focus {
                Focus::Files => self.key_files(key, ctrl),
                Focus::Outline => self.key_outline(key, ctrl),
                Focus::Content => self.key_view(key, ctrl),
            },
            Mode::Help => self.key_help(key),
            Mode::Toc => self.key_toc(key),
            Mode::Search => self.key_search(key),
            Mode::Finder => self.key_finder(key, ctrl),
        }
    }

    fn key_view(&mut self, key: KeyEvent, ctrl: bool) {
        let page = self.view_height.max(1) as isize;
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc if self.link.is_some() => self.link = None,
            KeyCode::Esc if !self.query.is_empty() => self.clear_search(),
            KeyCode::Esc => self.quit = true,
            KeyCode::Enter if self.link.is_some() => {
                let url = self.layout.links[self.link.unwrap()].url.clone();
                self.follow(&url);
            }
            KeyCode::Char(']') => self.select_link(1),
            KeyCode::Char('[') => self.select_link(-1),
            KeyCode::Char('H') => self.go_back(),
            KeyCode::Char('L') => self.go_forward(),
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
            KeyCode::Char('b') => self.toggle_files(),
            KeyCode::Char('o') => self.toggle_outline(),
            KeyCode::Char('h') | KeyCode::Char('?') => {
                self.help_scroll = 0;
                self.mode = Mode::Help;
            }
            KeyCode::Char('l') if self.outline.area.is_some() => self.set_focus(Focus::Outline),
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
            KeyCode::Char('f') => self.toggle_focus_mode(),
            KeyCode::Char('p') if ctrl => self.open_finder(),
            KeyCode::Char('E') => self.edit_request = true,
            KeyCode::Char('r') => self.reload(),
            _ => {}
        }
    }

    fn key_outline(&mut self, key: KeyEvent, ctrl: bool) {
        let last = self.outline.rows.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.outline_move(1),
            KeyCode::Char('k') | KeyCode::Up => self.outline_move(-1),
            KeyCode::Char('g') | KeyCode::Home => self.outline_set(0),
            KeyCode::Char('G') | KeyCode::End => self.outline_set(last),
            KeyCode::Char(' ') => {
                if let Some(row) = self.outline.selected() {
                    self.outline.toggle(row);
                }
            }
            KeyCode::Left => self.panel_collapse(Focus::Outline),
            KeyCode::Char('l') | KeyCode::Right => {
                if !self.panel_expand(Focus::Outline) {
                    self.focus = Focus::Content;
                }
            }
            KeyCode::Char('-') => self.outline.fold_all(true),
            KeyCode::Char('+') | KeyCode::Char('=') => self.outline.fold_all(false),
            KeyCode::Enter | KeyCode::Esc => self.focus = Focus::Content,
            _ => self.key_view(key, ctrl),
        }
    }

    fn key_files(&mut self, key: KeyEvent, ctrl: bool) {
        let last = self.files.rows.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.files_move(1),
            KeyCode::Char('k') | KeyCode::Up => self.files_move(-1),
            KeyCode::Char('g') | KeyCode::Home => self.files.list.select(Some(0)),
            KeyCode::Char('G') | KeyCode::End => self.files.list.select(Some(last)),
            KeyCode::Char(' ') => {
                if let Some(row) = self.files.selected() {
                    self.open_entry(row);
                }
            }
            KeyCode::Enter => {
                if let Some(row) = self.files.selected()
                    && self.open_entry(row)
                {
                    self.focus = Focus::Content;
                }
            }
            KeyCode::Left => self.panel_collapse(Focus::Files),
            KeyCode::Char('l') | KeyCode::Right => {
                if !self.panel_expand(Focus::Files)
                    && let Some(row) = self.files.selected()
                    && self.open_entry(row)
                {
                    self.focus = Focus::Content;
                }
            }
            KeyCode::Char('-') => self.files.fold_all(true),
            KeyCode::Char('+') | KeyCode::Char('=') => self.files.fold_all(false),
            KeyCode::Char('T') => self.files_titles = !self.files_titles,
            KeyCode::Esc => self.focus = Focus::Content,
            _ => self.key_view(key, ctrl),
        }
    }

    fn key_help(&mut self, key: KeyEvent) {
        let last = keys::KEYS.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.help_scroll = (self.help_scroll + 1).min(last)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.help_scroll = self.help_scroll.saturating_sub(1)
            }
            KeyCode::Char('g') | KeyCode::Home => self.help_scroll = 0,
            KeyCode::Char('G') | KeyCode::End => self.help_scroll = last,
            _ => {
                self.help_scroll = 0;
                self.mode = Mode::View;
            }
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
        let at = Position::new(mouse.column, mouse.row);
        if self.outline.area.is_some_and(|r| r.contains(at)) {
            match mouse.kind {
                MouseEventKind::ScrollDown => self.outline_move(1),
                MouseEventKind::ScrollUp => self.outline_move(-1),
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((row, on_marker)) = self.outline.row_at(&mouse) {
                        if on_marker {
                            self.outline.toggle(row);
                        } else {
                            self.outline_set(row);
                        }
                        self.focus = Focus::Outline;
                    }
                }
                _ => {}
            }
        } else if self.files.area.is_some_and(|r| r.contains(at)) {
            match mouse.kind {
                MouseEventKind::ScrollDown => self.files_move(1),
                MouseEventKind::ScrollUp => self.files_move(-1),
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((row, on_marker)) = self.files.row_at(&mouse) {
                        self.files.list.select(Some(row));
                        if on_marker {
                            self.files.toggle(row);
                        } else {
                            self.open_entry(row);
                        }
                        self.focus = Focus::Files;
                    }
                }
                _ => {}
            }
        } else {
            match mouse.kind {
                MouseEventKind::ScrollDown => self.scroll_by(3),
                MouseEventKind::ScrollUp => self.scroll_by(-3),
                MouseEventKind::Down(MouseButton::Left) => {
                    let (x, y) = self.content_origin;
                    if mouse.column < x || mouse.row < y || mouse.column >= x + self.content_cols {
                        return;
                    }
                    let line = self.scroll + (mouse.row - y) as usize;
                    if let Some(link) = self.layout.link_at(line, mouse.column - x) {
                        let url = link.url.clone();
                        self.follow(&url);
                    }
                }
                _ => {}
            }
        }
    }
}
