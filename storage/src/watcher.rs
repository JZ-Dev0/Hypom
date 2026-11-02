use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

pub enum WorkspaceEvent {
    Changed(PathBuf),
    Other,
}

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    pub rx: Receiver<WorkspaceEvent>,
}

impl WorkspaceWatcher {
    pub fn new(root: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let _ = tx.send(WorkspaceEvent::Changed(path));
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(root, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }
}

