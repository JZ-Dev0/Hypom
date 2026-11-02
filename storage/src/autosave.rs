use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AutosaveManager {
    pub debounce_duration: Duration,
    modified_times: HashMap<PathBuf, Instant>,
}

impl AutosaveManager {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            debounce_duration: Duration::from_millis(debounce_ms),
            modified_times: HashMap::new(),
        }
    }

    pub fn mark_modified(&mut self, path: PathBuf) {
        self.modified_times.insert(path, Instant::now());
    }

    pub fn remove(&mut self, path: &PathBuf) {
        self.modified_times.remove(path);
    }

    pub fn check_should_save(&self, path: &PathBuf) -> bool {
        if let Some(time) = self.modified_times.get(path) {
            time.elapsed() >= self.debounce_duration
        } else {
            false
        }
    }
}

