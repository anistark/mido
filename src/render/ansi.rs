use std::io::{self, Write};

use crossterm::queue;
use crossterm::style::{Attribute, SetAttribute, SetBackgroundColor, SetForegroundColor};
use ratatui::backend::IntoCrossterm;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

pub fn write(lines: &[Line<'_>], out: &mut impl Write, color: bool) -> io::Result<()> {
    for line in lines {
        for span in &line.spans {
            if color {
                begin(out, span.style)?;
            }
            out.write_all(span.content.as_bytes())?;
            if color {
                queue!(out, SetAttribute(Attribute::Reset))?;
            }
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
