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

#[test]
fn links_and_anchors_are_indexed() {
    let text = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
    let out = layout(&parse(&text), &Theme::dark(), 60);
    let lines = out.plain_lines();
    let link = out
        .links
        .iter()
        .find(|l| l.url == "https://example.com/docs")
        .expect("link indexed");
    let shown = &lines[link.line][..];
    let cols: String = shown
        .chars()
        .scan(0usize, |col, c| {
            let start = *col;
            *col += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            Some((start, c))
        })
        .filter(|(col, _)| (link.start as usize..link.end as usize).contains(col))
        .map(|(_, c)| c)
        .collect();
    assert!(cols.starts_with("link"), "{cols:?}");
    assert!(
        out.links.iter().any(|l| l.url == "https://example.com"),
        "autolink indexed"
    );
    assert!(out.link_at(link.line, link.start).is_some());
    assert!(
        out.link_at(link.line, link.end).is_none()
            || out.link_at(link.line, link.end).unwrap().url != link.url
    );

    assert_eq!(out.headings[1].slug, "second-level");
    assert_eq!(
        out.line_for_anchor("#Second-Level".trim_start_matches('#')),
        Some(out.headings[1].line)
    );
    assert_eq!(out.line_for_anchor("nope"), None);
}
