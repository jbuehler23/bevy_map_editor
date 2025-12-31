//! Project file save/load operations

use super::Project;
use std::path::Path;

#[derive(Debug)]
pub enum ProjectError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
    NoPath,
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::IoError(e) => write!(f, "IO error: {}", e),
            ProjectError::ParseError(e) => write!(f, "Parse error: {}", e),
            ProjectError::SerializeError(e) => write!(f, "Serialize error: {}", e),
            ProjectError::NoPath => write!(f, "No file path set"),
        }
    }
}

impl std::error::Error for ProjectError {}

impl Project {
    /// Load project from file
    pub fn load(path: &Path) -> Result<Self, ProjectError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ProjectError::IoError(e.to_string()))?;

        let mut project: Project =
            serde_json::from_str(&content).map_err(|e| ProjectError::ParseError(e.to_string()))?;

        project.path = Some(path.to_path_buf());

        // Build lookup indices for O(1) access to levels, tilesets, etc.
        project.rebuild_indices();

        // Validate and clean up any orphaned references (e.g., terrain sets pointing to deleted tilesets)
        project.validate_and_cleanup();

        // Only mark dirty if we haven't modified anything
        // (validate_and_cleanup sets dirty=true if it removes orphaned data)
        if !project.dirty {
            project.dirty = false;
        }

        Ok(project)
    }

    /// Save project to file
    pub fn save(&mut self, path: &Path) -> Result<(), ProjectError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProjectError::SerializeError(e.to_string()))?;

        std::fs::write(path, content).map_err(|e| ProjectError::IoError(e.to_string()))?;

        self.path = Some(path.to_path_buf());
        self.dirty = false;

        Ok(())
    }

    /// Save to current path if set
    pub fn save_current(&mut self) -> Result<(), ProjectError> {
        if let Some(path) = self.path.clone() {
            self.save(&path)
        } else {
            Err(ProjectError::NoPath)
        }
    }
}
