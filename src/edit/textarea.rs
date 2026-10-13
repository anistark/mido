use std::hash::{DefaultHasher, Hash, Hasher};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Widget;
use ratatui_textarea::{CursorMove, Scrolling, TextArea, WrapMode};
use unicode_width::UnicodeWidthChar;

use super::{Command, Editor, Look, Motion};

const HISTORY: usize = 10_000;
const TAB: usize = 4;

/// One undo step: the text's hash before and after it. Undo replays the textarea's own
/// history until the text hashes to `before` again, so a step can span any number of edits.
#[derive(Debug, Clone, Copy)]
struct Step {
    before: u64,
    after: u64,
}

/// What the last edit was, so a run of typing or erasing undoes as one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Streak {
    Typing { at: (usize, usize), space: bool },
    Erasing { at: (usize, usize) },
}

/// A painted screen row: the line and the characters of it shown there.
#[derive(Debug, Clone, Copy)]
struct Row {
    line: usize,
    start: usize,
    end: usize,
}

pub struct TextAreaEditor {
    area: TextArea<'static>,
    hash: u64,
    version: u64,
    undo: Vec<Step>,
    redo: Vec<Step>,
    streak: Option<Streak>,
    rows: Vec<Row>,
    painted: Rect,
}

impl TextAreaEditor {
    pub fn new(lines: Vec<String>) -> Self {
        let mut area = TextArea::new(lines);
        area.set_wrap_mode(WrapMode::WordOrGlyph);
        area.set_max_histories(HISTORY);
        area.set_tab_length(TAB as u8);
        area.set_cursor_line_style(Default::default());
        area.set_cursor_style(Default::default());
        area.set_selection_style(Default::default());
        let mut editor = Self {
            area,
            hash: 0,
            version: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            streak: None,
            rows: Vec::new(),
            painted: Rect::default(),
        };
        editor.hash = editor.digest();
        editor
    }

    fn digest(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.area.lines().hash(&mut hasher);
        hasher.finish()
    }

    fn jump(&mut self, (row, col): (usize, usize)) {
        let clamp = |n: usize| n.min(u16::MAX as usize) as u16;
        self.area
            .move_cursor(CursorMove::Jump(clamp(row), clamp(col)));
    }

    fn select(&mut self, extend: bool) {
        if extend {
            if !self.area.is_selecting() {
                self.area.start_selection();
            }
        } else {
            self.area.cancel_selection();
        }
    }

    fn apply(&mut self, command: Command) {
        let area = &mut self.area;
        match command {
            Command::Insert(c) => area.insert_char(c),
            Command::Text(text) => {
                area.insert_str(text);
            }
            Command::Newline => area.insert_newline(),
            Command::Backspace => {
                area.delete_char();
            }
            Command::Delete => {
                area.delete_next_char();
            }
            Command::DeleteForward(n) => keep_yank(area, |a| a.delete_str(n)),
            Command::DeleteWordBack => keep_yank(area, TextArea::delete_word),
            Command::DeleteWordForward => keep_yank(area, TextArea::delete_next_word),
            Command::Move(motion, extend) => {
                self.select(extend);
                let before = self.cursor();
                let last = self.area.lines().len() - 1;
                let step = match motion {
                    Motion::Left => CursorMove::Back,
                    Motion::Right => CursorMove::Forward,
                    Motion::Up => CursorMove::Up,
                    Motion::Down => CursorMove::Down,
                    Motion::WordLeft => CursorMove::WordBack,
                    Motion::WordRight => CursorMove::WordForward,
                    Motion::LineStart => CursorMove::Head,
                    Motion::LineEnd => CursorMove::End,
                    Motion::ParagraphUp => CursorMove::ParagraphBack,
                    Motion::ParagraphDown => CursorMove::ParagraphForward,
                    Motion::PageUp => return self.area.scroll(Scrolling::PageUp),
                    Motion::PageDown => return self.area.scroll(Scrolling::PageDown),
                    Motion::Top => return self.jump((0, 0)),
                    Motion::Bottom => return self.jump((last, usize::MAX)),
                };
                self.area.move_cursor(step);
                if self.cursor() == before {
                    match motion {
                        Motion::Up => self.jump((before.0, 0)),
                        Motion::Down => self.jump((before.0, usize::MAX)),
                        _ => {}
                    }
                }
            }
            Command::Jump(row, col) => {
                self.area.cancel_selection();
                self.jump((row, col));
            }
            Command::Select(from, to) => {
                self.area.cancel_selection();
                self.jump(from);
                self.area.start_selection();
                self.jump(to);
            }
            Command::SelectAll => area.select_all(),
            Command::Deselect => area.cancel_selection(),
            Command::Cut => {
                area.cut();
            }
            Command::Copy => area.copy(),
            Command::Paste => {
                if !area.yank_text().is_empty() {
                    area.paste();
                }
            }
            Command::Scroll(rows) => area.scroll((rows, 0)),
            Command::Undo | Command::Redo => {}
        }
    }

