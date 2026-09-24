use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::project::default_extensions;
use crate::render::glyphs::{GlyphTier, Glyphs};
use crate::render::layout::FrontMatterView;
use crate::render::theme::{self, BUILTIN, Kind, Theme};

pub const FILE_NAME: &str = "config.toml";
pub const PROJECT_FILE: &str = ".mido.toml";
pub const DEFAULT_GUTTER: u16 = 2;
const MAX_EXTENDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Background {
    #[default]
    Auto,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FrontMatterSetting {
    Collapsed,
    Expanded,
    Hidden,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    theme: Option<String>,
    dark_theme: Option<String>,
    light_theme: Option<String>,
    background: Option<Background>,
    glyphs: Option<GlyphTier>,
    width: Option<usize>,
    gutter: Option<u16>,
    front_matter: Option<FrontMatterSetting>,
    extensions: Option<Vec<String>>,
    #[serde(default)]
    keys: BTreeMap<String, Keys>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Keys {
    One(String),
    Many(Vec<String>),
}

/// A theme name or path, with the folder a relative path is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeRef {
    pub value: String,
    pub dir: PathBuf,
}

impl ThemeRef {
    fn named(value: &str) -> Self {
        Self {
            value: value.to_string(),
            dir: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Option<ThemeRef>,
    pub dark_theme: ThemeRef,
    pub light_theme: ThemeRef,
    pub background: Background,
    pub glyphs: GlyphTier,
    pub width: Option<usize>,
    pub gutter: u16,
    pub front_matter: FrontMatterView,
    pub extensions: Vec<String>,
    pub keys: BTreeMap<String, Vec<String>>,
    pub files: Vec<PathBuf>,
    pub themes_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: None,
            dark_theme: ThemeRef::named(theme::DEFAULT_DARK),
            light_theme: ThemeRef::named(theme::DEFAULT_LIGHT),
            background: Background::Auto,
            glyphs: GlyphTier::default(),
            width: None,
            gutter: DEFAULT_GUTTER,
            front_matter: FrontMatterView::Collapsed,
            extensions: default_extensions(),
            keys: BTreeMap::new(),
            files: Vec::new(),
            themes_dir: None,
        }
    }
}

/// `MIDO_CONFIG_DIR`, then `$XDG_CONFIG_HOME/mido`, then `~/.config/mido`, or the roaming
/// AppData folder on Windows.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("MIDO_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("mido"));
    }
    if cfg!(windows) {
        return directories::ProjectDirs::from("", "", "mido")
            .map(|d| d.config_dir().to_path_buf());
    }
    directories::BaseDirs::new().map(|d| d.home_dir().join(".config").join("mido"))
}

impl Config {
    /// Reads the user config (or `explicit`), then the nearest `.mido.toml` at or above `start`.
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Self> {
        let dir = config_dir();
        let mut config = Self {
            themes_dir: dir.as_ref().map(|d| d.join("themes")),
            ..Self::default()
        };
        let user = match explicit {
            Some(path) => {
                if !path.exists() {
                    bail!("no config file at {}", path.display());
                }
                Some(path.to_path_buf())
            }
            None => dir.map(|d| d.join(FILE_NAME)).filter(|p| p.is_file()),
        };
        if let Some(path) = &user {
            config.apply(path)?;
        }
        if let Some(project) = find_project_file(start)
            && user.as_ref().is_none_or(|u| !same_file(u, &project))
        {
            config.apply(&project)?;
        }
        Ok(config)
    }

    fn apply(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        self.merge(&text, path.parent().unwrap_or(Path::new("")))
            .with_context(|| format!("in {}", path.display()))?;
        self.files.push(path.to_path_buf());
        Ok(())
    }

