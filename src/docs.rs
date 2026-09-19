use std::io;
use std::path::PathBuf;

use include_dir::{Dir, include_dir};

static BUNDLE: Dir = include_dir!("$OUT_DIR/docs");

pub fn pages() -> impl Iterator<Item = &'static str> {
    BUNDLE
        .get_dir("docs")
        .into_iter()
        .flat_map(|dir| dir.files())
        .filter_map(|file| file.path().file_name()?.to_str())
}

pub fn extract() -> io::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!("mido-docs-{}", env!("CARGO_PKG_VERSION")));
    std::fs::create_dir_all(&root)?;
    BUNDLE.extract(&root)?;
    Ok(root.join("docs"))
}
