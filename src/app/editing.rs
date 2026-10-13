use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};

use super::keys::Action;
use super::watch::FileWatcher;
use super::{App, Focus, Mode};
use crate::edit::file::{self, EditFile, Eol};
use crate::edit::{self, Command, Editor, Look, Motion, assist};
use crate::markdown::parse;
use crate::render::source::{Run, SourceHighlighter};
use crate::render::theme::{ColorMode, Theme};

/// A question the editor waits on before it goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Prompt {
    /// Leaving the editor with unsaved changes.
    Leave,
    /// Saving over a file that changed on disk since it was opened. `leave` closes the
    /// editor after the save.
    Overwrite { leave: bool },
}

#[derive(Debug, Default)]
pub(super) struct Find {
    pub query: String,
    pub typing: bool,
    pub matches: Vec<(usize, usize, usize)>,
    pub current: Option<usize>,
    origin: (usize, usize),
    version: Option<u64>,
}

pub(super) struct Session {
    pub editor: Box<dyn Editor>,
    pub file: EditFile,
    pub find: Find,
    saved: u64,
    dirty: (u64, bool),
    highlighter: SourceHighlighter,
    runs: (Option<u64>, Vec<Vec<Run>>),
    pub pending_line: Option<usize>,
    pub anchor: Option<(usize, usize)>,
}

fn digest(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

impl Session {
    fn new(file: EditFile, lines: Vec<String>) -> Self {
        let editor = edit::new_editor(lines);
        let saved = digest(editor.lines());
        Self {
            dirty: (editor.version(), false),
            editor,
            file,
            find: Find::default(),
            saved,
            highlighter: SourceHighlighter::default(),
            runs: (None, Vec::new()),
            pending_line: None,
            anchor: None,
        }
    }

    pub fn dirty(&mut self) -> bool {
        let version = self.editor.version();
        if self.dirty.0 != version {
            self.dirty = (version, digest(self.editor.lines()) != self.saved);
        }
        self.dirty.1
    }

    fn mark_saved(&mut self) {
        self.saved = digest(self.editor.lines());
        self.dirty = (self.editor.version(), false);
    }

    /// Paints the buffer with its syntax colors, search hits and selection, and returns the
    /// cursor cell.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) -> Option<Position> {
        let version = self.editor.version();
        if self.runs.0 != Some(version) {
            let runs = self.highlighter.highlight(self.editor.lines(), theme);
            self.runs = (Some(version), runs);
        }
        self.refresh_find();
        let mono = theme.mode == ColorMode::Mono;
        let look = Look {
            base: theme.text(),
            cursor_line: if mono {
                Style::new()
            } else {
                Style::new().bg(theme.surface)
            },
            selection: if mono {
                Style::new().add_modifier(Modifier::REVERSED)
            } else {
                Style::new().bg(theme.selected_bg)
            },
            search: theme.search(false),
            current: theme.search(true),
            runs: &self.runs.1,
            matches: &self.find.matches,
            current_match: self.find.current,
        };
        let mut cursor = self.editor.render(area, buf, &look);
        if let Some(line) = self.pending_line.take() {
            self.editor.scroll_to(line);
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    buf[(x, y)].set_symbol(" ");
                }
            }
            cursor = self.editor.render(area, buf, &look);
        }
        cursor
    }

    pub fn position(&self) -> (usize, usize) {
        self.editor.cursor()
    }

    pub fn refresh_find(&mut self) {
        let version = self.editor.version();
        if self.find.version == Some(version) {
            return;
        }
        self.find.version = Some(version);
        self.find.matches = find(self.editor.lines(), &self.find.query);
        if self
            .find
            .current
            .is_some_and(|i| i >= self.find.matches.len())
        {
            self.find.current = None;
        }
    }

    pub fn name(&self) -> String {
        self.file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn encoded(&self) -> String {
        file::encode(self.file.format, self.editor.lines())
    }
}

/// Smart-case literal matches as (line, start char, end char), like the reader's search.
pub(super) fn find(lines: &[String], query: &str) -> Vec<(usize, usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let fold = !query.chars().any(char::is_uppercase);
    let needle: Vec<char> = if fold {
        query.to_lowercase().chars().collect()
    } else {
        query.chars().collect()
    };
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let hay: Vec<char> = if fold {
            line.chars()
                .map(|c| c.to_lowercase().next().unwrap_or(c))
                .collect()
        } else {
            line.chars().collect()
        };
        let mut at = 0;
        while at + needle.len() <= hay.len() {
            if hay[at..at + needle.len()] == needle[..] {
                out.push((row, at, at + needle.len()));
                at += needle.len();
            } else {
                at += 1;
            }
        }
    }
    out
}

