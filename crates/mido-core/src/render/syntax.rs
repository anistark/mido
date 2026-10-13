use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::FontStyle;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use super::theme::{ColorMode, Theme};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

type Lines = Vec<Vec<Span<'static>>>;
type Key = (
    String,
    String,
    two_face::theme::EmbeddedThemeName,
    ColorMode,
    Color,
);

/// Highlighted code by language, code and the theme inputs that color it. A live preview
/// lays the whole document out again after every pause in typing, and syntect is the slow
/// part, so a block that did not change is never highlighted twice.
static CACHE: LazyLock<Mutex<HashMap<Key, Option<Arc<Lines>>>>> = LazyLock::new(Default::default);
const CACHE_ENTRIES: usize = 1024;

pub fn highlight(lang: &str, code: &str, theme: &Theme) -> Option<Lines> {
    let key = (
        alias(lang).to_string(),
        code.to_string(),
        theme.syntax,
        theme.mode,
        theme.code_bg,
    );
    if let Some(found) = CACHE.lock().ok()?.get(&key) {
        return found.as_deref().cloned();
    }
    let lines = highlight_uncached(&key.0, code, theme).map(Arc::new);
    let mut cache = CACHE.lock().ok()?;
    if cache.len() >= CACHE_ENTRIES {
        cache.clear();
    }
    cache.insert(key, lines.clone());
    lines.as_deref().cloned()
}

fn highlight_uncached(lang: &str, code: &str, theme: &Theme) -> Option<Lines> {
    let syntax = SYNTAXES.find_syntax_by_token(lang)?;
    let mut highlighter = HighlightLines::new(syntax, theme.syntax_theme());
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
        .fg(theme.rgb(fg.r, fg.g, fg.b))
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
