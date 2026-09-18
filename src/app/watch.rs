use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<PathBuf>,
}

impl FileWatcher {
    pub fn file(path: &Path) -> notify::Result<Self> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ".".into());
        Self::watch(&dir, RecursiveMode::NonRecursive)
    }

    pub fn tree(root: &Path) -> notify::Result<Self> {
        Self::watch(root, RecursiveMode::Recursive)
    }

    fn watch(dir: &Path, mode: RecursiveMode) -> notify::Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        })?;
        watcher.watch(dir, mode)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn drain(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        while let Ok(path) = self.rx.try_recv() {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }
}