impl App {
    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    /// Opens the current file in the editor, at the part of it on screen.
    pub fn start_editing(&mut self) {
        if self.edit.is_some() {
            self.focus = Focus::Editor;
            return;
        }
        let Some(path) = self.file.clone() else {
            self.notice = Some(if self.project.is_some() {
                "no file to edit".to_string()
            } else {
                "stdin cannot be edited, open a file".to_string()
            });
            return;
        };
        let (file, lines) = match EditFile::open(&path) {
            Ok(opened) => opened,
            Err(err) => {
                self.notice = Some(format!("cannot edit {}: {err}", path.display()));
                return;
            }
        };
        let line = self
            .layout
            .source_at(self.scroll)
            .map(|offset| line_at(&lines, file.format.eol, file.format.bom, offset))
            .unwrap_or(0);
        self.notice = Some(if !file.exists() {
            "new file, Ctrl-s creates it".to_string()
        } else if file.recovery_path().exists() {
            format!(
                "unsaved work from a crash is in {}",
                file.recovery_path().display()
            )
        } else if file.format.mixed {
            "mixed line endings, saving will use the more common one".to_string()
        } else {
            String::new()
        })
        .filter(|n| !n.is_empty());
        let mut session = Session::new(file, lines);
        session.pending_line = Some(line);
        self.edit = Some(session);
        self.focus = Focus::Editor;
        self.link = None;
        self.clear_search();
    }

    /// Leaves the editor. Unsaved changes are kept in the reader unless `discard` is set,
    /// in which case the file is read again from disk.
    pub(super) fn close_editor(&mut self, discard: bool) {
        let Some(session) = self.edit.take() else {
            return;
        };
        let lines = session.editor.lines();
        let top = session.editor.top();
        let offset: usize = lines[..top.min(lines.len())]
            .iter()
            .map(|l| l.len() + 1)
            .sum();
        if discard {
            self.reload();
            self.notice = Some("changes discarded".to_string());
        } else {
            self.doc = parse(&session.editor.text());
            self.layout_width = 0;
        }
        self.pending_source = Some(offset);
        self.mode = Mode::View;
        self.focus = Focus::Content;
    }

    pub(super) fn leave_editor(&mut self) {
        let Some(session) = &mut self.edit else {
            return;
        };
        if session.dirty() {
            self.mode = Mode::Prompt(Prompt::Leave);
        } else {
            self.close_editor(false);
        }
    }

    /// Saves the buffer. Without `force`, a file that changed on disk asks first.
    pub(super) fn save(&mut self, force: bool, leave: bool) {
        let Some(session) = &mut self.edit else {
            return;
        };
        if !force && session.file.changed_on_disk() {
            self.mode = Mode::Prompt(Prompt::Overwrite { leave });
            return;
        }
        let created = !session.file.exists();
        let mixed = session.file.format.mixed;
        let lines = session.editor.lines().to_vec();
        match session.file.save(&lines) {
            Ok(()) => {
                session.mark_saved();
                let name = session.name();
                let eol = match session.file.format.eol {
                    Eol::Lf => "LF",
                    Eol::CrLf => "CRLF",
                };
                self.notice = Some(match (created, mixed) {
                    (true, _) => format!("created {name}"),
                    (false, true) => format!("saved {name} with {eol} line endings"),
                    (false, false) => format!("saved {name}"),
                });
                let _ = std::fs::remove_file(session.file.recovery_path());
                if created && self.project.is_none() {
                    self.watcher = FileWatcher::file(&session.file.path).ok();
                }
                self.mode = Mode::View;
                if leave {
                    self.close_editor(false);
                }
            }
            Err(err) => {
                self.notice = Some(format!("cannot save: {err}"));
                self.mode = Mode::View;
            }
        }
    }

    /// Throws the edits away and reads the file again, keeping the editor open.
    fn revert(&mut self) {
        let Some(session) = &self.edit else { return };
        let path = session.file.path.clone();
        let cursor = session.editor.cursor();
        let top = session.editor.top();
        match EditFile::open(&path) {
            Ok((file, lines)) => {
                let mut fresh = Session::new(file, lines);
                fresh.editor.run(Command::Jump(cursor.0, cursor.1));
                fresh.pending_line = Some(top);
                self.edit = Some(fresh);
                self.notice = Some("read the file again from disk".to_string());
            }
            Err(err) => self.notice = Some(format!("cannot read {}: {err}", path.display())),
        }
        self.mode = Mode::View;
    }

