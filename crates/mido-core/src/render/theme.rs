use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use ratatui_core::style::{Color, Modifier, Style};
use serde::Deserialize;
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

use super::glyphs::Glyphs;
use crate::markdown::Alert;

pub const SCHEMA: u32 = 1;
pub const DEFAULT_DARK: &str = "mido-dark";
pub const DEFAULT_LIGHT: &str = "mido-light";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Indexed,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Dark,
    Light,
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            other => Err(format!("unknown kind `{other}`, expected dark or light")),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::Dark => "dark",
            Self::Light => "light",
        })
    }
}

macro_rules! tokens {
    ($($name:ident: $doc:literal,)*) => {
        #[derive(Debug, Clone)]
        pub struct Theme {
            pub name: String,
            pub kind: Kind,
            pub syntax: EmbeddedThemeName,
            pub mode: ColorMode,
            pub glyphs: Glyphs,
            $(#[doc = $doc] pub $name: Color,)*
        }

        /// Every color token with what it paints, in the order the docs list them.
        pub const TOKENS: &[(&str, &str)] = &[$((stringify!($name), $doc)),*];

        impl Theme {
            fn blank(name: &str, kind: Kind) -> Self {
                Self {
                    name: name.to_string(),
                    kind,
                    syntax: default_syntax(kind),
                    mode: ColorMode::TrueColor,
                    glyphs: Glyphs::default(),
                    $($name: Color::Reset,)*
                }
            }

            fn token_mut(&mut self, token: &str) -> Option<&mut Color> {
                match token {
                    $(stringify!($name) => Some(&mut self.$name),)*
                    _ => None,
                }
            }

            pub fn map_colors(mut self, f: impl Fn(Color) -> Color) -> Self {
                $(self.$name = f(self.$name);)*
                self
            }
        }
    };
}

tokens! {
    bg: "Background behind the whole viewer, `default` keeps the terminal's own",
    fg: "Body text",
    fg_muted: "Secondary text: H5 and H6, quotes, front matter keys, panel connectors",
    fg_faint: "Hints, placeholders, raw HTML, the overflow marker",
    accent: "Status chip, focus, footnote markers, the image marker",
    h1: "The H1 title chip",
    h2: "H2 text and its underline, second-level list markers",
    h3: "H3 text, third-level list markers",
    h4: "H4 text",
    h5: "H5 text",
    h6: "H6 text",
    h1_fg: "Text on the H1 chip and the status chip",
    link: "Link text",
    link_url: "The URL shown after a link",
    code_fg: "Inline code text",
    code_bg: "Inline code and code block background",
    code_border: "The bar left of a code block",
    surface: "Panel tint, table header band",
    quote_bar: "The bar left of a quote",
    quote_fg: "Quote text",
    rule: "Thematic breaks",
    table_border: "Table box drawing",
    list_marker: "First-level list markers",
    status_bg: "Status bar background",
    status_fg: "Status bar text",
    search_bg: "Search matches",
    search_fg: "Text on a search match",
    search_current_bg: "The current search match",
    overlay_border: "Border of overlays: help, contents, finder",
    selected_bg: "Selected row in a panel or overlay",
    alert_note: "NOTE alerts",
    alert_tip: "TIP alerts",
    alert_important: "IMPORTANT alerts",
    alert_warning: "WARNING alerts",
    alert_caution: "CAUTION alerts",
    math: "Inline and block math",
    label_bg: "Key side of a front matter or badge chip",
    label_value_bg: "Value side of a front matter chip",
}

pub const BUILTIN: &[(&str, &str)] = &[
    ("mido-dark", include_str!("themes/mido-dark.toml")),
    ("mido-light", include_str!("themes/mido-light.toml")),
    ("mido-reading", include_str!("themes/mido-reading.toml")),
    (
        "catppuccin-mocha",
        include_str!("themes/catppuccin-mocha.toml"),
    ),
    (
        "catppuccin-latte",
        include_str!("themes/catppuccin-latte.toml"),
    ),
    ("gruvbox", include_str!("themes/gruvbox.toml")),
    ("nord", include_str!("themes/nord.toml")),
    ("tokyo-night", include_str!("themes/tokyo-night.toml")),
    ("dracula", include_str!("themes/dracula.toml")),
    ("solarized-dark", include_str!("themes/solarized-dark.toml")),
    (
        "solarized-light",
        include_str!("themes/solarized-light.toml"),
    ),
];

static SYNTAX_THEMES: LazyLock<EmbeddedLazyThemeSet> = LazyLock::new(two_face::theme::extra);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeError(pub String);

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ThemeError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    schema: u32,
    kind: Option<Kind>,
    extends: Option<String>,
    syntax: Option<String>,
    #[serde(default)]
    palette: BTreeMap<String, String>,
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

impl Theme {
    pub fn dark() -> Self {
        Self::builtin(DEFAULT_DARK).expect("mido-dark is built in")
    }

    pub fn light() -> Self {
        Self::builtin(DEFAULT_LIGHT).expect("mido-light is built in")
    }

    pub fn builtin(name: &str) -> Option<Self> {
        let (_, text) = BUILTIN.iter().find(|(n, _)| *n == name)?;
        Some(Self::parse(name, text, &|base| Self::builtin(base)).expect("built-in themes parse"))
    }

    /// Reads a theme file. `base` resolves the theme named by `extends`.
    pub fn parse(
        name: &str,
        text: &str,
        base: &dyn Fn(&str) -> Option<Theme>,
    ) -> Result<Self, ThemeError> {
        let file: ThemeFile =
            toml::from_str(text).map_err(|e| ThemeError(format!("{name}: {}", e.message())))?;
        if file.schema != SCHEMA {
            return Err(ThemeError(format!(
                "{name}: schema {} is not supported, this mido reads schema {SCHEMA}",
                file.schema
            )));
        }
        let mut theme = match &file.extends {
            Some(parent) => {
                let mut theme = base(parent).ok_or_else(|| {
                    ThemeError(format!("{name}: extends `{parent}`, which is not a theme"))
                })?;
                theme.name = name.to_string();
                if let Some(kind) = file.kind {
                    theme.kind = kind;
                }
                theme
            }
            None => {
                let kind = file
                    .kind
                    .ok_or_else(|| ThemeError(format!("{name}: `kind` must be dark or light")))?;
                Self::blank(name, kind)
            }
        };
        if file.extends.is_none() || file.kind.is_some() {
            theme.syntax = default_syntax(theme.kind);
        }
        if let Some(syntax) = &file.syntax {
            theme.syntax = syntax_theme(syntax)
                .ok_or_else(|| ThemeError(format!("{name}: unknown syntax theme `{syntax}`")))?;
        }
        let mut set = BTreeSet::new();
        for (token, value) in &file.colors {
            let lookup = file.palette.get(value).unwrap_or(value);
            let color = parse_color(lookup).ok_or_else(|| {
                ThemeError(format!(
                    "{name}: `{token} = \"{value}\"` is not a color or a palette name"
                ))
            })?;
            let slot = theme
                .token_mut(token)
                .ok_or_else(|| ThemeError(format!("{name}: unknown token `{token}`")))?;
            *slot = color;
            set.insert(token.as_str());
        }
        if file.extends.is_none() {
            let missing: Vec<&str> = TOKENS
                .iter()
                .map(|(t, _)| *t)
                .filter(|t| !set.contains(t))
                .collect();
            if !missing.is_empty() {
                return Err(ThemeError(format!(
                    "{name}: missing {}. Set them or add `extends = \"{}\"`",
                    missing.join(", "),
                    match theme.kind {
                        Kind::Dark => DEFAULT_DARK,
                        Kind::Light => DEFAULT_LIGHT,
                    }
                )));
            }
        }
        Ok(theme)
    }

    pub fn detect_mode() -> ColorMode {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            ColorMode::Mono
        } else if truecolor_supported() {
            ColorMode::TrueColor
        } else {
            ColorMode::Indexed
        }
    }

    pub fn with_mode(self, mode: ColorMode) -> Self {
        let mut theme = match mode {
            ColorMode::TrueColor => self,
            ColorMode::Indexed => self.map_colors(to_indexed),
            ColorMode::Mono => self.map_colors(|_| Color::Reset),
        };
        theme.mode = mode;
        theme
    }

    pub fn with_glyphs(mut self, glyphs: Glyphs) -> Self {
        self.glyphs = glyphs;
        self
    }

    pub fn adapt(&self, color: Color) -> Color {
        match self.mode {
            ColorMode::TrueColor => color,
            ColorMode::Indexed => to_indexed(color),
            ColorMode::Mono => Color::Reset,
        }
    }

    pub fn rgb(&self, r: u8, g: u8, b: u8) -> Color {
        self.adapt(Color::Rgb(r, g, b))
    }

    pub fn syntax_theme(&self) -> &'static syntect::highlighting::Theme {
        SYNTAX_THEMES.get(self.syntax)
    }

    pub fn paints_background(&self) -> bool {
        self.bg != Color::Reset
    }

    pub fn background(&self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }

    pub fn text(&self) -> Style {
        Style::new().fg(self.fg)
    }

    pub fn muted(&self) -> Style {
        Style::new().fg(self.fg_muted)
    }

    pub fn faint(&self) -> Style {
        Style::new().fg(self.fg_faint)
    }

    pub fn accent(&self) -> Style {
        Style::new().fg(self.accent)
    }

    pub fn heading_color(&self, level: u8) -> Color {
        match level {
            ..=1 => self.h1,
            2 => self.h2,
            3 => self.h3,
            4 => self.h4,
            5 => self.h5,
            _ => self.h6,
        }
    }

    pub fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(self.h1_fg)
                .bg(self.h1)
                .add_modifier(Modifier::BOLD),
            _ => self.heading_text(level),
        }
    }

    pub fn heading_text(&self, level: u8) -> Style {
        let style = Style::new()
            .fg(self.heading_color(level))
            .add_modifier(Modifier::BOLD);
        if level >= 6 {
            style.add_modifier(Modifier::ITALIC)
        } else {
            style
        }
    }

    pub fn heading_rule(&self) -> Style {
        Style::new().fg(self.h2)
    }

    pub fn code_inline(&self) -> Style {
        Style::new().fg(self.code_fg).bg(self.code_bg)
    }

    pub fn code_block(&self) -> Style {
        Style::new().fg(self.fg).bg(self.code_bg)
    }

    pub fn code_border(&self) -> Style {
        Style::new().fg(self.code_border)
    }

    pub fn link(&self) -> Style {
        Style::new()
            .fg(self.link)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn link_url(&self) -> Style {
        Style::new().fg(self.link_url)
    }

    pub fn quote(&self) -> Style {
        Style::new()
            .fg(self.quote_fg)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn quote_bar(&self) -> Style {
        Style::new().fg(self.quote_bar)
    }

    pub fn rule(&self) -> Style {
        Style::new().fg(self.rule)
    }

    pub fn table_border(&self) -> Style {
        Style::new().fg(self.table_border)
    }

    pub fn table_header(&self) -> Style {
        Style::new()
            .fg(self.fg)
            .bg(self.surface)
            .add_modifier(Modifier::BOLD)
    }

    pub fn table_band(&self) -> Style {
        Style::new().bg(self.surface)
    }

    pub fn list_marker(&self) -> Style {
        Style::new().fg(self.list_marker)
    }

    pub fn list_marker_at(&self, depth: usize) -> Style {
        let colors = [self.list_marker, self.h2, self.h3];
        Style::new().fg(colors[depth % colors.len()])
    }

    pub fn label(&self) -> Style {
        Style::new().fg(self.fg_muted).bg(self.label_bg)
    }

    pub fn label_value(&self) -> Style {
        Style::new().fg(self.fg).bg(self.label_value_bg)
    }

    pub fn label_colored(&self, (r, g, b): (u8, u8, u8)) -> Style {
        let luminance =
            (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
        let (dark_text, light_text) = match self.kind {
            Kind::Dark => (self.h1_fg, self.fg),
            Kind::Light => (self.fg, self.h1_fg),
        };
        let fg = if luminance > 0.55 {
            dark_text
        } else {
            light_text
        };
        Style::new().fg(fg).bg(self.rgb(r, g, b))
    }

    pub fn footnote(&self) -> Style {
        Style::new().fg(self.accent)
    }

    pub fn alert(&self, alert: Alert) -> Color {
        match alert {
            Alert::Note => self.alert_note,
            Alert::Tip => self.alert_tip,
            Alert::Important => self.alert_important,
            Alert::Warning => self.alert_warning,
            Alert::Caution => self.alert_caution,
        }
    }

    pub fn alert_bar(&self, alert: Alert) -> Style {
        Style::new().fg(self.alert(alert))
    }

    pub fn alert_title(&self, alert: Alert) -> Style {
        Style::new()
            .fg(self.alert(alert))
            .add_modifier(Modifier::BOLD)
    }

    pub fn math(&self) -> Style {
        Style::new().fg(self.math).add_modifier(Modifier::ITALIC)
    }

    pub fn status_chip(&self) -> Style {
        Style::new()
            .fg(self.h1_fg)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status(&self) -> Style {
        Style::new().fg(self.status_fg).bg(self.status_bg)
    }

    pub fn search(&self, current: bool) -> Style {
        let bg = if current {
            self.search_current_bg
        } else {
            self.search_bg
        };
        Style::new().fg(self.search_fg).bg(bg)
    }

    pub fn overlay_border(&self) -> Style {
        Style::new().fg(self.overlay_border)
    }

    pub fn selected(&self) -> Style {
        Style::new()
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "default" | "none" | "reset" => return Some(Color::Reset),
        _ => {}
    }
    if let Some(hex) = value.strip_prefix('#')
        && hex.len() == 3
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|v| v * 17);
        return Some(Color::Rgb(digit(0)?, digit(1)?, digit(2)?));
    }
    Color::from_str(value).ok()
}

pub fn syntax_theme(name: &str) -> Option<EmbeddedThemeName> {
    let wanted = normalize(name);
    EmbeddedLazyThemeSet::theme_names()
        .iter()
        .copied()
        .find(|t| normalize(t.as_name()) == wanted)
}

pub fn syntax_theme_names() -> Vec<String> {
    EmbeddedLazyThemeSet::theme_names()
        .iter()
        .map(|t| normalize(t.as_name()))
        .collect()
}

fn normalize(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The generated half of `docs/themes.md`: built-in themes, tokens and syntax theme names.
pub fn markdown() -> String {
    let mut out = String::from(
        "## Built-in themes\n\n| Theme | Kind | Code | About |\n| --- | --- | --- | --- |\n",
    );
    for (name, text) in BUILTIN {
        let theme = Theme::builtin(name).expect("built-in themes parse");
        let about = text
            .lines()
            .take_while(|l| l.starts_with('#'))
            .map(|l| l.trim_start_matches('#').trim())
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "| `{name}` | {} | `{}` | {about} |\n",
            theme.kind,
            normalize(theme.syntax.as_name())
        ));
    }
    out.push_str(concat!(
        "\n## Tokens\n\n",
        "Every token a theme sets, in the order `mido-dark.toml` lists them. A theme without ",
        "`extends` must set all of them.\n\n",
        "| Token | Paints |\n| --- | --- |\n",
    ));
    for (token, what) in TOKENS {
        out.push_str(&format!("| `{token}` | {what} |\n"));
    }
    out.push_str("\n## Syntax themes\n\n`syntax` takes one of these names, the set bat ships:\n\n");
    let names: Vec<String> = syntax_theme_names()
        .iter()
        .map(|n| format!("`{n}`"))
        .collect();
    out.push_str(&names.join(", "));
    out.push('\n');
    out
}

