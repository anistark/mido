use std::collections::HashMap;

use crate::markdown::{Block, BlockKind, Document, Inline};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeText {
    pub label: String,
    pub value: Option<String>,
}

pub type Badges = HashMap<String, BadgeText>;

const HOSTS: [&str; 7] = [
    "img.shields.io",
    "shields.io",
    "badgen.net",
    "flat.badgen.net",
    "badge.fury.io",
    "forthebadge.com",
    "deps.rs",
];

pub fn is_badge(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let path = path.split(['?', '#']).next().unwrap_or("");
    HOSTS.contains(&host)
        || path.split('/').any(|segment| {
            segment == "badge" || segment == "badges" || segment.starts_with("badge.")
        })
}

pub fn from_url(url: &str) -> Option<BadgeText> {
    let url = url.split('#').next().unwrap_or(url);
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let label = param(query, "label");
    let rest = path
        .strip_prefix("https://")
        .or_else(|| path.strip_prefix("http://"))?;
    let segments: Vec<&str> = rest.split('/').skip(1).filter(|s| !s.is_empty()).collect();
    let text = match segments.as_slice() {
        ["badge", content] => static_badge(&percent_decode(content)),
        ["static", "v1"] => BadgeText {
            label: label.clone().unwrap_or_default(),
            value: param(query, "message"),
        },
        _ => BadgeText {
            label: label.clone()?,
            value: None,
        },
    };
    let mut text = text;
    if let Some(label) = label {
        text.label = label;
    }
    Some(text)
}

fn static_badge(content: &str) -> BadgeText {
    let parts: Vec<String> = content
        .replace("--", "\u{1}")
        .replace("__", "\u{2}")
        .split('-')
        .map(|part| {
            part.replace('_', " ")
                .replace('\u{1}', "-")
                .replace('\u{2}', "_")
        })
        .collect();
    match parts.len() {
        0 => BadgeText {
            label: String::new(),
            value: None,
        },
        1 | 2 => BadgeText {
            label: parts[0].clone(),
            value: None,
        },
        n => BadgeText {
            label: parts[0].clone(),
            value: Some(parts[1..n - 1].join("-")),
        },
    }
}

pub fn from_svg(svg: &str) -> Option<BadgeText> {
    let start = svg.find("<title>")? + "<title>".len();
    let end = svg[start..].find("</title>")? + start;
    let title = unescape(svg[start..end].trim());
    if title.is_empty() {
        return None;
    }
    let (label, value) = match title.split_once(": ").or_else(|| title.split_once(" - ")) {
        Some((label, value)) => (label.to_string(), Some(value.to_string())),
        None => (title.clone(), None),
    };
    Some(BadgeText { label, value })
}

pub fn badge_urls(doc: &Document) -> Vec<String> {
    let mut urls = Vec::new();
    for block in &doc.blocks {
        walk_block(block, &mut urls);
    }
    for note in &doc.footnotes {
        for block in &note.blocks {
            walk_block(block, &mut urls);
        }
    }
    urls
}

fn walk_block(block: &Block, urls: &mut Vec<String>) {
    match &block.kind {
        BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => {
            walk_inlines(content, urls)
        }
        BlockKind::BlockQuote { blocks, .. } => blocks.iter().for_each(|b| walk_block(b, urls)),
        BlockKind::List(list) => {
            for item in &list.items {
                item.blocks.iter().for_each(|b| walk_block(b, urls));
            }
        }
        BlockKind::DefinitionList(defs) => {
            for def in defs {
                walk_inlines(&def.term, urls);
                for detail in &def.details {
                    detail.iter().for_each(|b| walk_block(b, urls));
                }
            }
        }
        BlockKind::Table(table) => {
            for cell in table.header.iter().chain(table.rows.iter().flatten()) {
                walk_inlines(cell, urls);
            }
        }
        BlockKind::CodeBlock { .. } | BlockKind::Rule | BlockKind::Html(_) => {}
    }
}

fn walk_inlines(inlines: &[Inline], urls: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Image { url, .. } => {
                if is_badge(url) && !urls.contains(url) {
                    urls.push(url.clone());
                }
            }
            Inline::Emphasis(c)
            | Inline::Strong(c)
            | Inline::Strikethrough(c)
            | Inline::Link { content: c, .. }
            | Inline::WikiLink { content: c, .. } => walk_inlines(c, urls),
            _ => {}
        }
    }
}

fn param(query: &str, name: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| percent_decode(&value.replace('+', " ")))
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(hi) = hex(bytes[i + 1])
            && let Some(lo) = hex(bytes[i + 2])
        {
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|d| d as u8)
}

fn unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(label: &str, value: Option<&str>) -> BadgeText {
        BadgeText {
            label: label.to_string(),
            value: value.map(str::to_string),
        }
    }

    #[test]
    fn recognises_badge_hosts_and_paths() {
        assert!(is_badge("https://img.shields.io/crates/v/mido"));
        assert!(is_badge(
            "https://github.com/o/r/actions/workflows/ci.yml/badge.svg"
        ));
        assert!(is_badge("https://docs.rs/mido/badge.svg?x=1"));
        assert!(is_badge(
            "https://codecov.io/gh/o/r/branch/main/graph/badge.svg"
        ));
        assert!(!is_badge("https://example.com/images/badger.png"));
        assert!(!is_badge("images/tiny.png"));
        assert!(!is_badge("badges/local.svg"));
    }

    #[test]
    fn static_badges_come_from_the_url() {
        assert_eq!(
            from_url("https://img.shields.io/badge/docs-anistark.github.io%2Fmido-2ea44f"),
            Some(text("docs", Some("anistark.github.io/mido")))
        );
        assert_eq!(
            from_url("https://img.shields.io/badge/made__with-rust--lang-orange?style=flat"),
            Some(text("made_with", Some("rust-lang")))
        );
        assert_eq!(
            from_url("https://img.shields.io/badge/just_a_message-blue"),
            Some(text("just a message", None))
        );
        assert_eq!(
            from_url("https://img.shields.io/static/v1?label=build&message=passing&color=green"),
            Some(text("build", Some("passing")))
        );
        assert_eq!(
            from_url("https://img.shields.io/crates/v/mido?label=version"),
            Some(text("version", None))
        );
        assert_eq!(from_url("https://img.shields.io/crates/v/mido"), None);
    }

    #[test]
    fn svg_titles_split_into_label_and_value() {
        assert_eq!(
            from_svg("<svg><title>crates.io: v0.4.0</title></svg>"),
            Some(text("crates.io", Some("v0.4.0")))
        );
        assert_eq!(
            from_svg("<svg>\n  <title>CI - passing</title>\n</svg>"),
            Some(text("CI", Some("passing")))
        );
        assert_eq!(
            from_svg("<svg><title>A &amp; B</title></svg>"),
            Some(text("A & B", None))
        );
        assert_eq!(from_svg("<svg></svg>"), None);
    }

    #[test]
    fn badge_urls_are_collected_from_anywhere_in_the_document() {
        let doc = crate::markdown::parse(
            "# Title [![a](https://img.shields.io/a)](https://x)\n\n![b](https://img.shields.io/b) ![p](pic.png)\n\n| h |\n| - |\n| ![c](https://docs.rs/x/badge.svg) |\n",
        );
        assert_eq!(
            badge_urls(&doc),
            [
                "https://img.shields.io/a",
                "https://img.shields.io/b",
                "https://docs.rs/x/badge.svg"
            ]
        );
    }
}
