use std::collections::HashMap;

use mido::markdown::parse;
use mido::render::layout::{FrontMatterView, ImageSizes, Options, layout, layout_with};
use mido::render::theme::Theme;
use mido::render::wrap::LinkKind;
use unicode_width::UnicodeWidthStr;

fn render(name: &str, width: usize) -> Vec<String> {
    let text = std::fs::read_to_string(format!("tests/fixtures/{name}.md")).unwrap();
    layout(&parse(&text), &Theme::dark(), width).plain_lines()
}

#[test]
fn snapshots_at_60_columns() {
    for name in ["basic", "lists", "tables", "quotes", "code", "rich"] {
        insta::assert_snapshot!(name, render(name, 60).join("\n"));
    }
}

#[test]
fn rich_content_links_and_front_matter() {
    let text = std::fs::read_to_string("tests/fixtures/rich.md").unwrap();
    let doc = parse(&text);
    let out = layout(&doc, &Theme::dark(), 60);
    let kinds: Vec<(LinkKind, &str)> = out.links.iter().map(|l| (l.kind, l.url.as_str())).collect();
    assert!(
        kinds.contains(&(LinkKind::Wiki, "Getting Started")),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&(LinkKind::Wiki, "keys#reading")),
        "{kinds:?}"
    );
    assert!(kinds.contains(&(LinkKind::Footnote, "1")), "{kinds:?}");
    let plain = out.plain_lines().join("\n");
    assert!(plain.starts_with("▸ front matter: title, tags"), "{plain}");
    assert!(plain.contains("● Note") && plain.contains("▲ Warning"));
    assert!(plain.contains("🎉") && plain.contains("$e = mc^2$"));
    assert!(plain.contains("▣ A tiny square (images/tiny.png)"));

    let expanded = layout_with(
        &doc,
        &Theme::dark(),
        60,
        &Options {
            front_matter: FrontMatterView::Expanded,
            images: None,
        },
    );
    let plain = expanded.plain_lines().join("\n");
    assert!(
        plain.contains("▾ front matter") && plain.contains("title: Rich content"),
        "{plain}"
    );
    let hidden = layout_with(
        &doc,
        &Theme::dark(),
        60,
        &Options {
            front_matter: FrontMatterView::Hidden,
            images: None,
        },
    );
    assert!(!hidden.plain_lines().join("\n").contains("front matter"));
}

#[test]
fn images_reserve_rows_when_sizes_are_known() {
    let text = std::fs::read_to_string("tests/fixtures/rich.md").unwrap();
    let doc = parse(&text);
    let mut sizes = HashMap::new();
    sizes.insert("images/tiny.png".to_string(), (400u32, 200u32));
    let out = layout_with(
        &doc,
        &Theme::dark(),
        60,
        &Options {
            front_matter: FrontMatterView::Collapsed,
            images: Some(ImageSizes {
                font: (10, 20),
                sizes,
            }),
        },
    );
    assert_eq!(out.images.len(), 1);
    let slot = &out.images[0];
    assert_eq!((slot.cols, slot.rows), (40, 10));
    let lines = out.plain_lines();
    for (row, line) in lines
        .iter()
        .enumerate()
        .skip(slot.line)
        .take(slot.rows as usize)
    {
        assert!(
            line.trim().is_empty(),
            "row {row} should be reserved: {line:?}"
        );
    }
    assert!(lines[slot.line + slot.rows as usize].contains("A tiny square"));
    assert!(!lines.join("\n").contains("(images/tiny.png)"));
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

#[test]
fn links_inside_table_cells_are_indexed() {
    let md = "| Name | Where |\n|---|---|\n| mido | see the [docs](guide/docs.md) |\n";
    let out = layout(&parse(md), &Theme::dark(), 60);
    let link = out
        .links
        .iter()
        .find(|l| l.url == "guide/docs.md")
        .expect("table link indexed");
    let row = &out.plain_lines()[link.line];
    let text: String = row
        .chars()
        .scan(0usize, |col, c| {
            let start = *col;
            *col += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            Some((start, c))
        })
        .filter(|(col, _)| (link.start as usize..link.end as usize).contains(col))
        .map(|(_, c)| c)
        .collect();
    assert!(
        text.starts_with("docs"),
        "link columns should cover the link text: {text:?} in {row:?}"
    );
    assert!(out.link_at(link.line, link.start).is_some());
}

#[test]
fn mermaid_blocks_become_diagrams_or_fall_back() {
    let md = "```mermaid\ngraph LR\n  A[Read] --> B[Render]\n```\n\n```mermaid\nzzzDiagram\n  a -> b\n```\n";
    let out = layout(&parse(md), &Theme::dark(), 80);
    let text = out.plain_lines().join("\n");
    assert!(
        text.contains("┌") && (text.contains("►") || text.contains("▶")),
        "flowchart should be drawn:\n{text}"
    );
    assert!(
        !text.contains("A[Read]"),
        "drawn diagrams do not show their source:\n{text}"
    );
    assert!(
        text.contains("zzzDiagram"),
        "unsupported diagrams keep their source:\n{text}"
    );
    let narrow = layout(&parse(md), &Theme::dark(), 24);
    let narrow_text = narrow.plain_lines().join("\n");
    assert!(
        narrow_text.contains("A[Read]"),
        "too-wide drawings fall back to source:\n{narrow_text}"
    );
}