    pub(super) fn key_prompt(&mut self, prompt: Prompt, key: KeyEvent) {
        let c = match key.code {
            KeyCode::Char(c) => c.to_ascii_lowercase(),
            KeyCode::Esc => 'c',
            _ => return,
        };
        match (prompt, c) {
            (Prompt::Leave, 's' | 'y') => self.save(false, true),
            (Prompt::Leave, 'd' | 'n') => self.close_editor(true),
            (Prompt::Overwrite { leave }, 'o' | 'y') => self.save(true, leave),
            (Prompt::Overwrite { .. }, 'r') => self.revert(),
            (_, 'c') => self.mode = Mode::View,
            _ => {}
        }
    }

    pub(super) fn prompt_text(&self, prompt: Prompt) -> (String, String) {
        let name = self.edit.as_ref().map(Session::name).unwrap_or_default();
        let dot = self.theme.glyphs.dot;
        match prompt {
            Prompt::Leave => (
                format!("{name} has unsaved changes."),
                format!("s save {dot} d discard {dot} Esc keep editing"),
            ),
            Prompt::Overwrite { .. } => (
                format!("{name} changed on disk since it was opened."),
                format!("o overwrite {dot} r reload from disk {dot} Esc cancel"),
            ),
        }
    }

    /// Notes a change to the file on disk. The buffer is never replaced under the editor,
    /// and saving asks before overwriting.
    pub(super) fn disk_changed_while_editing(&mut self) {
        if let Some(session) = &self.edit
            && session.file.changed_on_disk()
        {
            self.notice = Some(format!("{} changed on disk", session.name()));
        }
    }

