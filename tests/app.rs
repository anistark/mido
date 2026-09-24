use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use mido::app::{App, Source};
use mido::project::Project;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn screen(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String {
    terminal.draw(|frame| app.draw(frame)).unwrap();
    terminal.backend().to_string()
}

fn press(app: &mut App, code: KeyCode) {
    app.press(code);
}

fn content_line(screen: &str, index: usize) -> String {
    screen
        .lines()
        .nth(index + 2)
        .unwrap_or_default()
        .to_string()
}

fn has_files(screen: &str) -> bool {
    screen
        .lines()
        .next()
        .is_some_and(|l| l.trim_start_matches('"').starts_with(" docs"))
}

#[test]
fn readme_view_overlays_and_search() {
    let text = std::fs::read_to_string("tests/fixtures/readme.md").unwrap();
    let mut app = App::new(Source::Stdin, &text, None);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    insta::assert_snapshot!("view", screen(&mut app, &mut terminal));

    press(&mut app, KeyCode::Char('t'));
    insta::assert_snapshot!("toc", screen(&mut app, &mut terminal));
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('/'));
    for c in "table".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    insta::assert_snapshot!("search", screen(&mut app, &mut terminal));

    press(&mut app, KeyCode::Char('?'));
    insta::assert_snapshot!("help", screen(&mut app, &mut terminal));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('h'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("╭ Keys") && out.contains("j/k scroll"),
        "h opens help too:\n{out}"
    );
    assert!(
        out.lines().last().unwrap().contains("? help"),
        "the status bar stays visible:\n{out}"
    );
    press(&mut app, KeyCode::Char('G'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("This help") && out.contains("Quit"),
        "G scrolls the help to its end:\n{out}"
    );
    press(&mut app, KeyCode::Esc);
    assert!(!screen(&mut app, &mut terminal).contains("╭ Keys"));
}

#[test]
fn resize_keeps_the_reading_position() {
    let text: String = ["basic", "lists", "quotes", "tables", "code"]
        .iter()
        .map(|name| std::fs::read_to_string(format!("tests/fixtures/{name}.md")).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = App::new(Source::Stdin, &text, None);
    app.set_sidebar(Some(false));
    let mut wide = Terminal::new(TestBackend::new(120, 30)).unwrap();
    screen(&mut app, &mut wide);
    for _ in 0..40 {
        press(&mut app, KeyCode::Char('j'));
    }
    let before = screen(&mut app, &mut wide);
    let first_line = content_line(&before, 0);

    let mut narrow = Terminal::new(TestBackend::new(60, 30)).unwrap();
    let after = screen(&mut app, &mut narrow);
    let word = first_line.split_whitespace().nth(1).unwrap_or("");
    assert!(
        !word.is_empty() && after.lines().take(3).any(|l| l.contains(word)),
        "{first_line:?} vs\n{after}"
    );
}

#[test]
fn uses_the_full_width_and_survives_tiny_terminals() {
    let text = std::fs::read_to_string("tests/fixtures/readme.md").unwrap();
    let mut app = App::new(Source::Stdin, &text, None);
    app.set_sidebar(Some(false));

    let mut wide = Terminal::new(TestBackend::new(200, 40)).unwrap();
    let out = screen(&mut app, &mut wide);
    let longest = out
        .lines()
        .map(|l| l.trim_end_matches(['"', ' ', '│', '█']).len())
        .max()
        .unwrap();
    assert!(longest > 150, "longest line only {longest} wide:\n{out}");

    let mut capped = App::new(Source::Stdin, &text, Some(80));
    capped.set_sidebar(Some(false));
    let out = screen(&mut capped, &mut wide);
    let title = content_line(&out, 0);
    assert!(title[1..].starts_with(&" ".repeat(50)), "{title}");

    for (w, h) in [(30, 10), (12, 4), (8, 2), (200, 3)] {
        let mut small = Terminal::new(TestBackend::new(w, h)).unwrap();
        screen(&mut app, &mut small);
        press(&mut app, KeyCode::Char('t'));
        screen(&mut app, &mut small);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('?'));
        screen(&mut app, &mut small);
        press(&mut app, KeyCode::Esc);
    }
    let mut short = Terminal::new(TestBackend::new(200, 3)).unwrap();
    let mut forced = App::new(Source::Stdin, &text, Some(80));
    forced.set_sidebar(Some(true));
    screen(&mut forced, &mut short);
}

#[test]
fn outline_sidebar_follows_and_navigates() {
    let text = std::fs::read_to_string("tests/fixtures/readme.md").unwrap();
    let mut app = App::new(Source::Stdin, &text, None);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    insta::assert_snapshot!("sidebar", screen(&mut app, &mut terminal));

    press(&mut app, KeyCode::Char('l'));
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j'));
    let out = screen(&mut app, &mut terminal);
    insta::assert_snapshot!("sidebar_focus", out.clone());
    let first = content_line(&out, 0);
    assert!(
        first.contains("Use"),
        "content should have jumped to the second section: {first}"
    );

    let before = out.matches('├').count();
    press(&mut app, KeyCode::Char('g'));
    press(&mut app, KeyCode::Char(' '));
    let folded = screen(&mut app, &mut terminal);
    assert!(
        before >= 3 && folded.matches('├').count() == 0,
        "{before} branches, then:\n{folded}"
    );
    assert!(folded.contains("▸ mido"), "{folded}");
    press(&mut app, KeyCode::Char('l'));
    let unfolded = screen(&mut app, &mut terminal);
    assert_eq!(unfolded.matches('├').count(), before);

    press(&mut app, KeyCode::Enter);
    let in_outline = |s: &str| s.contains("outline: j/k follow");
    assert!(!in_outline(&screen(&mut app, &mut terminal)));
    press(&mut app, KeyCode::Tab);
    assert!(
        in_outline(&screen(&mut app, &mut terminal)),
        "Tab should focus the outline"
    );
    press(&mut app, KeyCode::Tab);
    assert!(
        !in_outline(&screen(&mut app, &mut terminal)),
        "Tab should cycle back"
    );
    press(&mut app, KeyCode::BackTab);
    assert!(
        in_outline(&screen(&mut app, &mut terminal)),
        "Shift-Tab should cycle backwards"
    );
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    assert!(
        in_outline(&screen(&mut app, &mut terminal)),
        "Tab then Right from the outline stays there"
    );
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Left);
    assert!(
        !in_outline(&screen(&mut app, &mut terminal)),
        "Tab then Left from the outline goes to the body"
    );
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    assert!(
        in_outline(&screen(&mut app, &mut terminal)),
        "Tab then Right from the body goes to the outline"
    );
    press(&mut app, KeyCode::Enter);

    press(&mut app, KeyCode::Char('o'));
    let out = screen(&mut app, &mut terminal);
    assert!(!out.contains("Outline"), "{out}");
    press(&mut app, KeyCode::Tab);
    assert!(screen(&mut app, &mut terminal).contains("no other pane"));
    press(&mut app, KeyCode::Char('b'));
    assert!(screen(&mut app, &mut terminal).contains("open a folder"));

    let mut narrow = Terminal::new(TestBackend::new(80, 24)).unwrap();
    app = App::new(Source::Stdin, &text, None);
    let out = screen(&mut app, &mut narrow);
    assert!(
        !out.contains("Outline"),
        "sidebar should auto-hide under 100 columns"
    );
}

