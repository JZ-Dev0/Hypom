use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: Option<PathBuf>,
    #[serde(default = "default_autosave_ms")]
    pub autosave_ms: u64,
    #[serde(default = "default_fecha_orden")]
    pub fecha_orden: String,
    #[serde(default = "default_clipboard_clear")]
    pub clipboard_clear: u64,
    #[serde(default = "default_secret_names")]
    pub secret_names: Vec<String>,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_autosave_ms() -> u64 { 800 }
fn default_fecha_orden() -> String { "dmy".to_string() }
fn default_clipboard_clear() -> u64 { 30 }
fn default_secret_names() -> Vec<String> {
    vec![
        "pass".into(), "password".into(), "contra".into(), "contraseña".into(),
        "clave".into(), "token".into(), "secret".into(), "pin".into(),
    ]
}
fn default_font_size() -> f32 { 15.0 }
fn default_theme() -> String { "system".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: None,
            autosave_ms: default_autosave_ms(),
            fecha_orden: default_fecha_orden(),
            clipboard_clear: default_clipboard_clear(),
            secret_names: default_secret_names(),
            font_size: default_font_size(),
            theme: default_theme(),
        }
    }
}

impl Config {
    pub fn config_file_path() -> Option<PathBuf> {
        let proj = ProjectDirs::from("com", "hypom", "Hypom")?;
        Some(proj.config_dir().join("config.toml"))
    }

    pub fn load() -> Self {
        let path = match Self::config_file_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = match Self::config_file_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        std::fs::write(path, content)
    }
}

