use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ScrollDown,
    ScrollUp,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    Top,
    Bottom,
    Toc,
    Images,
    FrontMatter,
    Reload,
    Files,
    Outline,
    FocusMode,
    FocusRight,
    NextLink,
    PrevLink,
    Back,
    Forward,
    Finder,
    Search,
    NextMatch,
    PrevMatch,
    Edit,
    Help,
    Quit,
}

/// Name in the config, default keys, and what the action does.
pub const ACTIONS: &[(Action, &str, &[&str], &str)] = &[
    (
        Action::ScrollDown,
        "scroll_down",
        &["j", "down"],
        "Scroll down one line, move down in a panel or list",
    ),
    (
        Action::ScrollUp,
        "scroll_up",
        &["k", "up"],
        "Scroll up one line, move up in a panel or list",
    ),
    (
        Action::HalfPageDown,
        "half_page_down",
        &["ctrl-d"],
        "Half page down",
    ),
    (
        Action::HalfPageUp,
        "half_page_up",
        &["ctrl-u"],
        "Half page up",
    ),
    (
        Action::PageDown,
        "page_down",
        &["ctrl-f", "space"],
        "Page down",
    ),
    (Action::PageUp, "page_up", &["ctrl-b"], "Page up"),
    (Action::Top, "top", &["g"], "Go to the top"),
    (Action::Bottom, "bottom", &["G"], "Go to the bottom"),
    (Action::Toc, "toc", &["t"], "Table of contents"),
    (Action::Images, "images", &["i"], "Show or hide images"),
    (
        Action::FrontMatter,
        "front_matter",
        &["m"],
        "Expand or collapse the front matter",
    ),
    (Action::Reload, "reload", &["r"], "Reload the file"),
    (Action::Files, "files", &["b"], "Toggle the files panel"),
    (
        Action::Outline,
        "outline",
        &["o"],
        "Toggle the outline panel",
    ),
    (Action::FocusMode, "focus_mode", &["f"], "Focus mode"),
    (
        Action::FocusRight,
        "focus_right",
        &["l"],
        "Focus the pane to the right",
    ),
    (
        Action::NextLink,
        "next_link",
        &["]"],
        "Select the next link",
    ),
    (
        Action::PrevLink,
        "prev_link",
        &["["],
        "Select the previous link",
    ),
    (Action::Back, "back", &["H"], "Back through visited pages"),
    (
        Action::Forward,
        "forward",
        &["L"],
        "Forward through visited pages",
    ),
    (
        Action::Finder,
        "finder",
        &["ctrl-p"],
        "Find a file in the folder",
    ),
    (Action::Search, "search", &["/"], "Search"),
    (Action::NextMatch, "next_match", &["n"], "Next match"),
    (Action::PrevMatch, "prev_match", &["N"], "Previous match"),
    (Action::Edit, "edit", &["E"], "Edit the file in $EDITOR"),
    (Action::Help, "help", &["?", "h"], "Key help"),
    (Action::Quit, "quit", &["q"], "Quit"),
];