    pub fn merge(&mut self, text: &str, dir: &Path) -> Result<()> {
        let file: File =
            toml::from_str(text).map_err(|e| anyhow::anyhow!(e.message().to_string()))?;
        let at = |value: String| ThemeRef {
            value,
            dir: dir.to_path_buf(),
        };
        if let Some(theme) = file.theme {
            self.theme = (theme != "auto").then(|| at(theme));
        }
        if let Some(theme) = file.dark_theme {
            self.dark_theme = at(theme);
        }
        if let Some(theme) = file.light_theme {
            self.light_theme = at(theme);
        }
        if let Some(background) = file.background {
            self.background = background;
        }
        if let Some(glyphs) = file.glyphs {
            self.glyphs = glyphs;
        }
        if let Some(width) = file.width {
            if width < 20 {
                bail!("width must be at least 20, got {width}");
            }
            self.width = Some(width);
        }
        if let Some(gutter) = file.gutter {
            if gutter > 16 {
                bail!("gutter must be at most 16, got {gutter}");
            }
            self.gutter = gutter;
        }
        if let Some(view) = file.front_matter {
            self.front_matter = match view {
                FrontMatterSetting::Collapsed => FrontMatterView::Collapsed,
                FrontMatterSetting::Expanded => FrontMatterView::Expanded,
                FrontMatterSetting::Hidden => FrontMatterView::Hidden,
            };
        }
        if let Some(extensions) = file.extensions {
            let extensions: Vec<String> = extensions
                .iter()
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
            if extensions.is_empty() {
                bail!("extensions needs at least one entry");
            }
            self.extensions = extensions;
        }
        for (action, keys) in file.keys {
            let keys = match keys {
                Keys::One(key) => vec![key],
                Keys::Many(keys) => keys,
            };
            self.keys.insert(action, keys);
        }
        Ok(())
    }

    pub fn set_theme(&mut self, value: &str) {
        self.theme = (value != "auto").then(|| ThemeRef {
            value: value.to_string(),
            dir: PathBuf::new(),
        });
    }

    /// The theme to use: the named one, or the dark or light one for the terminal background.
    /// `detect` runs only when both are needed and `background` is auto.
    pub fn resolve_theme(&self, detect: impl FnOnce() -> Kind) -> Result<Theme> {
        let chosen = match &self.theme {
            Some(theme) => theme,
            None => match self.background_kind(detect) {
                Kind::Dark => &self.dark_theme,
                Kind::Light => &self.light_theme,
            },
        };
        let theme = self.load_theme(chosen, 0)?;
        Ok(theme.with_glyphs(Glyphs::for_tier(self.glyphs)))
    }

    fn background_kind(&self, detect: impl FnOnce() -> Kind) -> Kind {
        match self.background {
            Background::Dark => Kind::Dark,
            Background::Light => Kind::Light,
            Background::Auto => detect(),
        }
    }

