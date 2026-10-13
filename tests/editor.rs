use std::fs;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use mido::app::{App, Settings, Source};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Position;

const NOTE: &str = "# Groceries\n\nThings to buy:\n\n- apples\n- [ ] bread\n\n> keep it short\n";

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    app: App,
    terminal: Terminal<TestBackend>,
}

impl Fixture {
    fn new(text: Option<&str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        if let Some(text) = text {
            fs::write(&path, text).unwrap();
        }
        let app = App::with_settings(
            Source::File(path.clone()),
            text.unwrap_or_default(),
            Settings::default(),
        );
        Self {
            _dir: dir,
            path,
            app,
            terminal: Terminal::new(TestBackend::new(60, 14)).unwrap(),
        }
    }

    fn screen(&mut self) -> String {
        self.terminal.draw(|frame| self.app.draw(frame)).unwrap();
        self.terminal.backend().to_string()
    }

    fn status(&mut self) -> String {
        self.screen().lines().last().unwrap().to_string()
    }

    fn key(&mut self, code: KeyCode) {
        self.app.press(code);
    }

    fn ctrl(&mut self, c: char) {
        self.app
            .key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
    }

    fn typing(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '\n' => self.key(KeyCode::Enter),
                c => self.key(KeyCode::Char(c)),
            }
        }
    }

    fn disk(&self) -> String {
        fs::read_to_string(&self.path).unwrap_or_default()
    }

    fn cursor(&mut self) -> Position {
        self.screen();
        self.terminal.get_cursor_position().unwrap()
    }
}

#[test]
fn e_opens_the_source_and_esc_returns_to_reading() {
    let mut f = Fixture::new(Some(NOTE));
    let reading = f.screen();
    assert!(
        !reading.contains("# Groceries"),
        "the reader renders:\n{reading}"
    );
    f.key(KeyCode::Char('e'));
    let editing = f.screen();
    insta::assert_snapshot!("editor_open", editing);
    assert!(editing.contains("# Groceries") && editing.contains("- [ ] bread"));
    assert!(f.status().contains("note.md") && f.status().contains("Ln 1, Col 1"));
    assert_eq!(f.cursor(), Position::new(2, 2));
    f.key(KeyCode::Esc);
    assert!(!f.app.editing());
    assert!(!f.screen().contains("# Groceries"));
}

#[test]
fn typing_marks_the_buffer_and_ctrl_s_saves_it() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.key(KeyCode::End);
    f.typing(" list");
    assert!(f.status().contains("note.md [+]"), "{}", f.status());
    assert_eq!(f.disk(), NOTE);
    f.ctrl('s');
    assert!(f.status().contains("saved note.md"), "{}", f.status());
    assert!(!f.status().contains("[+]"));
    assert_eq!(
        f.disk(),
        NOTE.replacen("# Groceries", "# Groceries list", 1)
    );
    f.key(KeyCode::Esc);
    assert!(!f.app.editing());
    assert!(
        f.screen().contains("Groceries list"),
        "the reader shows the edit"
    );
}

#[test]
fn leaving_with_unsaved_changes_asks_first() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.typing("x");
    f.key(KeyCode::Esc);
    let prompt = f.screen();
    assert!(
        prompt.contains("Unsaved changes") && prompt.contains("s save"),
        "{prompt}"
    );
    f.key(KeyCode::Esc);
    assert!(f.app.editing(), "Esc keeps editing");
    f.key(KeyCode::Esc);
    f.key(KeyCode::Char('d'));
    assert!(!f.app.editing());
    assert_eq!(f.disk(), NOTE);
    assert!(f.screen().contains("Groceries") && !f.screen().contains("xGroceries"));

    f.key(KeyCode::Char('e'));
    f.typing("y");
    f.key(KeyCode::Esc);
    f.key(KeyCode::Char('s'));
    assert!(!f.app.editing());
    assert!(f.disk().starts_with("y# Groceries"));
}