fn project_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("docs");
    std::fs::create_dir_all(root.join("guide")).unwrap();
    std::fs::write(
        root.join("README.md"),
        format!("# Home\n\nWelcome to the project. See [usage](#usage) and the [install guide](guide/install.md#steps).\n\n{filler}## Usage\n\nRead the guide.\n\n{filler}", filler = "Filler paragraph.\n\n".repeat(30)),
    )
    .unwrap();
    std::fs::write(
        root.join("guide/install.md"),
        format!("# Installing\n\nIntro.\n\n{filler}## Steps\n\nSteps go here. Back to [home](../README.md).\n\n{filler}", filler = "More intro.\n\n".repeat(30)),
    )
    .unwrap();
    std::fs::write(root.join("guide/faq.md"), "# FAQ\n\nQuestions.\n").unwrap();
    std::fs::write(root.join("notes.md"), "# Notes\n\nLoose notes.\n").unwrap();
    dir
}

fn open_project(dir: &tempfile::TempDir) -> App {
    let project = Project::scan(&dir.path().join("docs"));
    let file = project.entry_file();
    let source = Source::Project { project, file };
    let text = source.read().unwrap();
    App::new(source, &text, None)
}

#[test]
fn project_mode_shows_files_and_opens_them() {
    let dir = project_fixture();
    let mut app = open_project(&dir);
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let out = screen(&mut app, &mut terminal);
    assert!(has_files(&out) && out.contains("Outline"), "{out}");
    for name in ["guide", "install.md", "faq.md", "notes.md", "README.md"] {
        assert!(out.contains(name), "missing {name}:\n{out}");
    }
    assert!(out.contains("Welcome to the project"), "{out}");
    assert!(out.lines().last().unwrap().contains(" README.md "), "{out}");
    insta::assert_snapshot!("project", out);

    press(&mut app, KeyCode::Tab);
    assert!(
        screen(&mut app, &mut terminal).contains("outline: j/k follow"),
        "Tab goes right first"
    );
    press(&mut app, KeyCode::BackTab);
    press(&mut app, KeyCode::BackTab);
    assert!(
        screen(&mut app, &mut terminal).contains("files: j/k move"),
        "Shift-Tab twice reaches files"
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::BackTab);
    assert!(screen(&mut app, &mut terminal).contains("files: j/k move"));
    press(&mut app, KeyCode::Char('G'));
    press(&mut app, KeyCode::Char('k'));
    press(&mut app, KeyCode::Enter);
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("Loose notes"),
        "Enter should open notes.md:\n{out}"
    );
    assert!(out.lines().last().unwrap().contains(" notes.md "), "{out}");
    assert!(app.current_file().unwrap().ends_with("notes.md"));
    assert!(
        !out.contains("files: j/k move"),
        "Enter should hand focus back to the document"
    );

    press(&mut app, KeyCode::BackTab);
    press(&mut app, KeyCode::Char('g'));
    press(&mut app, KeyCode::Char(' '));
    let folded = screen(&mut app, &mut terminal);
    assert!(
        !folded.contains("install.md") && folded.contains("▸ guide"),
        "{folded}"
    );
    assert!(
        !folded.contains("▾ docs"),
        "the root is the title, not a row:\n{folded}"
    );
    assert!(
        has_files(&folded) && folded.contains("└   README.md"),
        "{folded}"
    );
    press(&mut app, KeyCode::Char(' '));
    assert!(screen(&mut app, &mut terminal).contains("install.md"));

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('b'));
    let out = screen(&mut app, &mut terminal);
    assert!(!has_files(&out), "{out}");
    assert!(out.contains("Outline"), "{out}");

    let mut narrow = Terminal::new(TestBackend::new(90, 24)).unwrap();
    let mut app = open_project(&dir);
    let out = screen(&mut app, &mut narrow);
    assert!(
        has_files(&out) && !out.contains("Outline"),
        "files first, outline only with room:\n{out}"
    );
}

