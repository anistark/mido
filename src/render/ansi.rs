use std::io::{self, Write};

use crossterm::queue;
use crossterm::style::{Attribute, SetAttribute, SetBackgroundColor, SetForegroundColor};
use ratatui::backend::IntoCrossterm;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use unicode_width::UnicodeWidthChar;

use super::layout::Link;

/// Writes the laid-out lines. With `color`, links whose target `url` resolves are wrapped in
/// OSC 8 hyperlinks, so terminals that support them make the text clickable. Control
/// characters in the text and the URLs are dropped, so a document cannot send the terminal
/// escape sequences of its own.
pub fn write(
    lines: &[Line<'_>],
    links: &[Link],
    out: &mut impl Write,
    color: bool,
    url: impl Fn(&Link) -> Option<String>,
) -> io::Result<()> {
    for (number, line) in lines.iter().enumerate() {
        let links: Vec<(usize, usize, String)> = if color {
            links
                .iter()
                .filter(|l| l.line == number)
                .filter_map(|l| {
                    let url: String = url(l)?.chars().filter(|c| !c.is_control()).collect();
                    Some((usize::from(l.start), usize::from(l.end), url))
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut col = 0;
        let mut open: Option<usize> = None;
        for span in &line.spans {
            if color {
                begin(out, span.style)?;
            }
            for c in span.content.chars().filter(|c| !c.is_control()) {
                if open.is_none()
                    && let Some(i) = links
                        .iter()
                        .position(|(start, end, _)| *start <= col && col < *end)
                {
                    write!(out, "\x1b]8;;{}\x1b\\", links[i].2)?;
                    open = Some(i);
                }
                write!(out, "{c}")?;
                col += c.width().unwrap_or(0);
                if open.is_some_and(|i| col >= links[i].1) {
                    out.write_all(b"\x1b]8;;\x1b\\")?;
                    open = None;
                }
            }
            if color {
                queue!(out, SetAttribute(Attribute::Reset))?;
            }
        }
        if open.is_some() {
            out.write_all(b"\x1b]8;;\x1b\\")?;
        }
        out.write_all(b"\n")?;
    }
    out.flush()
}

fn begin(out: &mut impl Write, style: Style) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(out, SetForegroundColor(fg.into_crossterm()))?;
    }
    if let Some(bg) = style.bg {
        queue!(out, SetBackgroundColor(bg.into_crossterm()))?;
    }
    let attrs = [
        (Modifier::BOLD, Attribute::Bold),
        (Modifier::DIM, Attribute::Dim),
        (Modifier::ITALIC, Attribute::Italic),
        (Modifier::UNDERLINED, Attribute::Underlined),
        (Modifier::CROSSED_OUT, Attribute::CrossedOut),
    ];
    for (modifier, attr) in attrs {
        if style.add_modifier.contains(modifier) {
            queue!(out, SetAttribute(attr))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;
    use crate::render::layout::layout;
    use crate::render::theme::Theme;

    fn print(text: &str, color: bool) -> String {
        let out = layout(&parse(text), &Theme::dark(), 60);
        let mut bytes = Vec::new();
        write(&out.lines, &out.links, &mut bytes, color, |link| {
            link.url.contains("://").then(|| link.url.clone())
        })
        .unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn links_become_osc_8_hyperlinks_when_styled() {
        let text = print("See [the site](https://x.y) and [a file](notes.md).", true);
        assert!(text.contains("\x1b]8;;https://x.y\x1b\\"), "{text:?}");
        assert_eq!(
            text.matches("\x1b]8;;").count(),
            2,
            "one open and one close"
        );
        assert!(!text.contains("notes.md\x1b\\"));
        let plain = print("See [the site](https://x.y).", false);
        assert!(!plain.contains("\x1b]8"));
    }

    #[test]
    fn escape_sequences_in_the_document_never_reach_the_terminal() {
        let text = "title \x1b]0;owned\x07, clipboard \x1b]52;c;aGk=\x07 and \u{9b}2J\n\n\
                    ```\ncode \x1b[2J\n```\n";
        for color in [true, false] {
            let out = print(text, color);
            for sequence in ["\x1b]0;", "\x1b]52;", "\x1b[2J", "\x07", "\u{9b}"] {
                assert!(!out.contains(sequence), "{sequence:?} in {out:?}");
            }
            assert!(out.contains("clipboard"));
        }
        let out = layout(&parse("[x](https://x.y)"), &Theme::dark(), 60);
        let mut bytes = Vec::new();
        write(&out.lines, &out.links, &mut bytes, true, |_| {
            Some("https://x.y\x1b\\\x1b]52;c;aGk=\x07".to_string())
        })
        .unwrap();
        let out = String::from_utf8(bytes).unwrap();
        assert!(!out.contains("\x1b]52;"), "{out:?}");
        assert_eq!(out.matches("\x1b]8;;").count(), 2, "{out:?}");
    }
}
