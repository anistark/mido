use std::fmt;
use std::str::FromStr;

use ratatui_core::symbols::border;
use serde::Deserialize;

use crate::markdown::Alert;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlyphTier {
    Ascii,
    #[default]
    Unicode,
    Nerd,
}

impl FromStr for GlyphTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ascii" => Ok(Self::Ascii),
            "unicode" => Ok(Self::Unicode),
            "nerd" => Ok(Self::Nerd),
            other => Err(format!(
                "unknown glyph tier `{other}`, expected ascii, unicode or nerd"
            )),
        }
    }
}

impl fmt::Display for GlyphTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ascii => "ascii",
            Self::Unicode => "unicode",
            Self::Nerd => "nerd",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableGlyphs {
    pub top: [&'static str; 4],
    pub header: [&'static str; 4],
    pub bottom: [&'static str; 4],
    pub vertical: &'static str,
}

/// Every symbol mido draws itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyphs {
    pub tier: GlyphTier,
    pub bullets: [&'static str; 3],
    pub task_done: &'static str,
    pub task_open: &'static str,
    pub bar: &'static str,
    pub rule: &'static str,
    pub heavy_rule: &'static str,
    pub table: TableGlyphs,
    pub image: &'static str,
    pub expanded: &'static str,
    pub collapsed: &'static str,
    pub branch: &'static str,
    pub last: &'static str,
    pub pipe: &'static str,
    pub ellipsis: &'static str,
    pub alerts: [&'static str; 5],
    pub superscript: bool,
    pub focus_bar: &'static str,
    pub cursor: &'static str,
    pub dot: &'static str,
    pub edge: &'static str,
    pub scroll_track: &'static str,
    pub scroll_thumb: &'static str,
    pub border: border::Set<'static>,
    pub folder_open: &'static str,
    pub folder_closed: &'static str,
    pub file: &'static str,
}

const ASCII_BORDER: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

impl Glyphs {
    pub const UNICODE: Self = Self {
        tier: GlyphTier::Unicode,
        bullets: ["•", "◦", "▪"],
        task_done: "☑ ",
        task_open: "☐ ",
        bar: "▎ ",
        rule: "─",
        heavy_rule: "━",
        table: TableGlyphs {
            top: ["┌", "─", "┬", "┐"],
            header: ["┝", "━", "┿", "┥"],
            bottom: ["└", "─", "┴", "┘"],
            vertical: "│",
        },
        image: "▣ ",
        expanded: "▾ ",
        collapsed: "▸ ",
        branch: "├ ",
        last: "└ ",
        pipe: "│ ",
        ellipsis: "…",
        alerts: ["●", "✦", "◆", "▲", "■"],
        superscript: true,
        focus_bar: "▎",
        cursor: "▏",
        dot: "·",
        edge: "│",
        scroll_track: "│",
        scroll_thumb: "█",
        border: border::ROUNDED,
        folder_open: "",
        folder_closed: "",
        file: "",
    };

    pub const ASCII: Self = Self {
        tier: GlyphTier::Ascii,
        bullets: ["-", "*", "+"],
        task_done: "[x] ",
        task_open: "[ ] ",
        bar: "| ",
        rule: "-",
        heavy_rule: "=",
        table: TableGlyphs {
            top: ["+", "-", "+", "+"],
            header: ["+", "=", "+", "+"],
            bottom: ["+", "-", "+", "+"],
            vertical: "|",
        },
        image: "[img] ",
        expanded: "v ",
        collapsed: "> ",
        branch: "|-",
        last: "`-",
        pipe: "| ",
        ellipsis: "~",
        alerts: ["i", "+", "!", "!", "x"],
        superscript: false,
        focus_bar: "|",
        cursor: "_",
        dot: "|",
        edge: "|",
        scroll_track: "|",
        scroll_thumb: "#",
        border: ASCII_BORDER,
        folder_open: "",
        folder_closed: "",
        file: "",
    };

    pub const NERD: Self = Self {
        tier: GlyphTier::Nerd,
        image: "\u{f03e} ",
        alerts: ["\u{f05a}", "\u{f0eb}", "\u{f06a}", "\u{f071}", "\u{f057}"],
        folder_open: "\u{f07c} ",
        folder_closed: "\u{f07b} ",
        file: "\u{e73e} ",
        ..Self::UNICODE
    };

    pub fn for_tier(tier: GlyphTier) -> Self {
        match tier {
            GlyphTier::Ascii => Self::ASCII,
            GlyphTier::Unicode => Self::UNICODE,
            GlyphTier::Nerd => Self::NERD,
        }
    }

    pub fn alert(&self, alert: Alert) -> &'static str {
        let index = match alert {
            Alert::Note => 0,
            Alert::Tip => 1,
            Alert::Important => 2,
            Alert::Warning => 3,
            Alert::Caution => 4,
        };
        self.alerts[index]
    }

    pub fn bullet(&self, depth: usize) -> &'static str {
        self.bullets[depth % self.bullets.len()]
    }
}

impl Default for Glyphs {
    fn default() -> Self {
        Self::UNICODE
    }
}