#[test]
fn links_anchors_and_history() {
    let dir = project_fixture();
    let mut app = open_project(&dir);
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    screen(&mut app, &mut terminal);

    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Enter);
    let out = screen(&mut app, &mut terminal);
    assert!(
        content_line(&out, 0).contains("Usage"),
        "anchor should scroll to Usage:\n{out}"
    );

    press(&mut app, KeyCode::Char('H'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        content_line(&out, 0).contains("Home"),
        "back should return to the top:\n{out}"
    );

    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Enter);
    let out = screen(&mut app, &mut terminal);
    assert!(app.current_file().unwrap().ends_with("install.md"), "{out}");
    assert!(
        content_line(&out, 0).contains("Steps"),
        "should land on the Steps heading:\n{out}"
    );

    press(&mut app, KeyCode::Char('H'));
    assert!(app.current_file().unwrap().ends_with("README.md"));
    press(&mut app, KeyCode::Char('L'));
    assert!(app.current_file().unwrap().ends_with("install.md"));
    let out = screen(&mut app, &mut terminal);
    assert!(
        content_line(&out, 0).contains("Steps"),
        "forward restores the scroll:\n{out}"
    );

    press(&mut app, KeyCode::Char(']'));
    let out = screen(&mut app, &mut terminal);
    let (row, line) = out
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains("home"))
        .expect("home link visible");
    let column = (line.find("home").unwrap() - 1) as u16;
    app.mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row: row as u16,
        modifiers: KeyModifiers::NONE,
    });
    assert!(
        app.current_file().unwrap().ends_with("README.md"),
        "click should follow the relative link"
    );
    press(&mut app, KeyCode::Char('L'));
    press(&mut app, KeyCode::Char('L'));
    assert!(screen(&mut app, &mut terminal).contains("nothing to go forward to"));
}

