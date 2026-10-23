use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use mido::app::keys::Keymap;
use mido::app::{App, Positions, Settings, Source};
use mido::cli::{BANNER, Cli};
use mido::config::{self, Config};
use mido::markdown::parse;
use mido::project::Project;
use mido::render::ansi;
use mido::render::layout::{Link, Options, layout_with};
use mido::render::theme::{ColorMode, Theme};
use mido::render::wrap::LinkKind;

const MAN_PAGE: &str = include_str!(concat!(env!("OUT_DIR"), "/mido.1"));

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    if cli
        .path
        .as_deref()
        .is_some_and(|p| p == Path::new("completions") && !p.exists())
    {
        return completions(cli.file.as_deref());
    }
    let creating = cli
        .path
        .as_deref()
        .is_some_and(|p| p == Path::new("new") && !p.exists());
    if creating {
        cli.path = Some(
            cli.file
                .take()
                .context("mido new needs a file name, like mido new notes.md")?,
        );
    } else if let Some(file) = &cli.file {
        bail!(
            "unexpected argument {}, a second path only goes with new or completions",
            file.display()
        );
    }
    if cli.version {
        println!("{BANNER}");
        return Ok(());
    }
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
    let mode = if cli.no_color {
        ColorMode::Mono
    } else {
        Theme::detect_mode()
    };
    if listing_themes {
        let mode = if io::stdout().is_terminal() {
            mode
        } else {
            ColorMode::Mono
        };
        return mido::themes::list(&config, mode, &mut io::stdout().lock()).map_err(Into::into);
    }

    let source = source(&cli, &config)?;
    let missing = matches!(&source, Source::File(path) if !path.exists());
    let editing = creating || (missing && is_markdown(cli.path.as_deref(), &config));
    if editing && (cli.print || !io::stdout().is_terminal()) {
        bail!("the editor needs a terminal");
    }
    let text = if editing && missing {
        Ok(Default::default())
    } else {
        source.read()
    };
    let text = text.with_context(|| match &source {
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
    let detect = || {
        if interactive {
            config::detect_background()
        } else {
            mido::render::theme::Kind::Dark
        }
    };
    let mut warnings = config.warnings.clone();
    let name = match &source {
        Source::File(path) => path.display().to_string(),
        Source::Project { file, .. } => file
            .as_ref()
            .map(|f| f.display().to_string())
            .unwrap_or_default(),
        Source::Stdin => "stdin".to_string(),
    };
    let printing = cli.print || !interactive;
    if text.lossy || !printing {
        warnings.extend(text.notice(&name));
    }
    let theme = match config.resolve_theme(detect) {
        Ok(theme) => theme,
        Err(err) => match config.without_project_themes() {
            Some(fallback) => {
                let project = config.files.last().map(|f| f.display().to_string());
                warnings.push(format!(
                    "{}: skipped its theme, {err:#}",
                    project.unwrap_or_default()
                ));
                config = fallback;
                config.resolve_theme(detect)?
            }
            None => return Err(err),
        },
    }
    .with_mode(mode);

    if printing {
        for warning in &warnings {
            eprintln!("mido: {warning}");
        }
        let color = cli.print && theme.mode != ColorMode::Mono;
        let doc = parse(&text);
        let options = Options {
            front_matter: config.front_matter,
            ..Options::default()
        };
        let width = config.width.unwrap_or_else(print_width).max(20);
        let out = layout_with(&doc, &theme, width, &options);
        let base = match &source {
            Source::File(path) => path.parent().map(Path::to_path_buf),
            Source::Project { project, file } => file
                .as_ref()
                .and_then(|f| project.absolute(f).parent().map(Path::to_path_buf)),
            Source::Stdin => None,
        };
        return ansi::write(
            &out.lines,
            &out.links,
            &mut io::stdout().lock(),
            color,
            |link| hyperlink(link, base.as_deref()),
        )
        .map_err(Into::into);
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
        split: config.split,
        split_ratio: config.split_ratio,
    };
    let mut app = App::with_settings(source, &text, settings);
    app.set_remote_images(cli.remote_images);
    if config.remember_position
        && !editing
        && let Some(path) = Positions::default_path()
    {
        app.remember_positions(Positions::load(path));
    }
    if let Some(first) = warnings.first() {
        app.set_notice(match warnings.len() {
            1 => first.clone(),
            n => format!("{first} (and {} more)", n - 1),
        });
    }
    if editing {
        app.start_editing();
    }
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

/// The URL a terminal should open for a link: web and mail links as they are, a relative
/// path that exists as a `file://` URL, and nothing for anchors, wikilinks and footnotes.
fn hyperlink(link: &Link, base: Option<&Path>) -> Option<String> {
    if link.kind != LinkKind::Url || link.url.starts_with('#') {
        return None;
    }
    if link.url.contains("://") || link.url.starts_with("mailto:") {
        return Some(link.url.clone());
    }
    let path = link.url.split('#').next().unwrap_or_default();
    let target = base?.join(path).canonicalize().ok()?;
    Some(format!(
        "file://{}",
        target
            .to_string_lossy()
            .replace('\\', "/")
            .replace(' ', "%20")
    ))
}

/// Prints the completion script for a shell to stdout.
fn completions(shell: Option<&Path>) -> Result<()> {
    const SHELLS: &str = "bash, zsh, fish, elvish or powershell";
    let name = shell
        .and_then(Path::to_str)
        .with_context(|| format!("mido completions needs a shell: {SHELLS}"))?;
    let shell: Shell = name
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown shell `{name}`, expected {SHELLS}"))?;
    clap_complete::generate(shell, &mut Cli::command(), "mido", &mut io::stdout().lock());
    Ok(())
}

fn is_markdown(path: Option<&Path>, config: &Config) -> bool {
    path.is_some_and(|p| mido::project::is_markdown(p, &config.extensions))
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
