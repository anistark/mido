use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::CommandFactory;
use mido::markdown::parse;
use mido::markdown::{Block, BlockKind, Inline};
use mido::render::layout::layout;
use mido::render::theme::Theme;

fn pages() -> Vec<PathBuf> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir("docs")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    pages.sort();
    pages
}

fn body(source: &str) -> &str {
    let Some(rest) = source.strip_prefix("---\n") else {
        return source;
    };
    rest.find("\n---\n").map_or(source, |end| &rest[end + 5..])
}

fn inlines_have_html(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Html(_) => true,
        Inline::Emphasis(c) | Inline::Strong(c) | Inline::Strikethrough(c) => inlines_have_html(c),
        Inline::Link { content, .. } => inlines_have_html(content),
        Inline::Image { alt, .. } => inlines_have_html(alt),
        _ => false,
    })
}

fn blocks_have_html(blocks: &[Block]) -> bool {
    blocks.iter().any(|block| match &block.kind {
        BlockKind::Html(_) => true,
        BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => {
            inlines_have_html(content)
        }
        BlockKind::BlockQuote { blocks, .. } => blocks_have_html(blocks),
        BlockKind::List(list) => list.items.iter().any(|item| blocks_have_html(&item.blocks)),
        BlockKind::DefinitionList(defs) => defs.iter().any(|def| {
            inlines_have_html(&def.term) || def.details.iter().any(|d| blocks_have_html(d))
        }),
        BlockKind::Table(table) => table
            .header
            .iter()
            .chain(table.rows.iter().flatten())
            .any(|cell| inlines_have_html(cell)),
        BlockKind::CodeBlock { .. } | BlockKind::Rule => false,
    })
}

fn inline_urls(inlines: &[Inline], out: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Link { content, url, .. } => {
                out.push(url.clone());
                inline_urls(content, out);
            }
            Inline::Image { alt, url } => {
                out.push(url.clone());
                inline_urls(alt, out);
            }
            Inline::Emphasis(c) | Inline::Strong(c) | Inline::Strikethrough(c) => {
                inline_urls(c, out)
            }
            _ => {}
        }
    }
}

fn block_urls(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match &block.kind {
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => {
                inline_urls(content, out)
            }
            BlockKind::BlockQuote { blocks, .. } => block_urls(blocks, out),
            BlockKind::List(list) => list.items.iter().for_each(|i| block_urls(&i.blocks, out)),
            BlockKind::DefinitionList(defs) => {
                for def in defs {
                    inline_urls(&def.term, out);
                    def.details.iter().for_each(|d| block_urls(d, out));
                }
            }
            BlockKind::Table(table) => table
                .header
                .iter()
                .chain(table.rows.iter().flatten())
                .for_each(|cell| inline_urls(cell, out)),
            BlockKind::CodeBlock { .. } | BlockKind::Rule | BlockKind::Html(_) => {}
        }
    }
}

fn heading_slugs(page: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(page).unwrap();
    layout(&parse(&text), &Theme::dark(), 80)
        .headings
        .into_iter()
        .map(|h| h.slug)
        .collect()
}

