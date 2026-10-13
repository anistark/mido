//! The built-in editor: a backend behind the `Editor` trait, the Markdown assists that drive
//! it, and the file it saves to.

pub mod assist;
pub mod clipboard;
pub mod file;
mod textarea;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;

use crate::render::source::Run;
pub use textarea::TextAreaEditor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    ParagraphUp,
    ParagraphDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// Everything mido asks of an editor backend. Text-changing commands are undone one at a
/// time, except inside `Editor::group`, which undoes as one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Insert(char),
    Text(String),
    Newline,
    Backspace,
    Delete,
    DeleteForward(usize),
    DeleteWordBack,
    DeleteWordForward,
    /// Move the cursor, extending the selection when the flag is set.
    Move(Motion, bool),
    Jump(usize, usize),
    Select((usize, usize), (usize, usize)),
    SelectAll,
    Deselect,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Scroll(i16),
}

/// The styles an editor paints with, worked out by the app from the theme.
pub struct Look<'a> {
    pub base: Style,
    pub cursor_line: Style,
    pub selection: Style,
    pub search: Style,
    pub current: Style,
    pub runs: &'a [Vec<Run>],
    /// Search hits as (line, start char, end char), sorted.
    pub matches: &'a [(usize, usize, usize)],
    pub current_match: Option<usize>,
}

/// A text editor that mido can swap in behind the same keys, panels and preview. The
/// textarea backend is the first, and a vim-modal one can follow without touching the app.
pub trait Editor {
    fn lines(&self) -> &[String];
    /// Row and character column.
    fn cursor(&self) -> (usize, usize);
    fn selection(&self) -> Option<((usize, usize), (usize, usize))>;
    fn run(&mut self, command: Command);
    /// Runs commands as one undo step.
    fn group(&mut self, commands: Vec<Command>);
    /// Bumps whenever the text changes, undo and redo included.
    fn version(&self) -> u64;
    /// What `Copy` or `Cut` last took.
    fn yanked(&self) -> String;
    /// The first line in view.
    fn top(&self) -> usize;
    fn scroll_to(&mut self, line: usize);
    /// Paints the text into `area` and returns where the cursor sits on screen.
    fn render(&mut self, area: Rect, buf: &mut Buffer, look: &Look) -> Option<Position>;
    /// The row and column under a screen cell in the last painted area.
    fn position_at(&self, column: u16, row: u16) -> Option<(usize, usize)>;

    fn text(&self) -> String {
        self.lines().join("\n")
    }
}

pub fn new_editor(lines: Vec<String>) -> Box<dyn Editor> {
    Box::new(TextAreaEditor::new(lines))
}
