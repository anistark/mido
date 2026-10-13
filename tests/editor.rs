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

fn sections(count: usize) -> String {
    (1..=count)
        .map(|i| format!("## Section {i}\n\nBody of section {i}.\n\n"))
        .collect()
}

fn split_fixture(text: &str, cols: u16, rows: u16) -> Fixture {
    let mut f = Fixture::new(Some(text));
    f.terminal = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    f.key(KeyCode::Char('v'));
    f.screen();
    f
}

/// The editor's and the preview's rows, split at the divider.
fn panes(screen: &str) -> (Vec<String>, Vec<String>) {
    let rows: Vec<Vec<char>> = screen
        .lines()
        .map(|l| l.trim_matches('"').chars().collect())
        .collect();
    let divider = (0..rows[2].len())
        .find(|&x| {
            rows[2..rows.len() - 3]
                .iter()
                .all(|r| r.get(x) == Some(&'│'))
        })
        .expect("a divider");
    let body = &rows[..rows.len() - 1];
    let left = body.iter().map(|r| r[..divider].iter().collect()).collect();
    let right = body
        .iter()
        .map(|r| r[divider + 1..].iter().collect())
        .collect();
    (left, right)
}

/// The number of the first section either pane shows.
fn first_section(rows: &[String]) -> Option<usize> {
    rows.iter().find_map(|row| {
        let at = row.find("ection ")? + 7;
        let digits: String = row[at..].chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

#[test]
fn v_opens_the_editor_beside_a_live_preview() {
    let mut f = split_fixture(NOTE, 100, 16);
    assert!(f.app.editing() && f.app.splitting());
    let screen = f.screen();
    insta::assert_snapshot!("split_open", screen);
    let (left, right) = panes(&screen);
    assert!(left.iter().any(|r| r.contains("# Groceries")));
    assert!(
        right
            .iter()
            .any(|r| r.contains("Groceries") && !r.contains('#'))
    );
    f.key(KeyCode::End);
    f.typing(" list");
    f.app.flush_preview();
    let (_, right) = panes(&f.screen());
    assert!(
        right.iter().any(|r| r.contains("Groceries list")),
        "{right:#?}"
    );
}

#[test]
fn the_preview_follows_the_editor_through_the_source_map() {
    let mut f = split_fixture(&sections(40), 100, 20);
    for target in ["Section 12", "section 25", "Section 33", "section 3."] {
        f.ctrl('f');
        f.typing(target);
        f.key(KeyCode::Enter);
        let (left, right) = panes(&f.screen());
        assert_eq!(
            first_section(&left),
            first_section(&right),
            "after finding {target}:\n{left:#?}\n{right:#?}"
        );
    }
}

#[test]
fn the_editor_follows_the_preview_and_s_unlinks_them() {
    let mut f = split_fixture(&sections(40), 100, 20);
    f.key(KeyCode::Esc);
    assert!(f.status().contains("preview: s sync on"), "{}", f.status());
    for _ in 0..30 {
        f.key(KeyCode::Char('j'));
    }
    let (left, right) = panes(&f.screen());
    let shown = first_section(&right);
    assert!(shown > Some(3), "{right:#?}");
    assert_eq!(first_section(&left), shown, "{left:#?}\n{right:#?}");
    f.key(KeyCode::Char('s'));
    for _ in 0..30 {
        f.key(KeyCode::Char('j'));
    }
    let (left, right) = panes(&f.screen());
    assert_eq!(first_section(&left), shown, "the editor stays put");
    assert!(first_section(&right) > shown);
}

#[test]
fn sync_holds_inside_a_long_paragraph() {
    let mut text = String::from("# Long\n\n");
    for i in 1..=60 {
        text.push_str(&format!(
            "Line {i} of one long paragraph, wrapped by hand.\n"
        ));
    }
    let mut f = split_fixture(&text, 100, 20);
    f.ctrl('f');
    f.typing("Line 40 ");
    f.key(KeyCode::Enter);
    let (left, right) = panes(&f.screen());
    let top = |rows: &[String]| {
        rows.iter().find_map(|row| {
            let at = row.find("Line ")? + 5;
            row[at..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<usize>()
                .ok()
        })
    };
    let (editor, preview) = (top(&left).unwrap(), top(&right).unwrap());
    assert!(editor > 20, "{left:#?}");
    assert!(
        preview.abs_diff(editor) <= 1,
        "editor at line {editor}, preview at {preview}:\n{right:#?}"
    );
}

#[test]
fn clicking_the_preview_moves_the_cursor_there() {
    let mut f = split_fixture(&sections(10), 100, 24);
    let screen = f.screen();
    let (_, right) = panes(&screen);
    let row = right
        .iter()
        .position(|r| r.contains("Body of section 3."))
        .unwrap();
    let column = screen.lines().nth(row).unwrap().find("Body").unwrap() as u16;
    f.app.mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row: row as u16,
        modifiers: KeyModifiers::NONE,
    });
    assert!(f.status().contains("Ln 11, Col 1"), "{}", f.status());
}

#[test]
fn angle_brackets_move_the_divider_and_ctrl_e_hides_the_preview() {
    let mut f = split_fixture(NOTE, 100, 16);
    let at = |screen: &str| {
        screen
            .lines()
            .nth(4)
            .unwrap()
            .trim_matches('"')
            .chars()
            .position(|c| c == '│')
    };
    let before = at(&f.screen()).unwrap();
    f.key(KeyCode::Esc);
    f.key(KeyCode::Char('>'));
    f.key(KeyCode::Char('>'));
    let wider = at(&f.screen()).unwrap();
    assert!(wider > before, "{before} then {wider}");
    f.key(KeyCode::Char('e'));
    f.ctrl('e');
    assert!(!f.app.splitting());
    assert_eq!(at(&f.screen()), None);
    f.ctrl('e');
    assert!(f.app.splitting());
}

#[test]
fn the_files_panel_gives_way_to_the_split() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("README.md"), NOTE).unwrap();
    fs::write(dir.path().join("other.md"), "# Other\n").unwrap();
    let project = mido::project::Project::scan(dir.path());
    let file = project.entry_file();
    let mut app = App::with_settings(Source::Project { project, file }, NOTE, Settings::default());
    let mut terminal = Terminal::new(TestBackend::new(82, 16)).unwrap();
    let mut screen = |app: &mut App| {
        terminal.draw(|frame| app.draw(frame)).unwrap();
        terminal.backend().to_string()
    };
    app.press(KeyCode::Char('e'));
    assert!(
        screen(&mut app).contains("other.md"),
        "the sidebar fits beside the editor"
    );
    app.press(KeyCode::Esc);
    app.press(KeyCode::Char('v'));
    screen(&mut app);
    let split = screen(&mut app);
    assert!(
        !split.contains("other.md"),
        "the sidebar goes first:\n{split}"
    );
    assert!(app.splitting());
}