#[test]
fn finder_opens_files_by_fuzzy_name() {
    let dir = project_fixture();
    let mut app = open_project(&dir);
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    screen(&mut app, &mut terminal);
    app.key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("Find file") && out.contains("guide/faq.md"),
        "{out}"
    );
    for c in "fq".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    let out = screen(&mut app, &mut terminal);
    assert!(out.contains("▸ guide/faq.md"), "{out}");
    insta::assert_snapshot!("finder", out);
    press(&mut app, KeyCode::Enter);
    let out = screen(&mut app, &mut terminal);
    assert!(app.current_file().unwrap().ends_with("faq.md"), "{out}");
    assert!(out.contains("Questions"), "{out}");
    press(&mut app, KeyCode::Char('H'));
    assert!(app.current_file().unwrap().ends_with("README.md"));
}

#[test]
fn focus_mode_hides_and_restores_panels() {
    let dir = project_fixture();
    let mut app = open_project(&dir);
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let one_line = |s: &str| {
        s.lines()
            .any(|l| l.contains("Welcome") && l.contains("#steps"))
    };
    let before = screen(&mut app, &mut terminal);
    assert!(has_files(&before) && before.contains("Outline"));
    assert!(
        !one_line(&before),
        "with panels the intro should wrap:\n{before}"
    );

    press(&mut app, KeyCode::Char('f'));
    let focused = screen(&mut app, &mut terminal);
    assert!(
        !has_files(&focused) && !focused.contains("Outline"),
        "{focused}"
    );
    assert!(
        one_line(&focused),
        "focus mode should use the full width:\n{focused}"
    );

    press(&mut app, KeyCode::Char('f'));
    let restored = screen(&mut app, &mut terminal);
    assert!(
        has_files(&restored) && restored.contains("Outline"),
        "{restored}"
    );

    press(&mut app, KeyCode::Char('f'));
    press(&mut app, KeyCode::Char('o'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("Outline") && has_files(&out),
        "o leaves focus mode:\n{out}"
    );
}

#[test]
fn scrollbar_thumb_reaches_both_ends() {
    let text: String = ["basic", "lists", "quotes", "tables", "code"]
        .iter()
        .map(|name| std::fs::read_to_string(format!("tests/fixtures/{name}.md")).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = App::new(Source::Stdin, &text, None);
    app.set_sidebar(Some(false));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let column = |s: &str| -> Vec<char> {
        s.lines()
            .take(23)
            .map(|l| l.trim_end_matches('"').chars().last().unwrap())
            .collect()
    };

    let top = column(&screen(&mut app, &mut terminal));
    assert_eq!(
        &top[0..2],
        [' ', ' '],
        "the top margin rows carry no scrollbar: {top:?}"
    );
    assert_eq!(top[2], '█', "thumb starts at the first text row: {top:?}");
    assert_eq!(
        top[20], '│',
        "track shows at the last text row when at the top: {top:?}"
    );
    assert_eq!(
        &top[21..23],
        [' ', ' '],
        "the bottom margin rows carry no scrollbar: {top:?}"
    );

    press(&mut app, KeyCode::Char('G'));
    let bottom = column(&screen(&mut app, &mut terminal));
    assert_eq!(
        bottom[20], '█',
        "thumb reaches the last text row: {bottom:?}"
    );
    assert_eq!(
        bottom[2], '│',
        "track shows at the top when at the bottom: {bottom:?}"
    );
}

#[test]
fn long_panel_entries_wrap_and_stay_clickable() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("docs");
    std::fs::create_dir_all(root.join("guide")).unwrap();
    std::fs::write(
        root.join("README.md"),
        "# Alpha beta gamma delta epsilon zeta eta theta iota kappa\n\nIntro.\n\n## Short\n\nText.\n",
    )
    .unwrap();
    let long = "a-very-long-file-name-that-will-certainly-need-wrapping.md";
    std::fs::write(root.join("guide").join(long), "# Long one\n\nFound it.\n").unwrap();
    let project = Project::scan(&root);
    let file = project.entry_file();
    let source = Source::Project { project, file };
    let text = source.read().unwrap();
    let mut app = App::new(source, &text, None);
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let out = screen(&mut app, &mut terminal);

    assert!(
        !out.lines().any(|l| l.contains(long)),
        "the name should not fit on one line:\n{out}"
    );
    assert!(
        out.contains("wrapping.md"),
        "the tail of the name should be visible:\n{out}"
    );
    let heading_lines: Vec<&str> = out
        .lines()
        .filter(|l| l.contains("kappa") || l.contains("Alpha beta"))
        .collect();
    assert!(
        out.lines()
            .any(|l| l.contains("kappa") && !l.contains("Alpha")),
        "the heading should wrap in the outline card: {heading_lines:?}"
    );

    let (row, line) = out
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains("wrapping.md"))
        .unwrap();
    let column = (line.find("wrapping.md").unwrap() - 1) as u16;
    app.mouse_event(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row: row as u16,
        modifiers: KeyModifiers::NONE,
    });
    assert!(
        app.current_file().unwrap().ends_with(long),
        "clicking a continuation line opens the file"
    );
    assert!(screen(&mut app, &mut terminal).contains("Found it"));
}

