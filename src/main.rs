use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use mido::app::keys::Keymap;
use mido::app::{App, Settings, Source};
use mido::cli::Cli;
use mido::config::{self, Config};
use mido::markdown::parse;
use mido::project::Project;
use mido::render::ansi;
use mido::render::layout::{Options, layout_with};
use mido::render::theme::{ColorMode, Theme};

const MAN_PAGE: &str = include_str!(concat!(env!("OUT_DIR"), "/mido.1"));

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.man {
        io::stdout().lock().write_all(MAN_PAGE.as_bytes())?;
        return Ok(());
    }
    let listing_themes = cli
        .path
        .as_deref()
        .is_some_and(|p| p == Path::new("themes") && !p.exists());
    let start = match &cli.path {
        Some(path) if path != Path::new("-") && !listing_themes => path.clone(),
        _ => PathBuf::from("."),
    };
    let mut config = Config::load(cli.config.as_deref(), &start)?;
    if let Some(theme) = &cli.theme {
        config.set_theme(theme);
    }
    if let Some(width) = cli.width {
        config.width = Some(width);
    }
    let mode = Theme::detect_mode();
    if listing_themes {
        let mode = if io::stdout().is_terminal() {
            mode
        } else {
            ColorMode::Mono
        };
        return mido::themes::list(&config, mode, &mut io::stdout().lock()).map_err(Into::into);
    }

    let source = source(&cli, &config)?;
    let text = source.read().with_context(|| match &source {
        Source::File(path) => format!("cannot read {}", path.display()),
        Source::Project { project, file } => format!(
            "cannot read {}",
            file.as_ref()
                .map(|f| project.absolute(f))
                .unwrap_or_default()
                .display()
        ),
        Source::Stdin => "cannot read stdin".to_string(),
    })?;

    let interactive = io::stdout().is_terminal();
    let theme = config
        .resolve_theme(|| {
            if interactive {
                config::detect_background()
            } else {
                mido::render::theme::Kind::Dark
            }
        })?
        .with_mode(mode);

    if cli.print || !interactive {
        let color = cli.print && theme.mode != ColorMode::Mono;
        let doc = parse(&text);
        let options = Options {
            front_matter: config.front_matter,
            ..Options::default()
        };
        let width = config.width.unwrap_or_else(print_width).max(20);
        let out = layout_with(&doc, &theme, width, &options);
        return ansi::write(&out.lines, &mut io::stdout().lock(), color).map_err(Into::into);
    }

    let keymap = Keymap::with_overrides(&config.keys).map_err(|e| {
        anyhow::anyhow!(
            "{e}{}",
            config
                .files
                .last()
                .map(|f| format!(" (in {})", f.display()))
                .unwrap_or_default()
        )
    })?;
    let settings = Settings {
        theme,
        max_width: config.width,
        gutter: config.gutter,
        keymap,
        front_matter: config.front_matter,
        extensions: config.extensions.clone(),
    };
    let mut app = App::with_settings(source, &text, settings);
    app.set_remote_images(cli.remote_images);
    app.run()
}

fn source(cli: &Cli, config: &Config) -> Result<Source> {
    let project = |root: &Path| {
        let project = Project::scan_with(root, &config.extensions);
        let file = project.entry_file();
        Source::Project { project, file }
    };
    match &cli.path {
        Some(path) if path == Path::new("-") => Ok(Source::Stdin),
        Some(path) if path == Path::new("docs") && !path.exists() => {
            let root = mido::docs::extract().context("cannot unpack the bundled documentation")?;
            Ok(project(&root))
        }
        Some(path) if path.is_dir() => Ok(project(path)),
        Some(path) => Ok(Source::File(path.clone())),
        None if !io::stdin().is_terminal() => Ok(Source::Stdin),
        None => {
            let source = project(Path::new("."));
            if let Source::Project {
                project,
                file: None,
            } = &source
            {
                bail!(
                    "no Markdown files in {}\n\nUsage: mido <FILE or FOLDER>  (or pipe Markdown on stdin)",
                    project.root.display()
                );
            }
            Ok(source)
        }
    }
}

fn print_width() -> usize {
    if io::stdout().is_terminal() {
        crossterm::terminal::size()
            .map(|(cols, _)| cols as usize)
            .unwrap_or(80)
    } else {
        80
    }
}