#[test]
fn stacked_puts_the_preview_below() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("note.md");
    fs::write(&path, NOTE).unwrap();
    let settings = Settings {
        split: mido::config::Split::Stacked,
        ..Settings::default()
    };
    let mut app = App::with_settings(Source::File(path), NOTE, settings);
    let mut terminal = Terminal::new(TestBackend::new(60, 30)).unwrap();
    app.press(KeyCode::Char('v'));
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let screen = terminal.backend().to_string();
    let rows: Vec<&str> = screen.lines().collect();
    let divider = rows.iter().position(|r| r.contains("──────────")).unwrap();
    assert!(rows[..divider].iter().any(|r| r.contains("# Groceries")));
    assert!(
        rows[divider..]
            .iter()
            .any(|r| r.contains("Groceries") && !r.contains('#'))
    );
}

#[test]
fn quitting_with_unsaved_changes_asks_first() {
    let mut f = split_fixture(NOTE, 100, 16);
    f.typing("x");
    f.key(KeyCode::Esc);
    f.ctrl('c');
    assert!(f.screen().contains("s save and quit"));
    f.key(KeyCode::Esc);
    assert!(f.app.editing());
    f.key(KeyCode::Char('q'));
    assert!(f.screen().contains("Unsaved changes"));
}

#[test]
fn links_in_the_preview_do_not_leave_unsaved_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("note.md");
    fs::write(&path, "See [other](other.md).\n").unwrap();
    fs::write(dir.path().join("other.md"), "# Other\n").unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let mut app = App::with_settings(Source::File(path.clone()), &text, Settings::default());
    let mut terminal = Terminal::new(TestBackend::new(100, 16)).unwrap();
    app.press(KeyCode::Char('v'));
    terminal.draw(|frame| app.draw(frame)).unwrap();
    app.press(KeyCode::Char('!'));
    app.press(KeyCode::Esc);
    terminal.draw(|frame| app.draw(frame)).unwrap();
    app.press(KeyCode::Char(']'));
    app.press(KeyCode::Enter);
    assert_eq!(app.current_file(), Some(path.as_path()));
    assert!(app.notice().is_some_and(|n| n.contains("unsaved changes")));
}
