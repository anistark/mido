use std::fmt::Write;

use mido::app::{App, Settings, Source};
use mido::render::theme::{BUILTIN, Kind, Theme};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

const COLS: u16 = 104;
const ROWS: u16 = 50;
const CELL_W: f32 = 9.0;
const CELL_H: f32 = 19.0;

fn screen(theme: Theme) -> Buffer {
    let text = std::fs::read_to_string("tests/fixtures/gallery.md").unwrap();
    let settings = Settings {
        theme,
        ..Settings::default()
    };
    let mut app = App::with_settings(Source::Stdin, &text, settings);
    let mut terminal = Terminal::new(TestBackend::new(COLS, ROWS)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    terminal.backend().buffer().clone()
}

/// The terminal colors a theme on `default` falls back to in the picture.
fn ground(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Dark => ("#1c1c26", "#d0d0d8"),
        Kind::Light => ("#fbfbf8", "#2a2a30"),
    }
}

fn hex(color: Option<Color>, fallback: &str) -> String {
    match color {
        Some(Color::Rgb(r, g, b)) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => fallback.to_string(),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn svg(name: &str, theme: &Theme, buffer: &Buffer) -> String {
    let (bg, fg) = ground(theme.kind);
    let width = f32::from(COLS) * CELL_W;
    let height = f32::from(ROWS) * CELL_H;
    let mut out = String::new();
    writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" aria-label="mido in the {name} theme">"#
    )
    .unwrap();
    writeln!(out, r#"<rect width="100%" height="100%" fill="{bg}"/>"#).unwrap();
    writeln!(
        out,
        r#"<g font-family="Menlo, 'DejaVu Sans Mono', 'Cascadia Mono', monospace" font-size="15" xml:space="preserve">"#
    )
    .unwrap();
    for y in 0..ROWS {
        let top = f32::from(y) * CELL_H;
        let mut x = 0;
        while x < COLS {
            let cell = &buffer[(x, y)];
            let fill = hex(cell.bg.into(), bg);
            let start = x;
            while x < COLS && hex(buffer[(x, y)].bg.into(), bg) == fill {
                x += 1;
            }
            if fill != bg {
                writeln!(
                    out,
                    r#"<rect x="{}" y="{top}" width="{}" height="{CELL_H}" fill="{fill}"/>"#,
                    f32::from(start) * CELL_W,
                    f32::from(x - start) * CELL_W
                )
                .unwrap();
            }
        }
        let baseline = top + CELL_H * 0.75;
        let mut x = 0;
        while x < COLS {
            let cell = &buffer[(x, y)];
            if cell.symbol().trim().is_empty() {
                x += 1;
                continue;
            }
            let color = hex(cell.fg.into(), fg);
            let modifier = cell.modifier;
            let start = x;
            let mut text = String::new();
            while x < COLS {
                let next = &buffer[(x, y)];
                if next.symbol().trim().is_empty()
                    || hex(next.fg.into(), fg) != color
                    || next.modifier != modifier
                {
                    break;
                }
                text.push_str(next.symbol());
                x += 1;
            }
            let cells = x - start;
            let mut attrs = String::new();
            if modifier.contains(Modifier::BOLD) {
                attrs.push_str(r#" font-weight="bold""#);
            }
            if modifier.contains(Modifier::ITALIC) {
                attrs.push_str(r#" font-style="italic""#);
            }
            if modifier.contains(Modifier::UNDERLINED) {
                attrs.push_str(r#" text-decoration="underline""#);
            }
            writeln!(
                out,
                r#"<text x="{}" y="{baseline}" fill="{color}" textLength="{}" lengthAdjust="spacingAndGlyphs"{attrs}>{}</text>"#,
                f32::from(start) * CELL_W,
                f32::from(cells) * CELL_W,
                escape(&text)
            )
            .unwrap();
        }
    }
    out.push_str("</g>\n</svg>\n");
    out
}

fn index(names: &[&str]) -> String {
    let mut out = String::from(concat!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
        "<title>mido theme gallery</title>\n<style>\n",
        "body{margin:0;padding:24px 16px;background:#0e1116;color:#d0d0d8;font-family:system-ui,sans-serif}\n",
        "main{max-width:960px;margin:0 auto}figure{margin:0 0 40px}img{width:100%;height:auto;border-radius:8px}\n",
        "figcaption{font-family:ui-monospace,Menlo,monospace;margin:0 0 8px}code{color:#5cffb0}\n",
        "</style>\n</head>\n<body>\n<main>\n<h1>mido theme gallery</h1>\n",
        "<p>Every built-in theme, drawn from the same screen buffer the snapshot tests use. ",
        "<code>mido-dark</code> and <code>mido-light</code> keep the terminal's own background, ",
        "shown here as a plain dark or light ground.</p>\n",
    ));
    for name in names {
        writeln!(
            out,
            "<figure><figcaption><code>--theme {name}</code></figcaption><img src=\"{name}.svg\" alt=\"mido in the {name} theme\" loading=\"lazy\"></figure>"
        )
        .unwrap();
    }
    out.push_str("</main>\n</body>\n</html>\n");
    out
}

#[test]
fn every_builtin_theme_draws_its_own_screen() {
    let target = std::env::var_os("MIDO_GALLERY").map(std::path::PathBuf::from);
    if let Some(dir) = &target {
        std::fs::create_dir_all(dir).unwrap();
    }
    let mut seen = Vec::new();
    for (name, _) in BUILTIN {
        let theme = Theme::builtin(name).unwrap();
        let buffer = screen(theme.clone());
        let picture = svg(name, &theme, &buffer);
        assert!(
            picture.contains(">Field<") && picture.contains(">Checklist<"),
            "{name} draws the document"
        );
        let accent = hex(Some(theme.accent), "");
        assert!(
            picture.contains(&accent),
            "{name} paints its accent {accent}"
        );
        assert!(!seen.contains(&picture), "{name} looks like another theme");
        if let Some(dir) = &target {
            std::fs::write(dir.join(format!("{name}.svg")), &picture).unwrap();
        }
        seen.push(picture);
    }
    if let Some(dir) = &target {
        let names: Vec<&str> = BUILTIN.iter().map(|(n, _)| *n).collect();
        std::fs::write(dir.join("index.html"), index(&names)).unwrap();
    }
}
