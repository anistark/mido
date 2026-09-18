use mido::markdown::parse;
use mido::render::layout::layout;
use mido::render::theme::Theme;
use unicode_width::UnicodeWidthStr;

fn render(name: &str, width: usize) -> Vec<String> {
    let text = std::fs::read_to_string(format!("tests/fixtures/{name}.md")).unwrap();
    layout(&parse(&text), &Theme::dark(), width).plain_lines()
}

#[test]
fn snapshots_at_60_columns() {
    for name in ["basic", "lists", "tables", "quotes", "code"] {
        insta::assert_snapshot!(name, render(name, 60).join("\n"));
    }
}

#[test]
fn lines_never_exceed_the_width() {
    for name in ["basic", "lists", "quotes", "code"] {
        for width in [20, 33, 47, 80, 120] {
            for line in render(name, width) {
                assert!(
                    line.width() <= width,
                    "{name} at {width}: {line:?} is {} wide",
                    line.width()
                );
            }
        }
    }
}

#[test]
fn headings_index_points_at_their_lines() {
    let text = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
    let out = layout(&parse(&text), &Theme::dark(), 80);
    assert_eq!(out.headings.len(), 6);
    assert_eq!(out.headings[0].level, 1);
    assert_eq!(out.headings[0].text, "Basic document");
    let lines = out.plain_lines();
    assert!(lines[out.headings[1].line].contains("Second level"));
}

#[test]
fn source_map_round_trips() {
    let text = std::fs::read_to_string("tests/fixtures/lists.md").unwrap();
    let doc = parse(&text);
    let wide = layout(&doc, &Theme::dark(), 100);
    let narrow = layout(&doc, &Theme::dark(), 30);
    let anchor = wide.source_at(8).unwrap();
    let line = narrow.line_for_source(anchor);
    assert_eq!(narrow.source_at(line), Some(anchor));
}