#[test]
fn every_internal_link_in_the_docs_resolves() {
    let mut corpus = pages();
    corpus.extend(["README.md", "CONTRIBUTING.md", "CHANGELOG.md"].map(PathBuf::from));
    let mut checked = 0;
    let mut anchors = 0;
    let mut failures = Vec::new();
    for page in &corpus {
        let doc = parse(&std::fs::read_to_string(page).unwrap());
        let mut urls = Vec::new();
        block_urls(&doc.blocks, &mut urls);
        doc.footnotes
            .iter()
            .for_each(|note| block_urls(&note.blocks, &mut urls));
        let base = page.parent().unwrap_or(Path::new("."));
        for url in urls {
            if url.contains("://") || url.starts_with("mailto:") || url.starts_with("tel:") {
                continue;
            }
            checked += 1;
            let (path_part, anchor) = url
                .split_once('#')
                .map_or((url.as_str(), None), |(p, a)| (p, Some(a)));
            let mut target = if path_part.is_empty() {
                page.clone()
            } else {
                base.join(path_part)
            };
            if target.is_dir() {
                target = target.join("README.md");
            }
            if !target.is_file() {
                failures.push(format!("{}: {url} is not a file", page.display()));
                continue;
            }
            let Some(anchor) = anchor else { continue };
            if target.extension().is_some_and(|ext| ext == "md") {
                anchors += 1;
                let slugs = heading_slugs(&target);
                if !slugs.iter().any(|slug| slug == anchor) {
                    failures.push(format!(
                        "{}: {url} has no heading #{anchor}, headings are {slugs:?}",
                        page.display()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "broken links:\n{}",
        failures.join("\n")
    );
    assert!(
        checked >= 10 && anchors >= 3,
        "checked {checked} links and {anchors} anchors"
    );
}

#[test]
fn keys_page_matches_the_keymap() {
    let path = "docs/keys.md";
    let file = std::fs::read_to_string(path).unwrap();
    let start = file.find("\n## ").expect("docs/keys.md has key sections") + 1;
    let generated = mido::app::keys::markdown();
    if std::env::var_os("MIDO_UPDATE_DOCS").is_some() {
        std::fs::write(path, format!("{}{generated}", &file[..start])).unwrap();
        return;
    }
    assert_eq!(
        &file[start..],
        generated,
        "docs/keys.md is out of date with src/app/keys.rs, run `just keys`"
    );
}

#[test]
fn every_flag_is_documented() {
    let doc = std::fs::read_to_string("docs/cli.md").unwrap();
    for arg in mido::cli::Cli::command().get_arguments() {
        if arg.is_hide_set() {
            continue;
        }
        if let Some(long) = arg.get_long() {
            assert!(
                doc.contains(&format!("--{long}`")),
                "docs/cli.md lacks --{long}"
            );
        }
        if let Some(short) = arg.get_short() {
            assert!(
                doc.contains(&format!("`-{short}`")),
                "docs/cli.md lacks -{short}"
            );
        }
    }
    for word in ["mido docs", "`-` for stdin", "--man"] {
        assert!(doc.contains(word), "docs/cli.md lacks {word}");
    }
}

#[test]
fn every_page_prints_without_site_only_syntax() {
    for page in pages() {
        let source = std::fs::read_to_string(&page).unwrap();
        let body = body(&source);
        for tag in ["{{", "{%"] {
            assert!(
                !body.contains(tag),
                "{} uses template syntax {tag}",
                page.display()
            );
        }
        let doc = parse(&source);
        assert!(
            !blocks_have_html(&doc.blocks),
            "{} contains raw HTML",
            page.display()
        );
        let no_config = std::env::temp_dir().join("mido-tests-no-config");
        let output = Command::new(env!("CARGO_BIN_EXE_mido"))
            .args(["-p", "-w", "80"])
            .arg(&page)
            .env("NO_COLOR", "1")
            .env("MIDO_CONFIG_DIR", &no_config)
            .output()
            .unwrap();
        assert!(output.status.success(), "mido -p {} failed", page.display());
        let text = String::from_utf8_lossy(&output.stdout);
        let title = body
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .unwrap_or_else(|| panic!("{} has no title", page.display()));
        assert!(
            text.contains(title),
            "{} did not render its title",
            page.display()
        );
        assert!(
            !text
                .lines()
                .any(|line| line.trim_start().starts_with("layout:")),
            "{} leaked its front matter",
            page.display()
        );
    }
}

#[test]
fn bundled_docs_include_every_page() {
    let bundled: BTreeSet<String> = mido::docs::pages().map(str::to_string).collect();
    let on_disk: BTreeSet<String> = pages()
        .iter()
        .filter_map(|page| page.file_name()?.to_str().map(str::to_string))
        .collect();
    assert_eq!(
        bundled, on_disk,
        "the bundled docs differ from docs/*.md, run `touch build.rs` and build again"
    );
}

#[test]
fn bundled_docs_extract_to_a_project() {
    let root = mido::docs::extract().unwrap();
    assert!(root.join("index.md").is_file());
    assert!(root.parent().unwrap().join("CHANGELOG.md").is_file());
    let project = mido::project::Project::scan(&root);
    assert_eq!(
        project.entry_file().as_deref(),
        Some(std::path::Path::new("index.md"))
    );
}

#[test]
fn themes_page_matches_the_themes() {
    let path = "docs/themes.md";
    let file = std::fs::read_to_string(path).unwrap();
    let start = file
        .find("\n## Built-in themes")
        .expect("docs/themes.md has a built-in themes section")
        + 1;
    let generated = mido::render::theme::markdown();
    if std::env::var_os("MIDO_UPDATE_DOCS").is_some() {
        std::fs::write(path, format!("{}{generated}", &file[..start])).unwrap();
        return;
    }
    assert_eq!(
        &file[start..],
        generated,
        "docs/themes.md is out of date with the built-in themes, run `just keys`"
    );
}

#[test]
fn config_and_theme_flags_reach_print_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        "glyphs = \"ascii\"\nwidth = 40\n",
    )
    .unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_mido"))
            .args(args)
            .env("MIDO_CONFIG_DIR", dir.path())
            .env_remove("NO_COLOR")
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    let (ok, text, _) = run(&["tests/fixtures/lists.md"]);
    assert!(ok);
    assert!(text.is_ascii(), "the ascii tier from the config\n{text}");
    assert!(
        text.lines().all(|l| l.chars().count() <= 40),
        "width 40\n{text}"
    );

    let (ok, text, _) = run(&["-p", "-t", "nord", "tests/fixtures/basic.md"]);
    assert!(
        ok && text.contains("\u{1b}["),
        "print mode colors with a theme"
    );

    let (ok, _, err) = run(&["-t", "no-such-theme", "tests/fixtures/basic.md"]);
    assert!(!ok && err.contains("no theme `no-such-theme`"), "{err}");

    let (ok, text, _) = run(&["themes"]);
    assert!(
        ok && text.contains("tokyo-night") && text.contains("default dark"),
        "{text}"
    );
}
