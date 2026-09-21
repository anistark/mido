use ratatui::style::{Color, Modifier, Style};

use crate::markdown::Alert;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Indexed,
    Mono,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ColorMode,
    pub fg: Color,
    pub fg_muted: Color,
    pub fg_faint: Color,
    pub accent: Color,
    pub headings: [Color; 6],
    pub h1_fg: Color,
    pub link: Color,
    pub link_url: Color,
    pub code_fg: Color,
    pub code_bg: Color,
    pub surface: Color,
    pub code_border: Color,
    pub quote_bar: Color,
    pub quote_fg: Color,
    pub rule: Color,
    pub table_border: Color,
    pub list_marker: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub search_bg: Color,
    pub search_fg: Color,
    pub search_current_bg: Color,
    pub overlay_border: Color,
    pub selected_bg: Color,
    pub alert_note: Color,
    pub alert_tip: Color,
    pub alert_important: Color,
    pub alert_warning: Color,
    pub alert_caution: Color,
    pub math: Color,
    pub label_bg: Color,
    pub label_value_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        let fg = Color::Rgb(205, 214, 244);
        let fg_muted = Color::Rgb(166, 173, 200);
        let fg_faint = Color::Rgb(108, 112, 134);
        let blue = Color::Rgb(137, 180, 250);
        let mint = Color::Rgb(92, 255, 176);
        let aqua = Color::Rgb(98, 232, 226);
        let mauve = Color::Rgb(203, 166, 247);
        let surface = Color::Rgb(69, 71, 90);
        Self {
            mode: ColorMode::TrueColor,
            fg,
            fg_muted,
            fg_faint,
            accent: mint,
            headings: [mint, aqua, mauve, fg, fg_muted, fg_muted],
            h1_fg: Color::Rgb(6, 15, 12),
            link: aqua,
            link_url: fg_faint,
            code_fg: Color::Rgb(250, 179, 135),
            code_bg: Color::Rgb(36, 38, 54),
            surface: Color::Rgb(36, 38, 54),
            code_border: surface,
            quote_bar: Color::Rgb(166, 227, 161),
            quote_fg: fg_muted,
            rule: surface,
            table_border: Color::Rgb(88, 91, 112),
            list_marker: mint,
            status_bg: Color::Rgb(49, 50, 68),
            status_fg: fg,
            search_bg: Color::Rgb(249, 226, 175),
            search_fg: Color::Rgb(30, 30, 46),
            search_current_bg: Color::Rgb(250, 179, 135),
            overlay_border: mint,
            selected_bg: Color::Rgb(49, 50, 68),
            alert_note: blue,
            alert_tip: Color::Rgb(166, 227, 161),
            alert_important: mauve,
            alert_warning: Color::Rgb(249, 226, 175),
            alert_caution: Color::Rgb(243, 139, 168),
            math: Color::Rgb(245, 194, 231),
            label_bg: Color::Rgb(49, 50, 68),
            label_value_bg: surface,
        }
    }

    pub fn for_terminal() -> Self {
        let mode = if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            ColorMode::Mono
        } else if truecolor_supported() {
            ColorMode::TrueColor
        } else {
            ColorMode::Indexed
        };
        Self::dark().with_mode(mode)
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

    pub fn adapt(&self, color: Color) -> Color {
        match self.mode {
            ColorMode::TrueColor => color,
            ColorMode::Indexed => to_indexed(color),
            ColorMode::Mono => Color::Reset,
        }
    }

    pub fn map_colors(mut self, f: impl Fn(Color) -> Color) -> Self {
        for c in [
            &mut self.fg,
            &mut self.fg_muted,
            &mut self.fg_faint,
            &mut self.accent,
            &mut self.h1_fg,
            &mut self.link,
            &mut self.link_url,
            &mut self.code_fg,
            &mut self.code_bg,
            &mut self.surface,
            &mut self.code_border,
            &mut self.quote_bar,
            &mut self.quote_fg,
            &mut self.rule,
            &mut self.table_border,
            &mut self.list_marker,
            &mut self.status_bg,
            &mut self.status_fg,
            &mut self.search_bg,
            &mut self.search_fg,
            &mut self.search_current_bg,
            &mut self.overlay_border,
            &mut self.selected_bg,
            &mut self.alert_note,
            &mut self.alert_tip,
            &mut self.alert_important,
            &mut self.alert_warning,
            &mut self.alert_caution,
            &mut self.math,
            &mut self.label_bg,
            &mut self.label_value_bg,
        ] {
            *c = f(*c);
        }
        for c in self.headings.iter_mut() {
            *c = f(*c);
        }
        self
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

    pub fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(self.h1_fg)
                .bg(self.headings[0])
                .add_modifier(Modifier::BOLD),
            _ => self.heading_text(level),
        }
    }

    pub fn heading_text(&self, level: u8) -> Style {
        let idx = level.clamp(1, 6) as usize - 1;
        let style = Style::new()
            .fg(self.headings[idx])
            .add_modifier(Modifier::BOLD);
        if level == 6 {
            style.add_modifier(Modifier::ITALIC)
        } else {
            style
        }
    }

    pub fn heading_rule(&self) -> Style {
        Style::new().fg(self.headings[1])
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
        let colors = [self.list_marker, self.headings[1], self.headings[2]];
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
        let fg = if luminance > 0.55 {
            self.h1_fg
        } else {
            self.fg
        };
        Style::new().fg(fg).bg(self.adapt(Color::Rgb(r, g, b)))
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
