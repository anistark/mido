use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;

use super::{App, Focus};
use crate::config::Split;
use crate::markdown::{Document, parse};
use crate::render::layout::{Layout, Options, layout_with};
use crate::render::theme::Theme;

/// How long the text must rest before the preview takes it.
pub const DEBOUNCE: Duration = Duration::from_millis(40);
const MIN_PANE_WIDTH: u16 = 30;
const MIN_PANE_HEIGHT: u16 = 3;

/// What the preview can be asked to do. The built-in editor is the first client, and a
/// control socket for other editors can send the same commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preview {
    /// Show a file, as following a link would.
    Open(PathBuf),
    /// Show this Markdown instead of the file on disk, keeping the reading position.
    SetContent(String),
    /// Bring a line of the source, counted from zero, to the top.
    ScrollToSourceLine(usize),
    /// Read the file again.
    Reload,
}

struct Job {
    id: u64,
    doc: Document,
    line_starts: Vec<usize>,
    theme: Theme,
    width: usize,
    options: Options,
}

struct Done {
    id: u64,
    doc: Document,
    line_starts: Vec<usize>,
    layout: Layout,
    width: usize,
}

/// Lays the preview out on a thread of its own, so typing never waits for a long document.
/// Only the newest job matters: older ones still queued are skipped.
pub(super) struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    next: u64,
    waiting: Option<u64>,
}

impl Worker {
    fn spawn() -> Self {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (outbox, done) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(mut job) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    job = newer;
                }
                let layout = layout_with(&job.doc, &job.theme, job.width, &job.options);
                let done = Done {
                    id: job.id,
                    doc: job.doc,
                    line_starts: job.line_starts,
                    layout,
                    width: job.width,
                };
                if outbox.send(done).is_err() {
                    break;
                }
            }
        });
        Self {
            jobs,
            done,
            next: 0,
            waiting: None,
        }
    }
}

/// Where the editor, the preview and the line between them go.
pub(super) struct SplitAreas {
    pub editor: Rect,
    pub preview: Option<Rect>,
    pub divider: Option<Rect>,
}

/// Byte offsets where each line starts.
pub(super) fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

impl App {
    pub fn preview(&mut self, command: Preview) {
        match command {
            Preview::Open(path) => {
                self.navigate(Some(path), None);
            }
            Preview::SetContent(text) => {
                self.set_text(&text);
                self.layout_width = 0;
            }
            Preview::ScrollToSourceLine(line) => {
                let last = self.line_starts.last().copied().unwrap_or(0);
                self.pending_source = Some(self.line_starts.get(line).copied().unwrap_or(last));
            }
            Preview::Reload => self.reload(),
        }
    }

    pub(super) fn set_text(&mut self, text: &str) {
        self.doc = parse(text);
        self.line_starts = line_starts(text);
        if let Some(worker) = &mut self.worker {
            worker.waiting = None;
        }
    }

