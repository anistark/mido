use std::io::{self, Write};

use crossterm::queue;
use crossterm::style::{
    Attribute, Color as CColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use ratatui::style::{Color, Modifier, Style};
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
        queue!(out, SetForegroundColor(convert(fg)))?;
    }
    if let Some(bg) = style.bg {
        queue!(out, SetBackgroundColor(convert(bg)))?;
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

fn convert(color: Color) -> CColor {
    match color {
        Color::Reset => CColor::Reset,
        Color::Black => CColor::Black,
        Color::Red => CColor::DarkRed,
        Color::Green => CColor::DarkGreen,
        Color::Yellow => CColor::DarkYellow,
        Color::Blue => CColor::DarkBlue,
        Color::Magenta => CColor::DarkMagenta,
        Color::Cyan => CColor::DarkCyan,
        Color::Gray => CColor::Grey,
        Color::DarkGray => CColor::DarkGrey,
        Color::LightRed => CColor::Red,
        Color::LightGreen => CColor::Green,
        Color::LightYellow => CColor::Yellow,
        Color::LightBlue => CColor::Blue,
        Color::LightMagenta => CColor::Magenta,
        Color::LightCyan => CColor::Cyan,
        Color::White => CColor::White,
        Color::Rgb(r, g, b) => CColor::Rgb { r, g, b },
        Color::Indexed(i) => CColor::AnsiValue(i),
    }
}
