//! Bevy CLI detection and installation
//!
//! This module provides functions to check if Bevy CLI is installed,
//! install it if needed, and create new game projects using templates.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// The GitHub URL for the Bevy CLI repository
pub const BEVY_CLI_REPO: &str = "https://github.com/TheBevyFlock/bevy_cli";

/// The tag/version of Bevy CLI to install
pub const BEVY_CLI_TAG: &str = "cli-v0.1.0-alpha.2";

/// The template URL for creating game projects with bevy_map_editor support
pub const GAME_TEMPLATE_URL: &str = "https://github.com/jbuehler23/bevy_map_editor_template";

/// Error type for Bevy CLI operations
#[derive(Debug)]
pub enum BevyCliError {
    /// IO error during command execution
    IoError(io::Error),
    /// Cargo is not installed or not found in PATH
    CargoNotFound,
    /// Bevy CLI installation failed
    InstallFailed(String),
    /// Project creation failed
    CreateFailed(String),
    /// Invalid project name
    InvalidProjectName(String),
}

impl std::fmt::Display for BevyCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BevyCliError::IoError(e) => write!(f, "IO error: {}", e),
            BevyCliError::CargoNotFound => {
                write!(f, "Cargo not found. Please install Rust toolchain.")
            }
            BevyCliError::InstallFailed(msg) => write!(f, "Bevy CLI installation failed: {}", msg),
            BevyCliError::CreateFailed(msg) => write!(f, "Project creation failed: {}", msg),
            BevyCliError::InvalidProjectName(name) => {
                write!(f, "Invalid project name: '{}'. Use lowercase letters, numbers, underscores, or hyphens.", name)
            }
        }
    }
}

impl std::error::Error for BevyCliError {}

impl From<io::Error> for BevyCliError {
    fn from(e: io::Error) -> Self {
        BevyCliError::IoError(e)
    }
}

/// Check if Bevy CLI is installed and available in PATH
pub fn is_bevy_cli_installed() -> bool {
    Command::new("bevy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if cargo is available
pub fn is_cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Install Bevy CLI using cargo
///
/// This runs: cargo install --git <repo> --tag <tag> --locked bevy_cli
pub fn install_bevy_cli() -> Result<(), BevyCliError> {
    if !is_cargo_available() {
        return Err(BevyCliError::CargoNotFound);
    }

    let output = Command::new("cargo")
        .args([
            "install",
            "--git",
            BEVY_CLI_REPO,
            "--tag",
            BEVY_CLI_TAG,
            "--locked",
            "bevy_cli",
        ])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BevyCliError::InstallFailed(stderr.to_string()))
    }
}

/// Create a new game project using Bevy CLI and the map editor template
///
/// # Arguments
/// * `name` - The project name (must be a valid crate name)
/// * `parent_dir` - The directory where the project folder will be created
///
/// The project will be created at `parent_dir/name/`
pub fn create_project(name: &str, parent_dir: &Path) -> Result<(), BevyCliError> {
    // Validate project name
    if !is_valid_crate_name(name) {
        return Err(BevyCliError::InvalidProjectName(name.to_string()));
    }

    // Use --define to pass placeholder values non-interactively
    // Args after "--" are forwarded to cargo-generate
    // This avoids "not a terminal" errors when running from GUI
    let output = Command::new("bevy")
        .args([
            "new",
            name,
            "--template",
            GAME_TEMPLATE_URL,
            "--",
            "--define",
            "author=",
            "--define",
            "description=A game created with bevy_map_editor",
        ])
        .current_dir(parent_dir)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BevyCliError::CreateFailed(stderr.to_string()))
    }
}

/// Check if a string is a valid Rust crate name
fn is_valid_crate_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must start with letter or underscore
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Must contain only alphanumeric, underscore, or hyphen
    // and must be lowercase
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Get the version of Bevy CLI if installed
pub fn get_bevy_cli_version() -> Option<String> {
    let output = Command::new("bevy").arg("--version").output().ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Some(version.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_crate_names() {
        assert!(is_valid_crate_name("my_game"));
        assert!(is_valid_crate_name("my-game"));
        assert!(is_valid_crate_name("game123"));
        assert!(is_valid_crate_name("_private"));
    }

    #[test]
    fn test_invalid_crate_names() {
        assert!(!is_valid_crate_name(""));
        assert!(!is_valid_crate_name("123game")); // starts with number
        assert!(!is_valid_crate_name("My_Game")); // uppercase
        assert!(!is_valid_crate_name("my game")); // space
    }
}
