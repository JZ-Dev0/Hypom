use std::time::{Duration, Instant};
use arboard::Clipboard;

pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
    pending_clear: Option<(String, Instant, Duration)>,
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardManager {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok();
        Self {
            clipboard,
            pending_clear: None,
        }
    }

    pub fn copy_text(&mut self, text: &str) -> bool {
        if let Some(cb) = &mut self.clipboard {
            self.pending_clear = None;
            cb.set_text(text.to_string()).is_ok()
        } else {
            false
        }
    }

    pub fn copy_secret(&mut self, secret: &str, clear_after_secs: u64) -> bool {
        if let Some(cb) = &mut self.clipboard {
            if cb.set_text(secret.to_string()).is_ok() {
                if clear_after_secs > 0 {
                    self.pending_clear = Some((
                        secret.to_string(),
                        Instant::now(),
                        Duration::from_secs(clear_after_secs),
                    ));
                }
                return true;
            }
        }
        false
    }

    pub fn update(&mut self) {
        if let Some((secret, started, duration)) = &self.pending_clear {
            if started.elapsed() >= *duration {
                let should_clear = if let Some(cb) = &mut self.clipboard {
                    if let Ok(current) = cb.get_text() {
                        current == *secret
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_clear {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.clear();
                    }
                }
                self.pending_clear = None;
            }
        }
    }
}