    fn changes_text(command: &Command) -> bool {
        matches!(
            command,
            Command::Insert(_)
                | Command::Text(_)
                | Command::Newline
                | Command::Backspace
                | Command::Delete
                | Command::DeleteForward(_)
                | Command::DeleteWordBack
                | Command::DeleteWordForward
                | Command::Cut
                | Command::Paste
        )
    }

    /// Notes an edit as an undo step, or folds it into the last one when it carries on the
    /// same run of typing or erasing. `streak` holds the cursor from before the edit.
    fn record(&mut self, before: u64, streak: Option<Streak>) {
        let after = self.digest();
        self.hash = after;
        let previous = self.streak.take();
        self.redo.clear();
        if after == before {
            return;
        }
        self.version += 1;
        let continues = match (previous, streak) {
            (
                Some(Streak::Typing { at, space }),
                Some(Streak::Typing {
                    at: now,
                    space: now_space,
                }),
            ) => at == now && (space || !now_space),
            (Some(Streak::Erasing { at }), Some(Streak::Erasing { at: now })) => at == now,
            _ => false,
        };
        match self.undo.last_mut() {
            Some(step) if continues => step.after = after,
            _ => self.undo.push(Step { before, after }),
        }
        let at = self.cursor();
        self.streak = streak.map(|s| match s {
            Streak::Typing { space, .. } => Streak::Typing { at, space },
            Streak::Erasing { .. } => Streak::Erasing { at },
        });
    }

    fn step(&mut self, undo: bool) {
        let step = if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        let Some(step) = step else { return };
        self.area.cancel_selection();
        let target = if undo { step.before } else { step.after };
        while self.digest() != target {
            let moved = if undo {
                self.area.undo()
            } else {
                self.area.redo()
            };
            if !moved {
                break;
            }
        }
        self.hash = self.digest();
        self.version += 1;
        self.streak = None;
        if undo {
            self.redo.push(step);
        } else {
            self.undo.push(step);
        }
    }
}

impl Editor for TextAreaEditor {
    fn lines(&self) -> &[String] {
        self.area.lines()
    }