#[test]
fn footnote_popup_and_front_matter_toggle() {
    let text = std::fs::read_to_string("tests/fixtures/rich.md").unwrap();
    let mut app = App::new(Source::Stdin, &text, None);
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
    let out = screen(&mut app, &mut terminal);
    assert!(out.contains("▸  title  Rich content   tags  demo"), "{out}");

    press(&mut app, KeyCode::Char('m'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("▾ front matter") && out.contains("title: Rich content"),
        "{out}"
    );
    press(&mut app, KeyCode::Char('m'));
    assert!(screen(&mut app, &mut terminal).contains("▸  title  Rich content"));

    for _ in 0..3 {
        press(&mut app, KeyCode::Char(']'));
    }
    press(&mut app, KeyCode::Enter);
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains("Footnote ¹") && out.contains("The footnote body."),
        "footnote popup:\n{out}"
    );
    press(&mut app, KeyCode::Esc);
    assert!(!screen(&mut app, &mut terminal).contains("Footnote ¹"));
}

#[test]
fn images_render_as_halfblocks_and_toggle_off() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("images")).unwrap();
    let mut pixels = image::RgbImage::new(40, 40);
    for (_, y, p) in pixels.enumerate_pixels_mut() {
        *p = if (y / 10) % 2 == 0 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        };
    }
    pixels.save(dir.path().join("images/tiny.png")).unwrap();
    let page = dir.path().join("page.md");
    std::fs::write(
        &page,
        "# Pictures\n\n![A tiny square](images/tiny.png)\n\nAfter the picture.\n",
    )
    .unwrap();

    let text = std::fs::read_to_string(&page).unwrap();
    let mut app = App::new(Source::File(page), &text, None);
    app.set_picker(ratatui_image::picker::Picker::halfblocks());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let out = screen(&mut app, &mut terminal);
    assert!(
        out.contains('▀') || out.contains('▄'),
        "halfblocks should be drawn:\n{out}"
    );
    assert!(
        out.contains("A tiny square") && !out.contains("(images/tiny.png)"),
        "{out}"
    );
    assert!(out.contains("After the picture."), "{out}");

    press(&mut app, KeyCode::Char('i'));
    let out = screen(&mut app, &mut terminal);
    assert!(
        !out.contains('▀') && !out.contains('▄') && out.contains("(images/tiny.png)"),
        "placeholder after toggling images off:\n{out}"
    );
}

