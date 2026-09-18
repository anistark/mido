use std::ffi::OsString;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<()>,
}

impl FileWatcher {
    pub fn new(path: &Path) -> notify::Result<Self> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ".".into());
        let name: Option<OsString> = path.file_name().map(OsString::from);
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && event.paths.iter().any(|p| p.file_name() == name.as_deref())
            {
                let _ = tx.send(());
            }
        })?;
        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn changed(&self) -> bool {
        let mut any = false;
        while self.rx.try_recv().is_ok() {
            any = true;
        }
        any
    }
}
