use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme as SyntectTheme};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

use super::theme::Theme;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static THEME: LazyLock<SyntectTheme> = LazyLock::new(|| {
    two_face::theme::extra()
        .get(EmbeddedThemeName::CatppuccinMocha)
        .clone()
});

pub fn highlight(lang: &str, code: &str, theme: &Theme) -> Option<Vec<Vec<Span<'static>>>> {
    let syntax = SYNTAXES.find_syntax_by_token(alias(lang))?;
    let mut highlighter = HighlightLines::new(syntax, &THEME);
    let mut lines = Vec::new();
    for line in LinesWithEndings::from(code) {
        let regions = highlighter.highlight_line(line, &SYNTAXES).ok()?;
        let spans = regions
            .into_iter()
            .map(|(style, text)| {
                Span::styled(
                    text.trim_end_matches('\n').to_string(),
                    convert(style, theme),
                )
            })
            .filter(|span| !span.content.is_empty())
            .collect();
        lines.push(spans);
    }
    Some(lines)
}

fn alias(lang: &str) -> &str {
    match lang {
        "shell" | "zsh" | "console" | "shell-session" => "bash",
        "jsonc" | "json5" => "json",
        "yml" => "yaml",
        "rs" => "rust",
        "ts" => "typescript",
        other => other,
    }
}

fn convert(style: syntect::highlighting::Style, theme: &Theme) -> Style {
    let fg = style.foreground;
    let mut out = Style::new()
        .fg(theme.adapt(Color::Rgb(fg.r, fg.g, fg.b)))
        .bg(theme.code_bg);
    if style.font_style.contains(FontStyle::BOLD) {
        out = out.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        out = out.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        out = out.add_modifier(Modifier::UNDERLINED);
    }
    out
}
