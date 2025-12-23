//! Preferences file save/load operations

use super::EditorPreferences;
use directories::ProjectDirs;
use std::path::PathBuf;

const PREFERENCES_FILE: &str = "preferences.json";

#[derive(Debug)]
pub enum PreferencesError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
    NoConfigDir,
}

impl std::fmt::Display for PreferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferencesError::IoError(e) => write!(f, "IO error: {}", e),
            PreferencesError::ParseError(e) => write!(f, "Parse error: {}", e),
            PreferencesError::SerializeError(e) => write!(f, "Serialize error: {}", e),
            PreferencesError::NoConfigDir => write!(f, "Could not determine config directory"),
        }
    }
}

impl std::error::Error for PreferencesError {}

impl EditorPreferences {
    /// Get the config directory path for the editor
    pub fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "bevy_map_editor", "bevy_map_editor")
            .map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Get the preferences file path
    pub fn preferences_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join(PREFERENCES_FILE))
    }

    /// Load preferences from file, returning defaults if not found
    pub fn load() -> Self {
        match Self::load_from_file() {
            Ok(prefs) => prefs,
            Err(e) => {
                bevy::log::warn!("Could not load preferences: {}. Using defaults.", e);
                Self::default()
            }
        }
    }

    /// Load preferences from file
    fn load_from_file() -> Result<Self, PreferencesError> {
        let path = Self::preferences_path().ok_or(PreferencesError::NoConfigDir)?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content =
            std::fs::read_to_string(&path).map_err(|e| PreferencesError::IoError(e.to_string()))?;

        serde_json::from_str(&content).map_err(|e| PreferencesError::ParseError(e.to_string()))
    }

    /// Save preferences to file
    pub fn save(&self) -> Result<(), PreferencesError> {
        let dir = Self::config_dir().ok_or(PreferencesError::NoConfigDir)?;
        let path = dir.join(PREFERENCES_FILE);

        // Create config directory if it doesn't exist
        std::fs::create_dir_all(&dir).map_err(|e| PreferencesError::IoError(e.to_string()))?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| PreferencesError::SerializeError(e.to_string()))?;

        std::fs::write(&path, content).map_err(|e| PreferencesError::IoError(e.to_string()))?;

        bevy::log::info!("Saved preferences to {:?}", path);
        Ok(())
    }
}