    pub(super) fn awaiting_layout(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.waiting.is_some())
    }

    /// Parses here, which is quick, and leaves the layout to the worker. Falls back to laying
    /// out in place when there is no width to lay out at yet.
    fn set_content_later(&mut self, text: String) {
        if self.layout_width == 0 {
            return self.preview(Preview::SetContent(text));
        }
        let doc = parse(&text);
        let options = self.layout_options(&doc);
        let worker = self.worker.get_or_insert_with(Worker::spawn);
        worker.next += 1;
        let job = Job {
            id: worker.next,
            doc,
            line_starts: line_starts(&text),
            theme: self.theme.clone(),
            width: self.layout_width,
            options,
        };
        if worker.jobs.send(job).is_ok() {
            worker.waiting = Some(worker.next);
        } else {
            self.worker = None;
            self.preview(Preview::SetContent(text));
        }
    }

    /// Installs the worker's layout once it is ready, if nothing newer replaced it meanwhile.
    pub(super) fn receive_layout(&mut self) {
        let Some(worker) = &mut self.worker else {
            return;
        };
        let Some(done) = worker.done.try_iter().last() else {
            return;
        };
        if worker.waiting != Some(done.id) {
            return;
        }
        worker.waiting = None;
        self.doc = done.doc;
        self.line_starts = done.line_starts;
        if done.width == self.layout_width {
            self.install_layout(done.layout, done.width);
        } else {
            self.layout_width = 0;
        }
    }

    /// The source line holding a byte offset of the text the preview shows.
    pub(super) fn source_line(&self, offset: usize) -> usize {
        self.line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    pub fn splitting(&self) -> bool {
        self.split && self.edit.is_some()
    }

    /// Hands the editor's text to the preview once typing has rested for `DEBOUNCE`, or at
    /// once with `now`. Returns how long until it is due.
    pub(super) fn refresh_preview(&mut self, now: bool) -> Option<Duration> {
        if !self.split {
            return None;
        }
        let version = self.edit.as_ref()?.editor.version();
        if self.preview_version == Some(version) {
            return None;
        }
        if self.seen_version != Some(version) {
            self.seen_version = Some(version);
            self.edited_at = Instant::now();
        }
        let wait = DEBOUNCE.saturating_sub(self.edited_at.elapsed());
        if !now && !wait.is_zero() {
            return Some(wait);
        }
        let text = self.edit.as_ref()?.editor.text();
        if now {
            self.preview(Preview::SetContent(text));
        } else {
            self.set_content_later(text);
        }
        self.preview_version = Some(version);
        None
    }

    /// Puts the latest edits in the preview without waiting for typing to rest.
    pub fn flush_preview(&mut self) {
        self.refresh_preview(true);
    }

    pub(super) fn toggle_split(&mut self) {
        if self.edit.is_none() {
            self.start_editing();
            if self.edit.is_none() {
                return;
            }
            self.split = true;
        } else {
            self.split = !self.split;
        }
        if self.split {
            self.preview_version = None;
            self.flush_preview();
            self.driver = Focus::Editor;
        } else {
            self.focus = Focus::Editor;
        }
    }

    pub(super) fn toggle_sync(&mut self) {
        if !self.splitting() {
            self.notice = Some("sync is for the split, v opens it".to_string());
            return;
        }
        self.sync = !self.sync;
        self.synced = None;
        self.notice = Some(if self.sync { "sync on" } else { "sync off" }.to_string());
    }

    pub(super) fn resize_split(&mut self, delta: i16) {
        if !self.splitting() {
            self.notice = Some("no split, v opens one".to_string());
            return;
        }
        self.split_ratio = self.split_ratio.saturating_add_signed(delta).clamp(20, 80);
    }

    pub(super) fn split_areas(&self, area: Rect) -> SplitAreas {
        let single = SplitAreas {
            editor: area,
            preview: None,
            divider: None,
        };
        if !self.split {
            return single;
        }
        let ratio = |total: u16, min: u16| {
            (u32::from(total) * u32::from(self.split_ratio) / 100)
                .clamp(u32::from(min), u32::from(total.saturating_sub(min))) as u16
        };
        match self.split_mode {
            Split::SideBySide => {
                if area.width < 2 * MIN_PANE_WIDTH + 1 {
                    return single;
                }
                let width = ratio(area.width - 1, MIN_PANE_WIDTH);
                SplitAreas {
                    editor: Rect { width, ..area },
                    divider: Some(Rect {
                        x: area.x + width,
                        width: 1,
                        ..area
                    }),
                    preview: Some(Rect {
                        x: area.x + width + 1,
                        width: area.width - width - 1,
                        ..area
                    }),
                }
            }
            Split::Stacked => {
                if area.height < 2 * MIN_PANE_HEIGHT + 1 {
                    return single;
                }
                let height = ratio(area.height - 1, MIN_PANE_HEIGHT);
                SplitAreas {
                    editor: Rect { height, ..area },
                    divider: Some(Rect {
                        y: area.y + height,
                        height: 1,
                        ..area
                    }),
                    preview: Some(Rect {
                        y: area.y + height + 1,
                        height: area.height - height - 1,
                        ..area
                    }),
                }
            }
        }
    }

    /// The width the panels must leave for the split, so the sidebar goes before the preview.
    pub(super) fn split_min_width(&self) -> u16 {
        if self.split && self.edit.is_some() && self.split_mode == Split::SideBySide {
            2 * MIN_PANE_WIDTH + 1
        } else {
            0
        }
    }

    /// The preview leads: bring its top source line to the top of the editor.
    pub(super) fn follow_preview(&mut self) {
        if !self.sync || self.driver != Focus::Content || self.synced == Some(self.scroll) {
            return;
        }
        self.synced = Some(self.scroll);
        let Some(offset) = self.layout.source_at(self.scroll) else {
            return;
        };
        let line = self.source_line(offset);
        if let Some(session) = &mut self.edit {
            session.editor.scroll_to(line);
        }
    }

    /// The editor leads: bring its top line to the top of the preview.
    pub(super) fn follow_editor(&mut self) {
        if !self.sync || self.driver != Focus::Editor {
            return;
        }
        if let Some(session) = &self.edit {
            let top = session.editor.top();
            self.preview(Preview::ScrollToSourceLine(top));
            self.synced = None;
        }
    }

    /// A click in the preview puts the editor's cursor on the source line under it.
    pub(super) fn click_preview(&mut self, line: usize) {
        let Some(offset) = self.layout.source_at(line) else {
            return;
        };
        let row = self.source_line(offset);
        if let Some(session) = &mut self.edit {
            session.editor.run(crate::edit::Command::Jump(row, 0));
        }
        self.focus = Focus::Editor;
        self.driver = Focus::Content;
        self.synced = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worker_lays_out_the_preview_off_the_main_thread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "# One\n").unwrap();
        let mut app = App::with_settings(
            super::super::Source::File(path),
            "# One\n",
            super::super::Settings::default(),
        );
        app.ensure_layout(60);
        app.set_content_later("# Two\n\nmore\n".to_string());
        assert!(app.awaiting_layout());
        assert_eq!(app.layout.plain_lines()[0].trim(), "One");
        let start = Instant::now();
        while app.awaiting_layout() && start.elapsed() < Duration::from_secs(5) {
            app.receive_layout();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(app.layout.plain_lines()[0].trim(), "Two");
        assert_eq!(app.line_starts, [0, 6, 7, 12]);
        app.set_content_later("# Three\n".to_string());
        app.set_text("# Four\n");
        assert!(
            !app.awaiting_layout(),
            "a direct change drops the pending layout"
        );
    }

    #[test]
    fn line_starts_count_every_line() {
        assert_eq!(line_starts(""), [0]);
        assert_eq!(line_starts("a\nbc\n"), [0, 2, 5]);
    }
}