enum Row {
    One(Action, &'static str),
    Pair(Action, Action, &'static str, &'static str),
    Fixed(&'static str, &'static str),
}

const SECTIONS: &[(&str, &[Row])] = &[
    (
        "Reading",
        &[
            Row::Pair(
                Action::ScrollDown,
                Action::ScrollUp,
                ", wheel",
                "Scroll one line",
            ),
            Row::Pair(
                Action::HalfPageDown,
                Action::HalfPageUp,
                "",
                "Half page down / up",
            ),
            Row::Pair(Action::PageDown, Action::PageUp, "", "Page down / up"),
            Row::Pair(Action::Top, Action::Bottom, "", "Top / bottom"),
            Row::One(Action::Toc, "Table of contents"),
            Row::One(Action::Images, "Show or hide images"),
            Row::One(Action::FrontMatter, "Expand or collapse the front matter"),
            Row::One(Action::Reload, "Reload the file"),
        ],
    ),
    (
        "Panels and focus",
        &[
            Row::Pair(
                Action::Files,
                Action::Outline,
                "",
                "Toggle the files / outline panel",
            ),
            Row::One(Action::FocusMode, "Focus mode: hide the panels, full width"),
            Row::Fixed("`Tab` / `Shift-Tab`", "Cycle focus between the panes"),
            Row::Fixed("`Tab` then `←` / `→`", "Move focus in that direction"),
            Row::One(Action::FocusRight, "Focus the pane to the right"),
            Row::Pair(
                Action::ScrollDown,
                Action::ScrollUp,
                " in a panel",
                "Move, Enter opens or returns",
            ),
            Row::Fixed(
                "`Space`, `←` / `→` in a panel",
                "Fold, collapse / expand a folder or section",
            ),
            Row::Fixed("`-` / `=` in a panel", "Fold all / unfold all"),
            Row::Fixed(
                "`T` in the files panel",
                "Show titles instead of file names",
            ),
        ],
    ),
    (
        "Links and history",
        &[
            Row::Pair(
                Action::NextLink,
                Action::PrevLink,
                "",
                "Select the next / previous link",
            ),
            Row::Fixed(
                "`Enter`, click",
                "Follow the selected link or open a footnote",
            ),
            Row::Pair(
                Action::Back,
                Action::Forward,
                "",
                "Back / forward through visited pages",
            ),
            Row::One(Action::Finder, "Find a file in the folder"),
        ],
    ),
    (
        "Search and editing",
        &[
            Row::One(Action::Search, "Search, Enter to keep, Esc to cancel"),
            Row::Pair(
                Action::NextMatch,
                Action::PrevMatch,
                "",
                "Next / previous match",
            ),
            Row::One(Action::Edit, "Edit the file in $EDITOR, reload on return"),
        ],
    ),
    (
        "Help and quit",
        &[
            Row::One(Action::Help, "This help"),
            Row::One(Action::Quit, "Quit"),
        ],
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyChord {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rest = text;
        let mut ctrl = false;
        let mut alt = false;
        loop {
            let lower = rest.to_ascii_lowercase();
            if rest.len() > 5 && (lower.starts_with("ctrl-") || lower.starts_with("ctrl+")) {
                ctrl = true;
                rest = &rest[5..];
            } else if rest.len() > 4 && (lower.starts_with("alt-") || lower.starts_with("alt+")) {
                alt = true;
                rest = &rest[4..];
            } else {
                break;
            }
        }
        let mut chars = rest.chars();
        let code = match (chars.next(), chars.next()) {
            (Some(c), None) if ctrl => KeyCode::Char(c.to_ascii_lowercase()),
            (Some(c), None) => KeyCode::Char(c),
            _ => match rest.to_ascii_lowercase().as_str() {
                "space" => KeyCode::Char(' '),
                "enter" | "return" => KeyCode::Enter,
                "esc" | "escape" => KeyCode::Esc,
                "tab" => KeyCode::Tab,
                "backspace" => KeyCode::Backspace,
                "delete" | "del" => KeyCode::Delete,
                "insert" | "ins" => KeyCode::Insert,
                "up" => KeyCode::Up,
                "down" => KeyCode::Down,
                "left" => KeyCode::Left,
                "right" => KeyCode::Right,
                "pageup" | "pgup" => KeyCode::PageUp,
                "pagedown" | "pgdn" => KeyCode::PageDown,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                name => match name.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                    Some(n @ 1..=12) => KeyCode::F(n),
                    _ => return Err(format!("`{text}` is not a key")),
                },
            },
        };
        Ok(Self { code, ctrl, alt })
    }

    pub fn from_event(key: &KeyEvent) -> Self {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let code = match key.code {
            KeyCode::Char(c) if ctrl => KeyCode::Char(c.to_ascii_lowercase()),
            code => code,
        };
        Self {
            code,
            ctrl,
            alt: key.modifiers.contains(KeyModifiers::ALT),
        }
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl-")?;
        }
        if self.alt {
            f.write_str("Alt-")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("Space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Enter => f.write_str("Enter"),
            KeyCode::Esc => f.write_str("Esc"),
            KeyCode::Tab => f.write_str("Tab"),
            KeyCode::Backspace => f.write_str("Backspace"),
            KeyCode::Delete => f.write_str("Del"),
            KeyCode::Insert => f.write_str("Ins"),
            KeyCode::Up => f.write_str("↑"),
            KeyCode::Down => f.write_str("↓"),
            KeyCode::Left => f.write_str("←"),
            KeyCode::Right => f.write_str("→"),
            KeyCode::PageUp => f.write_str("PgUp"),
            KeyCode::PageDown => f.write_str("PgDn"),
            KeyCode::Home => f.write_str("Home"),
            KeyCode::End => f.write_str("End"),
            KeyCode::F(n) => write!(f, "F{n}"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<Action, Vec<KeyChord>>,
    lookup: HashMap<KeyChord, Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        let bindings = ACTIONS
            .iter()
            .map(|(action, _, keys, _)| {
                let chords = keys
                    .iter()
                    .map(|k| KeyChord::parse(k).expect("default keys parse"))
                    .collect();
                (*action, chords)
            })
            .collect();
        Self::from_bindings(bindings)
    }
}

impl Keymap {
    /// Applies `[keys]` from the config. A listed action loses its default keys, and a key
    /// taken by it is removed from whichever action had it before.
    pub fn with_overrides(overrides: &BTreeMap<String, Vec<String>>) -> Result<Self, String> {
        let mut bindings = Self::default().bindings;
        for (name, keys) in overrides {
            let action = ACTIONS
                .iter()
                .find(|(_, n, _, _)| n == name)
                .map(|(a, _, _, _)| *a)
                .ok_or_else(|| format!("unknown action `{name}` in [keys]"))?;
            let chords = keys
                .iter()
                .map(|k| KeyChord::parse(k))
                .collect::<Result<Vec<_>, _>>()?;
            for other in bindings.values_mut() {
                other.retain(|c| !chords.contains(c));
            }
            bindings.insert(action, chords);
        }
        Ok(Self::from_bindings(bindings))
    }

    fn from_bindings(bindings: HashMap<Action, Vec<KeyChord>>) -> Self {
        let lookup = bindings
            .iter()
            .flat_map(|(action, chords)| chords.iter().map(|c| (*c, *action)))
            .collect();
        Self { bindings, lookup }
    }

    pub fn action(&self, key: &KeyEvent) -> Option<Action> {
        self.lookup.get(&KeyChord::from_event(key)).copied()
    }

    pub fn keys(&self, action: Action) -> &[KeyChord] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    pub fn first(&self, action: Action) -> String {
        self.keys(action)
            .first()
            .map_or_else(|| "-".to_string(), ToString::to_string)
    }

    fn row_keys(&self, row: &Row) -> Option<(String, &'static str)> {
        let tick = |c: &KeyChord| format!("`{c}`");
        match row {
            Row::Fixed(keys, label) => Some((keys.to_string(), label)),
            Row::One(action, label) => {
                let keys: Vec<String> = self.keys(*action).iter().map(tick).collect();
                (!keys.is_empty()).then(|| (keys.join(", "), *label))
            }
            Row::Pair(a, b, suffix, label) => {
                let (a, b) = (self.keys(*a), self.keys(*b));
                let parts: Vec<String> = (0..a.len().max(b.len()))
                    .map(|i| match (a.get(i), b.get(i)) {
                        (Some(x), Some(y)) => format!("{} / {}", tick(x), tick(y)),
                        (Some(x), None) | (None, Some(x)) => tick(x),
                        (None, None) => unreachable!(),
                    })
                    .collect();
                (!parts.is_empty()).then(|| (format!("{}{suffix}", parts.join(", ")), *label))
            }
        }
    }

    /// Sections of (keys in backticks, action) rows for the help overlay and the docs.
    pub fn sections(&self) -> Vec<(&'static str, Vec<(String, &'static str)>)> {
        SECTIONS
            .iter()
            .map(|(title, rows)| {
                (
                    *title,
                    rows.iter().filter_map(|r| self.row_keys(r)).collect(),
                )
            })
            .collect()
    }
}

pub fn plain(keys: &str) -> String {
    keys.replace('`', "")
}

pub fn markdown() -> String {
    let mut out = String::new();
    for (title, rows) in Keymap::default().sections() {
        out.push_str(&format!("## {title}\n\n| Key | Action |\n| --- | --- |\n"));
        for (keys, action) in rows {
            out.push_str(&format!("| {keys} | {action} |\n"));
        }
        out.push('\n');
    }
    out.push_str(concat!(
        "## Remapping\n\n",
        "Every key above that is not fixed can be bound in the `[keys]` table of the ",
        "[config file](configuration.md). An action listed there loses its default keys, and a ",
        "key it takes is removed from the action that had it. An empty list unbinds the action.\n\n",
        "```toml\n[keys]\nscroll_down = [\"j\", \"down\", \"ctrl-n\"]\nquit = [\"q\", \"ctrl-q\"]\nimages = []\n```\n\n",
        "Keys are written as a single character, which keeps its case, a name (`space`, `enter`, ",
        "`esc`, `tab`, `backspace`, `up`, `down`, `left`, `right`, `pageup`, `pagedown`, `home`, ",
        "`end`, `f1` to `f12`), or either with a `ctrl-` or `alt-` prefix. `Esc`, `Tab`, `Ctrl-c` ",
        "and the panel keys are fixed. `Enter`, `PgDn`, `PgUp`, `Home` and `End` scroll, page and ",
        "jump unless a binding takes them.\n\n",
        "| Action | Default | What it does |\n| --- | --- | --- |\n",
    ));
    let keymap = Keymap::default();
    for (action, name, _, what) in ACTIONS {
        let keys: Vec<String> = keymap
            .keys(*action)
            .iter()
            .map(|c| format!("`{c}`"))
            .collect();
        out.push_str(&format!("| `{name}` | {} | {what} |\n", keys.join(", ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn keys_parse_and_print() {
        for (text, shown) in [
            ("j", "j"),
            ("G", "G"),
            ("ctrl-d", "Ctrl-d"),
            ("Ctrl+P", "Ctrl-p"),
            ("alt-x", "Alt-x"),
            ("space", "Space"),
            ("pgdn", "PgDn"),
            ("f5", "F5"),
            ("-", "-"),
        ] {
            assert_eq!(KeyChord::parse(text).unwrap().to_string(), shown);
        }
        assert!(KeyChord::parse("ctrl-").is_err());
        assert_eq!(KeyChord::parse("ctrl--").unwrap().to_string(), "Ctrl--");
        assert!(KeyChord::parse("banana").is_err());
        assert!(KeyChord::parse("f13").is_err());
    }

    #[test]
    fn defaults_cover_every_action_once() {
        let keymap = Keymap::default();
        let bound: usize = ACTIONS.iter().map(|(a, ..)| keymap.keys(*a).len()).sum();
        assert_eq!(bound, keymap.lookup.len(), "no key is bound twice");
        assert_eq!(
            keymap.action(&event(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Some(Action::Bottom)
        );
        assert_eq!(
            keymap.action(&event(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            Some(Action::PageUp)
        );
        assert_eq!(
            keymap.action(&event(KeyCode::Char('b'), KeyModifiers::NONE)),
            Some(Action::Files)
        );
    }

    #[test]
    fn overrides_take_keys_from_other_actions() {
        let overrides = BTreeMap::from([
            ("quit".to_string(), vec!["j".to_string()]),
            ("images".to_string(), vec![]),
        ]);
        let keymap = Keymap::with_overrides(&overrides).unwrap();
        let j = event(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(keymap.action(&j), Some(Action::Quit));
        assert_eq!(keymap.keys(Action::ScrollDown).len(), 1);
        assert_eq!(
            keymap.action(&event(KeyCode::Char('q'), KeyModifiers::NONE)),
            None
        );
        assert!(keymap.keys(Action::Images).is_empty());
        let rows = keymap.sections();
        assert!(!rows[0].1.iter().any(|(_, a)| *a == "Show or hide images"));
        assert_eq!(rows[0].1[0].0, "`↓` / `k`, `↑`, wheel");
    }

    #[test]
    fn override_errors_name_the_problem() {
        let bad = |name: &str, key: &str| {
            Keymap::with_overrides(&BTreeMap::from([(name.to_string(), vec![key.to_string()])]))
                .unwrap_err()
        };
        assert!(bad("jump", "j").contains("unknown action `jump`"));
        assert!(bad("quit", "hyper-q").contains("`hyper-q` is not a key"));
    }
}
