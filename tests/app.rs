use crossterm::event::KeyCode;
use mido::app::{App, Source};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn screen(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String {
    terminal.draw(|frame| app.draw(frame)).unwrap();
    terminal.backend().to_string()
}

fn press(app: &mut App, code: KeyCode) {
    app.press(code);
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
    let first_line = before.lines().next().unwrap().to_string();

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
    let title = out.lines().next().unwrap();
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

    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j'));
    let out = screen(&mut app, &mut terminal);
    insta::assert_snapshot!("sidebar_focus", out.clone());
    let first = out.lines().next().unwrap();
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

    press(&mut app, KeyCode::Char('b'));
    let out = screen(&mut app, &mut terminal);
    assert!(!out.contains("Outline"), "{out}");
    press(&mut app, KeyCode::Tab);
    assert!(screen(&mut app, &mut terminal).contains("no other pane"));

    let mut narrow = Terminal::new(TestBackend::new(80, 24)).unwrap();
    app = App::new(Source::Stdin, &text, None);
    let out = screen(&mut app, &mut narrow);
    assert!(
        !out.contains("Outline"),
        "sidebar should auto-hide under 100 columns"
    );
}
