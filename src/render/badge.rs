use std::collections::HashMap;

use crate::markdown::{Block, BlockKind, Document, Inline};

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BadgeText {
    pub label: String,
    pub value: Option<String>,
    pub color: Option<Rgb>,
    pub label_color: Option<Rgb>,
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
    let mut text = match segments.as_slice() {
        ["badge", content] => static_badge(&percent_decode(content)),
        ["static", "v1"] => BadgeText {
            label: label.clone().unwrap_or_default(),
            value: param(query, "message"),
            ..BadgeText::default()
        },
        _ => BadgeText {
            label: label.clone()?,
            ..BadgeText::default()
        },
    };
    if let Some(label) = label {
        text.label = label;
    }
    if let Some(color) = param(query, "color").and_then(|c| parse_color(&c)) {
        text.color = Some(color);
    }
    if let Some(color) = param(query, "labelColor").and_then(|c| parse_color(&c)) {
        text.label_color = Some(color);
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
        0 => BadgeText::default(),
        1 => BadgeText {
            label: parts[0].clone(),
            ..BadgeText::default()
        },
        2 => BadgeText {
            label: parts[0].clone(),
            color: parse_color(&parts[1]),
            ..BadgeText::default()
        },
        n => BadgeText {
            label: parts[0].clone(),
            value: Some(parts[1..n - 1].join("-")),
            color: parse_color(&parts[n - 1]),
            label_color: None,
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
    Some(BadgeText {
        label,
        value,
        color: svg_color(svg),
        label_color: None,
    })
}

fn svg_color(svg: &str) -> Option<Rgb> {
    let mut fills = Vec::new();
    for tag in svg.split('<').skip(1) {
        let Some(tag) = tag
            .strip_prefix("rect")
            .or_else(|| tag.strip_prefix("path"))
        else {
            continue;
        };
        let tag = tag.split('>').next().unwrap_or("");
        if attr(tag, "rx").is_some() {
            continue;
        }
        let Some(color) = attr(tag, "fill").and_then(|fill| resolve_fill(svg, fill)) else {
            continue;
        };
        let offset = attr(tag, "x").is_some_and(|x| x.parse::<f64>().unwrap_or(0.0) > 0.0);
        fills.push((offset, color));
    }
    fills
        .iter()
        .find(|(offset, _)| *offset)
        .or_else(|| fills.get(1))
        .map(|(_, color)| *color)
}

fn resolve_fill(svg: &str, fill: &str) -> Option<Rgb> {
    let Some(id) = fill.strip_prefix("url(#").and_then(|f| f.strip_suffix(')')) else {
        return parse_color(fill);
    };
    let at = svg.find(&format!("id=\"{id}\""))?;
    let stop = svg[at..].split("<stop").nth(1)?;
    let stop = stop.split('>').next().unwrap_or("");
    if attr(stop, "stop-opacity").is_some() {
        return None;
    }
    parse_color(attr(stop, "stop-color")?)
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let at = tag.find(&format!(" {name}=\""))? + name.len() + 3;
    tag[at..].split('"').next()
}

pub fn parse_color(text: &str) -> Option<Rgb> {
    let text = text.trim().trim_start_matches('#').to_ascii_lowercase();
    let hex = match text.as_str() {
        "brightgreen" | "success" => "4c1",
        "green" => "97ca00",
        "yellow" => "dfb317",
        "yellowgreen" => "a4a61d",
        "orange" | "important" => "fe7d37",
        "red" | "critical" => "e05d44",
        "blue" | "informational" => "007ec6",
        "grey" | "gray" => "555",
        "lightgrey" | "lightgray" | "inactive" => "9f9f9f",
        "black" => "000",
        "white" => "fff",
        "silver" => "c0c0c0",
        "darkgrey" | "darkgray" | "dimgrey" | "dimgray" => "696969",
        "navy" => "000080",
        "darkblue" => "00008b",
        "royalblue" => "4169e1",
        "dodgerblue" => "1e90ff",
        "deepskyblue" => "00bfff",
        "skyblue" => "87ceeb",
        "steelblue" => "4682b4",
        "cornflowerblue" => "6495ed",
        "lightblue" => "add8e6",
        "teal" => "008080",
        "cyan" | "aqua" => "00ffff",
        "turquoise" => "40e0d0",
        "darkgreen" => "006400",
        "forestgreen" => "228b22",
        "seagreen" => "2e8b57",
        "limegreen" => "32cd32",
        "lime" => "00ff00",
        "lightgreen" => "90ee90",
        "olive" => "808000",
        "gold" => "ffd700",
        "goldenrod" => "daa520",
        "khaki" => "f0e68c",
        "darkorange" => "ff8c00",
        "orangered" => "ff4500",
        "tomato" => "ff6347",
        "coral" => "ff7f50",
        "salmon" => "fa8072",
        "crimson" => "dc143c",
        "firebrick" => "b22222",
        "darkred" => "8b0000",
        "maroon" => "800000",
        "brown" => "a52a2a",
        "chocolate" => "d2691e",
        "sienna" => "a0522d",
        "pink" => "ffc0cb",
        "hotpink" => "ff69b4",
        "deeppink" => "ff1493",
        "magenta" | "fuchsia" => "ff00ff",
        "purple" => "800080",
        "indigo" => "4b0082",
        "violet" => "ee82ee",
        "blueviolet" => "8a2be2",
        "orchid" => "da70d6",
        "plum" => "dda0dd",
        "lavender" => "e6e6fa",
        other => other,
    };
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    match digits.as_slice() {
        [r, g, b] => Some((r * 17, g * 17, b * 17)),
        [r1, r2, g1, g2, b1, b2] => Some((r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2)),
        _ => None,
    }
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

    fn text(label: &str, value: Option<&str>, color: Option<Rgb>) -> BadgeText {
        BadgeText {
            label: label.to_string(),
            value: value.map(str::to_string),
            color,
            label_color: None,
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
            Some(text(
                "docs",
                Some("anistark.github.io/mido"),
                Some((0x2e, 0xa4, 0x4f))
            ))
        );
        assert_eq!(
            from_url("https://img.shields.io/badge/made__with-rust--lang-orange?style=flat"),
            Some(text(
                "made_with",
                Some("rust-lang"),
                Some((0xfe, 0x7d, 0x37))
            ))
        );
        assert_eq!(
            from_url("https://img.shields.io/badge/just_a_message-blue"),
            Some(text("just a message", None, Some((0x00, 0x7e, 0xc6))))
        );
        assert_eq!(
            from_url(
                "https://img.shields.io/static/v1?label=build&message=passing&color=brightgreen"
            ),
            Some(text("build", Some("passing"), Some((0x44, 0xcc, 0x11))))
        );
        assert_eq!(
            from_url(
                "https://img.shields.io/crates/v/mido?label=version&color=%23123456&labelColor=000"
            ),
            Some(BadgeText {
                label: "version".to_string(),
                value: None,
                color: Some((0x12, 0x34, 0x56)),
                label_color: Some((0, 0, 0)),
            })
        );
        assert_eq!(from_url("https://img.shields.io/crates/v/mido"), None);
    }

    #[test]
    fn svg_titles_and_fills_are_read() {
        let shields = "<svg><title>crates.io: v0.4.0</title><linearGradient id=\"s\"><stop offset=\"0\" stop-color=\"#bbb\" stop-opacity=\".1\"/></linearGradient><rect width=\"102\" height=\"20\" rx=\"3\"/><rect width=\"57\" height=\"20\" fill=\"#555\"/><rect x=\"57\" width=\"45\" height=\"20\" fill=\"#ea7233\"/><rect width=\"102\" height=\"20\" fill=\"url(#s)\"/></svg>";
        assert_eq!(
            from_svg(shields),
            Some(text("crates.io", Some("v0.4.0"), Some((0xea, 0x72, 0x33))))
        );
        let github = "<svg>\n  <title>CI - passing</title>\n  <linearGradient id=\"workflow-fill\"><stop stop-color=\"#444D56\" offset=\"0%\"></stop></linearGradient>\n  <linearGradient id=\"state-fill\"><stop stop-color=\"#34D058\" offset=\"0%\"></stop><stop stop-color=\"#28A745\" offset=\"100%\"></stop></linearGradient>\n  <path id=\"workflow-bg\" fill=\"url(#workflow-fill)\" d=\"M0\"></path>\n  <path id=\"state-bg\" fill=\"url(#state-fill)\" d=\"M1\"></path>\n</svg>";
        assert_eq!(
            from_svg(github),
            Some(text("CI", Some("passing"), Some((0x34, 0xd0, 0x58))))
        );
        assert_eq!(
            from_svg("<svg><title>A &amp; B</title></svg>"),
            Some(text("A & B", None, None))
        );
        assert_eq!(from_svg("<svg></svg>"), None);
    }

    #[test]
    fn colors_parse_from_names_and_hex() {
        assert_eq!(parse_color("4c1"), Some((0x44, 0xcc, 0x11)));
        assert_eq!(parse_color("#2EA44F"), Some((0x2e, 0xa4, 0x4f)));
        assert_eq!(parse_color("critical"), Some((0xe0, 0x5d, 0x44)));
        assert_eq!(parse_color("blueviolet"), Some((0x8a, 0x2b, 0xe2)));
        assert_eq!(parse_color("not-a-color"), None);
        assert_eq!(parse_color("12345"), None);
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
