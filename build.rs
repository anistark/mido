use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::CommandFactory;

#[path = "src/cli.rs"]
mod cli;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli.rs");
    bundle_docs(&root, &out.join("docs"));
    man_page(&out);
}

fn bundle_docs(root: &Path, out: &Path) {
    let pages = out.join("docs");
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(&pages).unwrap();
    for name in ["README.md", "CHANGELOG.md"] {
        copy(&root.join(name), &out.join(name));
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(root.join("docs"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort();
    for path in entries {
        copy(&path, &pages.join(path.file_name().unwrap()));
    }
}

fn copy(from: &Path, to: &Path) {
    println!("cargo:rerun-if-changed={}", from.display());
    fs::copy(from, to).unwrap_or_else(|err| panic!("cannot copy {}: {err}", from.display()));
}

fn man_page(out: &Path) {
    let mut buf = Vec::new();
    clap_mangen::Man::new(cli::Cli::command())
        .render(&mut buf)
        .unwrap();
    fs::write(out.join("mido.1"), buf).unwrap();
}
