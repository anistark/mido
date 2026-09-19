use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use mido::app::{App, Source};
use mido::cli::Cli;
use mido::markdown::parse;
use mido::project::Project;
use mido::render::{ansi, layout::layout, theme::Theme};

const MAN_PAGE: &str = include_str!(concat!(env!("OUT_DIR"), "/mido.1"));

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.man {
        io::stdout().lock().write_all(MAN_PAGE.as_bytes())?;
        return Ok(());
    }
    let source = source(&cli)?;
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

    if cli.print || !io::stdout().is_terminal() {
        let theme = Theme::for_terminal();
        let color = cli.print && theme.mode != mido::render::theme::ColorMode::Mono;
        let doc = parse(&text);
        let out = layout(&doc, &theme, cli.width.unwrap_or_else(print_width).max(20));
        return ansi::write(&out.lines, &mut io::stdout().lock(), color).map_err(Into::into);
    }

    let mut app = App::new(source, &text, cli.width);
    app.set_remote_images(cli.remote_images);
    app.run()
}

fn source(cli: &Cli) -> Result<Source> {
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
            let here = PathBuf::from(".");
            let source = project(&here);
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

fn project(root: &Path) -> Source {
    let project = Project::scan(root);
    let file = project.entry_file();
    Source::Project { project, file }
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
