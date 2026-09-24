use std::io::{self, Write};

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::config::Config;
use crate::render::ansi;
use crate::render::theme::{ColorMode, Theme};

const SWATCH: [fn(&Theme) -> ratatui::style::Color; 8] = [
    |t| t.h1,
    |t| t.h2,
    |t| t.h3,
    |t| t.link,
    |t| t.code_fg,
    |t| t.alert_tip,
    |t| t.alert_warning,
    |t| t.alert_caution,
];

/// `mido themes`: every theme with its kind, a swatch of its colors, and where it comes from.
pub fn list(config: &Config, mode: ColorMode, out: &mut impl Write) -> io::Result<()> {
    let entries = config.theme_list();
    let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let chosen = config.theme.as_ref().map(|t| t.value.as_str());
    let mut lines = Vec::new();
    for entry in &entries {
        let mark = match chosen {
            Some(name) if name == entry.name => "*",
            _ => " ",
        };
        let mut spans = vec![Span::raw(format!(" {mark} {:<name_width$}  ", entry.name))];
        match &entry.theme {
            Ok(theme) => {
                let theme = theme.clone().with_mode(mode);
                spans.push(Span::raw(format!("{:<5}  ", theme.kind)));
                if mode != ColorMode::Mono {
                    let ground = Style::new().bg(theme.bg);
                    spans.push(Span::styled(" ", ground));
                    for color in SWATCH {
                        spans.push(Span::styled("■", ground.fg(color(&theme))));
                    }
                    spans.push(Span::styled(" ", ground));
                    spans.push(Span::raw("  "));
                }
                let mut notes = Vec::new();
                if chosen.is_none() && entry.name == config.dark_theme.value {
                    notes.push("default dark".to_string());
                }
                if chosen.is_none() && entry.name == config.light_theme.value {
                    notes.push("default light".to_string());
                }
                notes.push(match &entry.path {
                    Some(path) => path.display().to_string(),
                    None => "built in".to_string(),
                });
                spans.push(Span::raw(notes.join(", ")));
            }
            Err(error) => spans.push(Span::raw(format!("error: {error}"))),
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::default());
    lines.push(Line::raw(
        " Pick one with --theme <name>, or theme = \"<name>\" in the config.",
    ));
    if let Some(dir) = &config.themes_dir {
        lines.push(Line::raw(format!(" User themes go in {}", dir.display())));
    }
    ansi::write(&lines, out, mode != ColorMode::Mono)
}