    pub(super) fn key_edit(&mut self, key: KeyEvent) {
        let Some(session) = &mut self.edit else {
            return;
        };
        if session.find.typing {
            return self.key_find(key);
        }
        if let Some(action) = self.keymap.edit_action(&key) {
            return self.edit_action(action);
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        if key.code == KeyCode::F(1) {
            self.help_scroll = 0;
            self.mode = Mode::Help;
            return;
        }
        if key.code == KeyCode::Esc {
            if session.editor.selection().is_some() {
                session.editor.run(Command::Deselect);
            } else if !session.find.query.is_empty() {
                session.find = Find::default();
            } else {
                self.leave_editor();
            }
            return;
        }
        let editor = &mut session.editor;
        let lines = editor.lines();
        let cursor = editor.cursor();
        let selection = editor.selection();
        let motion = |m| Some(vec![Command::Move(m, shift)]);
        let commands = match key.code {
            KeyCode::Enter => Some(assist::enter(lines, cursor).unwrap_or(vec![Command::Newline])),
            KeyCode::Tab => Some(
                assist::indent(lines, cursor, false)
                    .unwrap_or_else(|| vec![Command::Text(soft_tab(cursor.1))]),
            ),
            KeyCode::BackTab => assist::indent(lines, cursor, true),
            KeyCode::Backspace if ctrl || alt => Some(vec![Command::DeleteWordBack]),
            KeyCode::Backspace if selection.is_none() => {
                Some(assist::unpair(lines, cursor).unwrap_or(vec![Command::Backspace]))
            }
            KeyCode::Backspace => Some(vec![Command::Backspace]),
            KeyCode::Delete if ctrl || alt => Some(vec![Command::DeleteWordForward]),
            KeyCode::Delete => Some(vec![Command::Delete]),
            KeyCode::Char('w') if ctrl => Some(vec![Command::DeleteWordBack]),
            KeyCode::Char('b') if alt => motion(Motion::WordLeft),
            KeyCode::Char('f') if alt => motion(Motion::WordRight),
            KeyCode::Char('d') if alt => Some(vec![Command::DeleteWordForward]),
            KeyCode::Left if ctrl || alt => motion(Motion::WordLeft),
            KeyCode::Right if ctrl || alt => motion(Motion::WordRight),
            KeyCode::Up if ctrl || alt => motion(Motion::ParagraphUp),
            KeyCode::Down if ctrl || alt => motion(Motion::ParagraphDown),
            KeyCode::Home if ctrl => motion(Motion::Top),
            KeyCode::End if ctrl => motion(Motion::Bottom),
            KeyCode::Left => motion(Motion::Left),
            KeyCode::Right => motion(Motion::Right),
            KeyCode::Up => motion(Motion::Up),
            KeyCode::Down => motion(Motion::Down),
            KeyCode::Home => motion(Motion::LineStart),
            KeyCode::End => motion(Motion::LineEnd),
            KeyCode::PageUp => motion(Motion::PageUp),
            KeyCode::PageDown => motion(Motion::PageDown),
            KeyCode::Char(c) if !ctrl => Some(match selection {
                Some(range) => assist::wrap(range, c).unwrap_or(vec![Command::Insert(c)]),
                None => assist::pair(lines, cursor, c).unwrap_or(vec![Command::Insert(c)]),
            }),
            _ => None,
        };
        match commands {
            Some(mut commands) if commands.len() == 1 => editor.run(commands.remove(0)),
            Some(commands) => editor.group(commands),
            None => {}
        }
    }

    fn edit_action(&mut self, action: Action) {
        let Some(session) = &mut self.edit else {
            return;
        };
        let editor = &mut session.editor;
        match action {
            Action::Save => self.save(false, false),
            Action::Undo => editor.run(Command::Undo),
            Action::Redo => editor.run(Command::Redo),
            Action::Cut | Action::Copy => {
                if editor.selection().is_none() {
                    self.notice = Some("select something first".to_string());
                    return;
                }
                editor.run(if action == Action::Cut {
                    Command::Cut
                } else {
                    Command::Copy
                });
                self.clipboard = Some(editor.yanked());
            }
            Action::Paste => editor.run(Command::Paste),
            Action::SelectAll => editor.run(Command::SelectAll),
            Action::Find => {
                session.find = Find {
                    typing: true,
                    origin: editor.cursor(),
                    ..Find::default()
                };
            }
            Action::FindNext => self.step_find(1),
            Action::FindPrev => self.step_find(-1),
            Action::ToggleTask => {
                let commands = assist::toggle_task(editor.lines(), editor.cursor());
                editor.group(commands);
            }
            _ => {}
        }
    }

    fn key_find(&mut self, key: KeyEvent) {
        let Some(session) = &mut self.edit else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                let (row, col) = session.find.origin;
                session.find = Find::default();
                session.editor.run(Command::Jump(row, col));
                return;
            }
            KeyCode::Enter => {
                session.find.typing = false;
                if session.find.matches.is_empty() && !session.find.query.is_empty() {
                    self.notice = Some("no matches".to_string());
                }
                return;
            }
            KeyCode::Backspace => {
                session.find.query.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                session.find.query.push(c);
            }
            _ => return,
        }
        session.find.version = None;
        session.refresh_find();
        let origin = session.find.origin;
        let first = session
            .find
            .matches
            .iter()
            .position(|m| (m.0, m.1) >= origin)
            .or((!session.find.matches.is_empty()).then_some(0));
        session.find.current = first;
        match first {
            Some(i) => {
                let (row, start, end) = session.find.matches[i];
                session
                    .editor
                    .run(Command::Select((row, start), (row, end)));
            }
            None => session.editor.run(Command::Jump(origin.0, origin.1)),
        }
    }

    fn step_find(&mut self, delta: isize) {
        let Some(session) = &mut self.edit else {
            return;
        };
        session.refresh_find();
        let count = session.find.matches.len();
        if count == 0 {
            self.notice = Some(if session.find.query.is_empty() {
                format!("{} finds first", self.keymap.first(Action::Find))
            } else {
                "no matches".to_string()
            });
            return;
        }
        let cursor = session.editor.cursor();
        let next = match session.find.current {
            Some(i) => (i as isize + delta).rem_euclid(count as isize) as usize,
            None if delta > 0 => session
                .find
                .matches
                .iter()
                .position(|m| (m.0, m.1) >= cursor)
                .unwrap_or(0),
            None => session
                .find
                .matches
                .iter()
                .rposition(|m| (m.0, m.1) < cursor)
                .unwrap_or(count - 1),
        };
        session.find.current = Some(next);
        let (row, start, end) = session.find.matches[next];
        session
            .editor
            .run(Command::Select((row, start), (row, end)));
    }

    pub(super) fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        match self.mode {
            Mode::View if self.focus == Focus::Editor => {
                if let Some(session) = &mut self.edit {
                    if session.find.typing {
                        session
                            .find
                            .query
                            .push_str(text.lines().next().unwrap_or(""));
                        session.find.version = None;
                    } else {
                        session.editor.group(vec![Command::Text(text)]);
                    }
                }
            }
            Mode::Search => {
                self.query.push_str(text.lines().next().unwrap_or(""));
                self.update_search();
            }
            Mode::Finder => {
                self.finder
                    .query
                    .push_str(text.lines().next().unwrap_or(""));
                if let Some(project) = &self.project {
                    self.finder.search(project);
                }
            }
            _ => {}
        }
    }

    pub(super) fn mouse_edit(&mut self, mouse: MouseEvent) -> bool {
        let Some(session) = &mut self.edit else {
            return false;
        };
        let at = session.editor.position_at(mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::ScrollDown => session.editor.run(Command::Scroll(3)),
            MouseEventKind::ScrollUp => session.editor.run(Command::Scroll(-3)),
            MouseEventKind::Down(MouseButton::Left) => {
                let Some((row, col)) = at else { return false };
                session.anchor = Some((row, col));
                session.editor.run(Command::Jump(row, col));
                self.focus = Focus::Editor;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let (Some(anchor), Some(to)) = (session.anchor, at) else {
                    return false;
                };
                session.editor.run(Command::Select(anchor, to));
            }
            MouseEventKind::Up(MouseButton::Left) => session.anchor = None,
            _ => return false,
        }
        true
    }

    /// Runs `f`, and if it panics, writes unsaved work beside the file before the panic goes
    /// on, then says where it went.
    pub(super) fn guarded<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        match panic::catch_unwind(AssertUnwindSafe(|| f(self))) {
            Ok(value) => value,
            Err(payload) => {
                if let Some(path) = self.write_recovery() {
                    eprintln!("mido: unsaved changes were written to {}", path.display());
                }
                panic::resume_unwind(payload)
            }
        }
    }

    fn write_recovery(&mut self) -> Option<PathBuf> {
        let session = self.edit.as_mut()?;
        if !session.dirty() {
            return None;
        }
        let path = session.file.recovery_path();
        std::fs::write(&path, session.encoded()).ok()?;
        Some(path)
    }
}

