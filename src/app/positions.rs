//! Where each file was left, kept in the user cache folder so a long document opens again at
//! the place it was read to. A position is a byte offset in the source, so it survives a
//! resize and lands close by after an edit.

use std::io;
use std::path::{Path, PathBuf};

use crate::edit::file::write_atomic;

const MAX_FILES: usize = 500;

#[derive(Debug, Default)]
pub struct Positions {
    path: PathBuf,
    /// Oldest first, so the front is dropped when the list is full.
    entries: Vec<(PathBuf, usize)>,
    changed: bool,
}

impl Positions {
    /// `positions` in the user cache folder.
    pub fn default_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "mido")?;
        Some(dirs.cache_dir().join("positions"))
    }

    /// Reads the list at `path`. A missing or unreadable file starts an empty one.
    pub fn load(path: PathBuf) -> Self {
        let entries = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (offset, file) = line.split_once('\t')?;
                Some((PathBuf::from(file), offset.parse().ok()?))
            })
            .collect();
        Self {
            path,
            entries,
            changed: false,
        }
    }

    pub fn get(&self, file: &Path) -> Option<usize> {
        let file = key(file);
        self.entries
            .iter()
            .rev()
            .find(|(f, _)| *f == file)
            .map(|(_, offset)| *offset)
    }

    /// Records where `file` was left. The top of the file is the default, so it is not kept.
    pub fn set(&mut self, file: &Path, offset: usize) {
        let file = key(file);
        if file.to_str().is_none_or(|f| f.contains(['\t', '\n'])) {
            return;
        }
        let before = self.entries.len();
        self.entries.retain(|(f, _)| *f != file);
        if offset > 0 {
            self.entries.push((file, offset));
        }
        if self.entries.len() > MAX_FILES {
            self.entries.drain(..self.entries.len() - MAX_FILES);
        }
        self.changed |= offset > 0 || self.entries.len() != before;
    }

    pub fn save(&mut self) -> io::Result<()> {
        if !self.changed {
            return Ok(());
        }
        let text: String = self
            .entries
            .iter()
            .filter_map(|(file, offset)| Some(format!("{offset}\t{}\n", file.to_str()?)))
            .collect();
        write_atomic(&self.path, text.as_bytes())?;
        self.changed = false;
        Ok(())
    }
}

fn key(file: &Path) -> PathBuf {
    file.canonicalize().unwrap_or_else(|_| file.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_round_trip_and_stay_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cache/positions");
        let page = dir.path().join("page.md");
        std::fs::write(&page, "# Page\n").unwrap();
        let mut positions = Positions::load(store.clone());
        assert_eq!(positions.get(&page), None);
        positions.set(&page, 120);
        positions.set(&dir.path().join("./page.md"), 140);
        positions.save().unwrap();

        let mut positions = Positions::load(store.clone());
        assert_eq!(positions.get(&page), Some(140));
        positions.set(&page, 0);
        assert_eq!(positions.get(&page), None);
        for i in 0..MAX_FILES + 10 {
            positions.set(Path::new(&format!("/nowhere/{i}.md")), i + 1);
        }
        positions.save().unwrap();
        let positions = Positions::load(store);
        assert_eq!(positions.entries.len(), MAX_FILES);
        assert_eq!(positions.get(Path::new("/nowhere/0.md")), None);
        assert_eq!(
            positions.get(Path::new(&format!("/nowhere/{}.md", MAX_FILES + 9))),
            Some(MAX_FILES + 10)
        );
    }
}
