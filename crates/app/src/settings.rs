use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use tangibles_chess_config::GameConfig;

/// Persisted app-level settings (last used game configuration, sound,
/// daemon endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Last configured game setup.
    pub last_config: GameConfig,
    /// Sound effects enabled.
    pub sound_enabled: bool,
    /// tangible-objects daemon GraphQL endpoint.
    pub daemon_url: String,
    /// Locale override; `None` = negotiate from the environment.
    pub locale: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_config: GameConfig::default(),
            sound_enabled: true,
            daemon_url: "http://127.0.0.1:8080".to_string(),
            locale: None,
        }
    }
}

impl AppSettings {
    /// Settings file path below the user config dir.
    fn path() -> PathBuf {
        glib::user_config_dir().join("tangibles-chess").join("settings.toml")
    }

    /// Loads settings; falls back to defaults on any error.
    pub fn load() -> Self {
        let path = Self::path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        toml::from_str(&text).unwrap_or_default()
    }

    /// Persists settings, creating the directory when needed.
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