    fn load_theme(&self, theme: &ThemeRef, depth: usize) -> Result<Theme> {
        if depth > MAX_EXTENDS {
            bail!("theme `{}` extends too deep, is there a loop?", theme.value);
        }
        let (name, path) = match self.theme_file(theme) {
            Some(found) => found,
            None => {
                if let Some(builtin) = Theme::builtin(&theme.value) {
                    return Ok(builtin);
                }
                bail!(
                    "no theme `{}`. Built in: {}. User themes go in {}",
                    theme.value,
                    BUILTIN
                        .iter()
                        .map(|(n, _)| *n)
                        .collect::<Vec<_>>()
                        .join(", "),
                    self.themes_dir.as_ref().map_or_else(
                        || "the config folder".to_string(),
                        |d| d.display().to_string()
                    )
                );
            }
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let error = RefCell::new(None);
        let base = |parent: &str| {
            if parent == name {
                return Theme::builtin(parent);
            }
            match self.load_theme(&ThemeRef::named(parent), depth + 1) {
                Ok(theme) => Some(theme),
                Err(e) => {
                    error.borrow_mut().get_or_insert(e);
                    None
                }
            }
        };
        match Theme::parse(&name, &text, &base) {
            Ok(theme) => Ok(theme),
            Err(e) => match error.into_inner() {
                Some(inner) => Err(inner.context(format!("{} ({})", e, path.display()))),
                None => bail!("{e} ({})", path.display()),
            },
        }
    }

    /// A path, or a name found in the user themes folder. Built-in names return `None`.
    fn theme_file(&self, theme: &ThemeRef) -> Option<(String, PathBuf)> {
        let value = &theme.value;
        if value.ends_with(".toml") || value.contains('/') || value.contains('\\') {
            let path = theme.dir.join(value);
            let name = path.file_stem()?.to_string_lossy().into_owned();
            return Some((name, path));
        }
        let path = self.themes_dir.as_ref()?.join(format!("{value}.toml"));
        path.is_file().then(|| (value.clone(), path))
    }

    /// Built-in themes and the ones in the user themes folder, by name.
    pub fn theme_list(&self) -> Vec<ThemeEntry> {
        let mut out: Vec<ThemeEntry> = BUILTIN
            .iter()
            .map(|(name, _)| ThemeEntry {
                name: name.to_string(),
                path: None,
                theme: Theme::builtin(name).ok_or_else(|| "missing".to_string()),
            })
            .collect();
        let Some(dir) = &self.themes_dir else {
            return out;
        };
        let Ok(read) = std::fs::read_dir(dir) else {
            return out;
        };
        let mut paths: Vec<PathBuf> = read
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                continue;
            };
            let theme = self
                .load_theme(&ThemeRef::named(&name), 0)
                .map_err(|e| format!("{e:#}"));
            out.retain(|e| e.name != name);
            out.push(ThemeEntry {
                name,
                path: Some(path),
                theme,
            });
        }
        out
    }
}

pub struct ThemeEntry {
    pub name: String,
    pub path: Option<PathBuf>,
    pub theme: Result<Theme, String>,
}

/// Light when the terminal answers an OSC 11 query with a light background, or `COLORFGBG`
/// says so. Dark when it cannot tell, and never asked unless stdin and stdout are both a tty.
pub fn detect_background() -> Kind {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Kind::Dark;
    }
    match terminal_light::luma() {
        Ok(luma) if luma > 0.6 => Kind::Light,
        _ => Kind::Dark,
    }
}