fn soft_tab(col: usize) -> String {
    " ".repeat(4 - col % 4)
}

/// The line holding a byte offset of the file as the reader parsed it.
fn line_at(lines: &[String], eol: Eol, bom: bool, offset: usize) -> usize {
    let break_len = match eol {
        Eol::Lf => 1,
        Eol::CrLf => 2,
    };
    let mut at = if bom { 3 } else { 0 };
    for (i, line) in lines.iter().enumerate() {
        at += line.len() + break_len;
        if at > offset {
            return i;
        }
    }
    lines.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Settings, Source};

    fn editing(text: &str) -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, text).unwrap();
        let mut app = App::with_settings(Source::File(path), text, Settings::default());
        app.start_editing();
        (dir, app)
    }

    #[test]
    fn a_panic_leaves_unsaved_work_beside_the_file() {
        let (dir, mut app) = editing("# Note\n");
        app.press(KeyCode::Char('x'));
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            app.guarded(|_| panic!("boom"));
        }));
        assert!(result.is_err());
        let recovery = dir.path().join(".note.md.mido-recover");
        assert_eq!(std::fs::read_to_string(recovery).unwrap(), "x# Note\n");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("note.md")).unwrap(),
            "# Note\n"
        );
    }

    #[test]
    fn a_clean_buffer_leaves_no_recovery_file() {
        let (dir, mut app) = editing("# Note\n");
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            app.guarded(|_| panic!("boom"));
        }));
        assert!(!dir.path().join(".note.md.mido-recover").exists());
    }

    #[test]
    fn find_is_smart_case() {
        let lines = vec!["Rust and rust".to_string(), "RUST".to_string()];
        assert_eq!(find(&lines, "rust"), [(0, 0, 4), (0, 9, 13), (1, 0, 4)]);
        assert_eq!(find(&lines, "Rust"), [(0, 0, 4)]);
    }

    #[test]
    fn offsets_map_to_lines() {
        let lines = vec!["ab".to_string(), "cd".to_string(), "ef".to_string()];
        assert_eq!(line_at(&lines, Eol::Lf, false, 0), 0);
        assert_eq!(line_at(&lines, Eol::Lf, false, 3), 1);
        assert_eq!(line_at(&lines, Eol::CrLf, false, 4), 1);
        assert_eq!(line_at(&lines, Eol::Lf, true, 5), 0);
        assert_eq!(line_at(&lines, Eol::Lf, true, 6), 1);
        assert_eq!(line_at(&lines, Eol::Lf, false, 99), 2);
    }
}