fn default_syntax(kind: Kind) -> EmbeddedThemeName {
    match kind {
        Kind::Dark => EmbeddedThemeName::CatppuccinMocha,
        Kind::Light => EmbeddedThemeName::CatppuccinLatte,
    }
}

pub fn truecolor_supported() -> bool {
    std::env::var("COLORTERM").is_ok_and(|v| matches!(v.as_str(), "truecolor" | "24bit"))
}

pub fn to_indexed(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    if r == g && g == b {
        return Color::Indexed(match r {
            0..=4 => 16,
            243.. => 231,
            v => 232 + ((v as u16 - 8) * 24 / 240).min(23) as u8,
        });
    }
    let q = |v: u8| -> u8 {
        match v {
            0..=47 => 0,
            48..=114 => 1,
            v => ((v as u16 - 35) / 40) as u8,
        }
    };
    Color::Indexed(16 + 36 * q(r) + 6 * q(g) + q(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_theme_defines_every_token() {
        for (name, text) in BUILTIN {
            let file: ThemeFile = toml::from_str(text).unwrap();
            assert!(file.extends.is_none(), "{name} stands alone");
            let theme = Theme::parse(name, text, &|_| None).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(theme.name, *name);
            for (token, _) in TOKENS {
                assert!(file.colors.contains_key(*token), "{name} sets {token}");
            }
        }
    }

    #[test]
    fn builtin_kinds_match_their_backgrounds() {
        for (name, _) in BUILTIN {
            let theme = Theme::builtin(name).unwrap();
            if let Color::Rgb(r, g, b) = theme.fg {
                let light_text = u32::from(r) + u32::from(g) + u32::from(b) > 3 * 128;
                assert_eq!(
                    light_text,
                    theme.kind == Kind::Dark,
                    "{name} fg suits its kind"
                );
            }
        }
    }

    #[test]
    fn a_theme_can_extend_another() {
        let text = "schema = 1\nextends = \"mido-dark\"\n[palette]\nrose = \"#f5c2e7\"\n[colors]\naccent = \"rose\"\nlink = \"#abc\"\n";
        let theme = Theme::parse("mine", text, &|n| Theme::builtin(n)).unwrap();
        assert_eq!(theme.name, "mine");
        assert_eq!(theme.accent, Color::Rgb(245, 194, 231));
        assert_eq!(theme.link, Color::Rgb(170, 187, 204));
        assert_eq!(theme.fg, Theme::dark().fg);
        assert_eq!(theme.kind, Kind::Dark);
    }

    #[test]
    fn theme_errors_name_the_problem() {
        let base = |n: &str| Theme::builtin(n);
        let err = |text: &str| Theme::parse("t", text, &base).unwrap_err().0;
        assert!(err("schema = 2\nkind = \"dark\"").contains("schema 2"));
        assert!(err("schema = 1\nkind = \"dark\"").contains("missing bg, fg"));
        assert!(
            err("schema = 1\nextends = \"mido-dark\"\n[colors]\nfgg = \"red\"")
                .contains("unknown token `fgg`")
        );
        assert!(
            err("schema = 1\nextends = \"mido-dark\"\n[colors]\nfg = \"rose\"")
                .contains("not a color")
        );
        assert!(err("schema = 1\nextends = \"nope\"").contains("extends `nope`"));
        assert!(
            err("schema = 1\nextends = \"mido-dark\"\nsyntax = \"nope\"").contains("syntax theme")
        );
    }

    #[test]
    fn colors_parse_in_every_form() {
        assert_eq!(parse_color("#ff8000"), Some(Color::Rgb(255, 128, 0)));
        assert_eq!(parse_color("#f80"), Some(Color::Rgb(255, 136, 0)));
        assert_eq!(parse_color("default"), Some(Color::Reset));
        assert_eq!(parse_color("bright-blue"), Some(Color::LightBlue));
        assert_eq!(parse_color("208"), Some(Color::Indexed(208)));
        assert_eq!(parse_color("#ggg"), None);
    }

    #[test]
    fn syntax_names_match_loosely() {
        assert_eq!(
            syntax_theme("catppuccin-mocha"),
            Some(EmbeddedThemeName::CatppuccinMocha)
        );
        assert_eq!(
            syntax_theme("Solarized (dark)"),
            Some(EmbeddedThemeName::SolarizedDark)
        );
        assert_eq!(
            syntax_theme("solarized-light"),
            Some(EmbeddedThemeName::SolarizedLight)
        );
        assert!(syntax_theme_names().contains(&"gruvbox-dark".to_string()));
    }
}
