use std::io::{self, IsTerminal};
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Parser;
use mido::app::{App, Source};
use mido::cli::Cli;
use mido::markdown::parse;
use mido::render::{ansi, layout::layout, theme::Theme};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = source(&cli)?;
    let text = source.read().with_context(|| match &source {
        Source::File(path) => format!("cannot read {}", path.display()),
        Source::Stdin => "cannot read stdin".to_string(),
    })?;

    if cli.print || !io::stdout().is_terminal() {
        let theme = Theme::for_terminal();
        let color = cli.print && theme.mode != mido::render::theme::ColorMode::Mono;
        let doc = parse(&text);
        let out = layout(&doc, &theme, cli.width.unwrap_or_else(print_width).max(20));
        return ansi::write(&out.lines, &mut io::stdout().lock(), color).map_err(Into::into);
    }

    App::new(source, &text, cli.width).run()
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

fn source(cli: &Cli) -> Result<Source> {
    match &cli.path {
        Some(path) if path == Path::new("-") => Ok(Source::Stdin),
        Some(path) if path.is_dir() => {
            bail!(
                "{} is a directory, pass a Markdown file. Project mode arrives in 0.2",
                path.display()
            )
        }
        Some(path) => Ok(Source::File(path.clone())),
        None if !io::stdin().is_terminal() => Ok(Source::Stdin),
        None => bail!("no file given\n\nUsage: mido <FILE>  (or pipe Markdown on stdin)"),
    }
}
