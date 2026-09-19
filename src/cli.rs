use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mido",
    version,
    about = "Markdown In, Document Out. A terminal Markdown reader."
)]
pub struct Cli {
    /// Markdown file or folder to read, - for stdin, or docs for the bundled documentation
    pub path: Option<PathBuf>,

    /// Print styled text to stdout instead of opening the viewer
    #[arg(short, long)]
    pub print: bool,

    /// Cap the content width in columns (default: the full terminal width)
    #[arg(short, long, value_name = "COLS")]
    pub width: Option<usize>,

    /// Download remote images, cached in the user cache dir with a 10 MB cap
    #[arg(long)]
    pub remote_images: bool,

    /// Print the man page as roff to stdout
    #[arg(long)]
    pub man: bool,
}