fn configured(text: &str, settings: mido::app::Settings) -> App {
    let mut app = App::with_settings(Source::Stdin, text, settings);
    app.set_sidebar(Some(false));
    app
}

#[test]
fn remapped_keys_drive_the_viewer_and_the_help() {
    let text = std::fs::read_to_string("tests/fixtures/readme.md").unwrap();
    let overrides = std::collections::BTreeMap::from([
        ("scroll_down".to_string(), vec!["n".to_string()]),
        ("help".to_string(), vec!["F1".to_string()]),
    ]);
    let settings = mido::app::Settings {
        keymap: mido::app::keys::Keymap::with_overrides(&overrides).unwrap(),
        ..Default::default()
    };
    let mut app = configured(&text, settings);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let top = screen(&mut app, &mut terminal);
    assert!(
        top.contains("F1 help"),
        "the status bar names the bound key"
    );

    press(&mut app, KeyCode::Char('j'));
    assert_eq!(screen(&mut app, &mut terminal), top, "j no longer scrolls");
    press(&mut app, KeyCode::Char('n'));
    assert_ne!(screen(&mut app, &mut terminal), top, "n scrolls instead");

    press(&mut app, KeyCode::F(1));
    let help = screen(&mut app, &mut terminal);
    assert!(help.contains("n / k, ↑, wheel"), "{help}");
}

#[test]
fn ascii_glyphs_draw_nothing_outside_ascii() {
    let text = std::fs::read_to_string("tests/fixtures/readme.md").unwrap();
    let theme = mido::render::theme::Theme::dark().with_glyphs(mido::render::glyphs::Glyphs::ASCII);
    let settings = mido::app::Settings {
        theme,
        ..Default::default()
    };
    let mut app = App::with_settings(Source::Stdin, &text, settings);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let view = screen(&mut app, &mut terminal);
    for symbol in ["│", "─", "╭", "•", "▸", "▾", "▎", "…", "☐"] {
        assert!(!view.contains(symbol), "{symbol} in\n{view}");
    }
    insta::assert_snapshot!("ascii", view);
}

#[test]
fn gutter_and_front_matter_follow_the_settings() {
    let text = "---\ntitle: Hi\n---\n\n# Heading\n\nBody text.\n";
    let settings = mido::app::Settings {
        gutter: 6,
        front_matter: mido::render::layout::FrontMatterView::Hidden,
        ..Default::default()
    };
    let mut app = configured(text, settings);
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    let view = screen(&mut app, &mut terminal);
    assert!(
        !view.contains("title"),
        "front matter starts hidden\n{view}"
    );
    assert!(
        content_line(&view, 0).starts_with("\"       Heading"),
        "{view}"
    );

    press(&mut app, KeyCode::Char('m'));
    assert!(screen(&mut app, &mut terminal).contains("front matter"));
    press(&mut app, KeyCode::Char('m'));
    assert!(
        !screen(&mut app, &mut terminal).contains("title"),
        "m returns to hidden"
    );
}