    fn cursor(&self) -> (usize, usize) {
        let cursor = self.area.cursor();
        (cursor.0, cursor.1)
    }

    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.area.selection_range().filter(|(a, b)| a != b)
    }

    fn run(&mut self, command: Command) {
        match command {
            Command::Undo => self.step(true),
            Command::Redo => self.step(false),
            command if Self::changes_text(&command) => {
                let streak = match &command {
                    Command::Insert(c) if !self.area.is_selecting() => Some(Streak::Typing {
                        at: self.cursor(),
                        space: c.is_whitespace(),
                    }),
                    Command::Backspace if !self.area.is_selecting() => {
                        Some(Streak::Erasing { at: self.cursor() })
                    }
                    _ => None,
                };
                let before = self.hash;
                self.apply(command);
                self.record(before, streak);
            }
            command => {
                if !matches!(command, Command::Scroll(_)) {
                    self.streak = None;
                }
                self.apply(command);
            }
        }
    }

    fn group(&mut self, commands: Vec<Command>) {
        let before = self.hash;
        for command in commands {
            self.apply(command);
        }
        self.streak = None;
        self.record(before, None);
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn yanked(&self) -> String {
        self.area.yank_text()
    }

    fn top(&self) -> usize {
        self.rows.first().map_or(0, |r| r.line)
    }

    fn scroll_to(&mut self, line: usize) {
        if self.area.is_selecting() {
            return;
        }
        let line = line.min(self.area.lines().len() - 1);
        let cursor = self.cursor();
        self.jump((line, 0));
        let target = self.area.screen_cursor().row;
        let (top, _) = self.area.scroll_offset();
        let delta = (target as i64 - i64::from(top)).clamp(i16::MIN.into(), i16::MAX.into());
        self.area.scroll((delta as i16, 0));
        self.jump(cursor);
        let row = self.area.screen_cursor().row;
        let height = usize::from(self.painted.height.max(1));
        if row < target || row >= target + height {
            self.jump((line, 0));
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, look: &Look) -> Option<Position> {
        self.painted = area;
        if area.width == 0 || area.height == 0 {
            return None;
        }
        (&self.area).render(area, buf);
        let (top, _) = self.area.scroll_offset();
        self.rows.clear();
        for r in 0..usize::from(area.height) {
            let start = self.area.screen_to_data(usize::from(top) + r, 0);
            if self
                .rows
                .last()
                .is_some_and(|last| (last.line, last.start) == (start.0, start.1))
            {
                break;
            }
            let end = self
                .area
                .screen_to_data(usize::from(top) + r, usize::MAX / 2);
            self.rows.push(Row {
                line: start.0,
                start: start.1,
                end: end.1,
            });
        }
        let (cursor_row, cursor_col) = self.cursor();
        let selection = self.selection();
        let lines = self.area.lines();
        let mut cursor = None;
        let mut found = None;
        for (i, row) in self.rows.iter().enumerate() {
            let y = area.y + i as u16;
            let current_line = row.line == cursor_row;
            let line_style = if current_line {
                look.base.patch(look.cursor_line)
            } else {
                look.base
            };
            buf.set_style(
                Rect {
                    y,
                    height: 1,
                    ..area
                },
                line_style,
            );
            let runs = look.runs.get(row.line).map_or(&[][..], Vec::as_slice);
            let first = look.matches.partition_point(|m| m.0 < row.line);
            let matches = &look.matches[first..];
            let mut x = 0usize;
            for (ci, (byte, c)) in line_chars(&lines[row.line], row.start, row.end) {
                if current_line && ci == cursor_col {
                    cursor = Some(x);
                }
                let width = if c == '\t' {
                    TAB - x % TAB
                } else {
                    c.width().unwrap_or(0)
                };
                if width == 0 {
                    continue;
                }
                let mut style = line_style;
                for run in runs.iter().filter(|r| r.range.contains(&byte)) {
                    style = style.patch(run.style);
                }
                if selection.is_some_and(|(a, b)| a <= (row.line, ci) && (row.line, ci) < b) {
                    style = style.patch(look.selection);
                }
                if let Some(index) = matches
                    .iter()
                    .take_while(|m| m.0 == row.line)
                    .position(|m| m.1 <= ci && ci < m.2)
                {
                    let current = look.current_match == Some(first + index);
                    style = style.patch(if current { look.current } else { look.search });
                }
                for dx in 0..width {
                    let cx = x + dx;
                    if cx < usize::from(area.width) {
                        buf[(area.x + cx as u16, y)].set_style(style);
                    }
                }
                x += width;
            }
            let owns_end = self
                .rows
                .get(i + 1)
                .is_none_or(|next| next.line != row.line);
            if current_line && cursor_col == row.end && owns_end && cursor.is_none() {
                cursor = Some(x);
            }
            if let Some(x) = cursor.take() {
                let x = (x as u16).min(area.width - 1);
                found = Some(Position::new(area.x + x, y));
            }
        }
        found
    }

    fn position_at(&self, column: u16, row: u16) -> Option<(usize, usize)> {
        let area = self.painted;
        if column < area.x || column >= area.right() || row < area.y || row >= area.bottom() {
            return None;
        }
        let lines = self.area.lines();
        let Some(r) = self.rows.get(usize::from(row - area.y)) else {
            let last = self.rows.last()?;
            return Some((last.line, lines[last.line].chars().count()));
        };
        let want = usize::from(column - area.x);
        let mut x = 0;
        for (ci, (_, c)) in line_chars(&lines[r.line], r.start, r.end) {
            let width = if c == '\t' {
                TAB - x % TAB
            } else {
                c.width().unwrap_or(0)
            };
            if want < x + width {
                return Some((r.line, ci));
            }
            x += width;
        }
        Some((r.line, r.end))
    }
}

/// Runs a delete that the textarea would also copy, leaving the copied text as it was.
fn keep_yank(area: &mut TextArea<'static>, delete: impl FnOnce(&mut TextArea<'static>) -> bool) {
    let yank = area.yank_text();
    delete(area);
    area.set_yank_text(yank);
}

fn line_chars(
    line: &str,
    start: usize,
    end: usize,
) -> impl Iterator<Item = (usize, (usize, char))> {
    line.char_indices()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
}