fn find_project_file(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().ok()?;
    let dir = if start.is_dir() {
        start.as_path()
    } else {
        start.parent()?
    };
    dir.ancestors()
        .map(|d| d.join(PROJECT_FILE))
        .find(|p| p.is_file())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merged(text: &str) -> Result<Config> {
        let mut config = Config::default();
        config.merge(text, Path::new("/cfg"))?;
        Ok(config)
    }

    #[test]
    fn every_setting_reads() {
        let config = merged(
            r#"
theme = "nord"
light_theme = "mine.toml"
background = "light"
glyphs = "ascii"
width = 90
gutter = 4
front_matter = "hidden"
extensions = [".md", "TXT"]

[keys]
quit = "x"
scroll_down = ["j", "ctrl-n"]
"#,
        )
        .unwrap();
        assert_eq!(
            config.theme,
            Some(ThemeRef {
                value: "nord".into(),
                dir: "/cfg".into()
            })
        );
        assert_eq!(config.light_theme.value, "mine.toml");
        assert_eq!(config.background, Background::Light);
        assert_eq!(config.glyphs, GlyphTier::Ascii);
        assert_eq!(config.width, Some(90));
        assert_eq!(config.gutter, 4);
        assert_eq!(config.front_matter, FrontMatterView::Hidden);
        assert_eq!(config.extensions, ["md", "txt"]);
        assert_eq!(config.keys["quit"], ["x"]);
        assert_eq!(config.keys["scroll_down"], ["j", "ctrl-n"]);
    }

    #[test]
    fn later_files_override_earlier_ones() {
        let mut config = merged("theme = \"nord\"\nwidth = 90\n[keys]\nquit = \"x\"").unwrap();
        config
            .merge(
                "theme = \"auto\"\n[keys]\nhelp = \"F1\"",
                Path::new("/project"),
            )
            .unwrap();
        assert_eq!(config.theme, None);
        assert_eq!(config.width, Some(90));
        assert_eq!(config.keys.len(), 2);
    }

    #[test]
    fn bad_settings_are_errors() {
        let err = |text: &str| format!("{:#}", merged(text).unwrap_err());
        assert!(err("thme = \"nord\"").contains("unknown field `thme`"));
        assert!(err("glyphs = \"emoji\"").contains("unknown variant `emoji`"));
        assert!(err("width = 5").contains("at least 20"));
        assert!(err("extensions = []").contains("at least one"));
    }

    #[test]
    fn auto_picks_by_background() {
        let config = Config::default();
        assert_eq!(
            config.resolve_theme(|| Kind::Light).unwrap().name,
            "mido-light"
        );
        assert_eq!(
            config.resolve_theme(|| Kind::Dark).unwrap().name,
            "mido-dark"
        );
        let forced = merged("background = \"light\"").unwrap();
        assert_eq!(
            forced.resolve_theme(|| panic!("not asked")).unwrap().name,
            "mido-light"
        );
        let named = merged("theme = \"nord\"").unwrap();
        assert_eq!(
            named.resolve_theme(|| panic!("not asked")).unwrap().name,
            "nord"
        );
    }

    #[test]
    fn themes_load_from_paths_and_the_user_folder() {
        let dir = tempfile::tempdir().unwrap();
        let themes = dir.path().join("themes");
        std::fs::create_dir(&themes).unwrap();
        std::fs::write(
            themes.join("rose.toml"),
            "schema = 1\nextends = \"nord\"\n[colors]\naccent = \"#f5c2e7\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("petal.toml"),
            "schema = 1\nextends = \"rose\"\n[colors]\nlink = \"red\"\n",
        )
        .unwrap();
        std::fs::write(themes.join("ping.toml"), "schema = 1\nextends = \"pong\"\n").unwrap();
        std::fs::write(themes.join("pong.toml"), "schema = 1\nextends = \"ping\"\n").unwrap();
        std::fs::write(themes.join("nord.toml"), "schema = 1\nextends = \"nord\"\n").unwrap();
        let mut config = Config {
            themes_dir: Some(themes),
            ..Config::default()
        };
        config.merge("theme = \"petal.toml\"", dir.path()).unwrap();
        let theme = config.resolve_theme(|| Kind::Dark).unwrap();
        assert_eq!(theme.name, "petal");
        assert_eq!(Some(theme.accent), theme::parse_color("#f5c2e7"));
        assert_eq!(theme.fg, Theme::builtin("nord").unwrap().fg);

        config.set_theme("nord");
        assert_eq!(
            config.resolve_theme(|| Kind::Dark).unwrap().bg,
            Theme::builtin("nord").unwrap().bg
        );
        config.set_theme("ping");
        assert!(
            format!("{:#}", config.resolve_theme(|| Kind::Dark).unwrap_err()).contains("too deep")
        );
        config.set_theme("missing");
        assert!(
            format!("{:#}", config.resolve_theme(|| Kind::Dark).unwrap_err())
                .contains("no theme `missing`")
        );

        let names: Vec<String> = config.theme_list().into_iter().map(|e| e.name).collect();
        assert!(names.contains(&"rose".to_string()) && names.contains(&"nord".to_string()));
    }

    #[test]
    fn project_file_is_found_above_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dir.path().join(PROJECT_FILE), "width = 72\n").unwrap();
        std::fs::write(nested.join("page.md"), "# hi\n").unwrap();
        let found = find_project_file(&nested.join("page.md")).unwrap();
        assert!(same_file(&found, &dir.path().join(PROJECT_FILE)));
    }
}