#[test]
fn a_file_changed_on_disk_is_not_overwritten_silently() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.typing("mine ");
    fs::write(&f.path, "theirs\n").unwrap();
    f.ctrl('s');
    let screen = f.screen();
    assert!(screen.contains("Changed on disk"), "{screen}");
    assert_eq!(f.disk(), "theirs\n");
    f.key(KeyCode::Char('r'));
    assert!(f.screen().contains("theirs"));
    f.typing("x");
    fs::write(&f.path, "again\n").unwrap();
    f.ctrl('s');
    f.key(KeyCode::Char('o'));
    assert_eq!(f.disk(), "theirxs\n", "the cursor stays where it was");
}

#[test]
fn enter_tab_and_tasks_follow_markdown() {
    let mut f = Fixture::new(Some("- apples\n"));
    f.key(KeyCode::Char('e'));
    f.key(KeyCode::End);
    f.typing("\npears\n");
    f.key(KeyCode::Tab);
    f.typing("green\n\n");
    f.typing("done");
    f.ctrl('t');
    f.ctrl('s');
    assert_eq!(f.disk(), "- apples\n- pears\n  - green\n- [ ] done\n");
    f.ctrl('z');
    f.ctrl('s');
    assert_eq!(f.disk(), "- apples\n- pears\n  - green\ndone\n");
}

#[test]
fn backticks_and_asterisks_come_in_pairs() {
    let mut f = Fixture::new(Some("\n"));
    f.key(KeyCode::Char('e'));
    f.typing("say `hi` and ");
    f.key(KeyCode::Char('*'));
    f.typing("wow");
    f.key(KeyCode::Char('*'));
    f.typing("!");
    f.ctrl('s');
    assert_eq!(f.disk(), "say `hi` and *wow*!\n");
}

#[test]
fn find_selects_matches_in_the_buffer() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.ctrl('f');
    f.typing("bread");
    assert!(f.status().contains("find: bread"), "{}", f.status());
    assert!(f.status().contains("1/1"));
    f.key(KeyCode::Enter);
    f.typing("toast");
    f.ctrl('s');
    assert!(f.disk().contains("- [ ] toast"));
    f.ctrl('f');
    f.typing("nothing here");
    assert!(f.status().contains("no matches"));
    f.key(KeyCode::Esc);
    assert!(f.app.editing());
}

#[test]
fn ctrl_c_copies_in_the_editor_and_paste_inserts_text_as_is() {
    let mut f = Fixture::new(Some("one\n"));
    f.key(KeyCode::Char('e'));
    f.app.key(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT));
    f.ctrl('c');
    assert!(f.app.editing(), "Ctrl-c does not quit the editor");
    f.key(KeyCode::End);
    f.ctrl('v');
    f.app.paste_text("\r\n- a\r\n- b");
    f.ctrl('s');
    assert_eq!(f.disk(), "oneone\n- a\n- b\n");
}

#[test]
fn clicking_places_the_cursor_and_dragging_selects() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.screen();
    let mouse = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    f.app
        .mouse_event(mouse(MouseEventKind::Down(MouseButton::Left), 4, 6));
    assert!(f.status().contains("Ln 5, Col 3"), "{}", f.status());
    f.app
        .mouse_event(mouse(MouseEventKind::Drag(MouseButton::Left), 10, 6));
    f.typing("X");
    f.ctrl('s');
    assert!(f.disk().contains("\n- X\n"), "{}", f.disk());
}

#[test]
fn a_missing_file_is_created_on_first_save() {
    let mut f = Fixture::new(None);
    f.app.start_editing();
    assert!(f.status().contains("new file"), "{}", f.status());
    f.typing("# Fresh");
    f.ctrl('s');
    assert!(f.status().contains("created note.md"));
    assert_eq!(f.disk(), "# Fresh\n");
}

#[test]
fn stdin_cannot_be_edited() {
    let mut app = App::new(Source::Stdin, "# Piped\n", None);
    app.press(KeyCode::Char('e'));
    assert!(!app.editing());
    assert_eq!(app.notice(), Some("stdin cannot be edited, open a file"));
}

#[test]
fn f1_shows_the_keys_inside_the_editor() {
    let mut f = Fixture::new(Some(NOTE));
    f.key(KeyCode::Char('e'));
    f.key(KeyCode::F(1));
    assert!(f.screen().contains("╭ Keys"));
    f.key(KeyCode::Char('q'));
    assert!(f.app.editing());
    f.typing("z");
    assert!(f.screen().contains("z# Groceries"));
}
