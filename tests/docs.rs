use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use clap::CommandFactory;
use mido::markdown::parse;
use mido::markdown::{Block, BlockKind, Inline};

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
        let output = Command::new(env!("CARGO_BIN_EXE_mido"))
            .args(["-p", "-w", "80"])
            .arg(&page)
            .env("NO_COLOR", "1")
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
            !text.contains("layout:"),
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
