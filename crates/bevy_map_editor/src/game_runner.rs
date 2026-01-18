//! Game process launching and management
//!
//! This module provides functions to launch, monitor, and terminate
//! game processes from the editor.

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;

/// Messages sent from the build thread to the UI
#[derive(Debug, Clone)]
pub enum BuildOutput {
    /// Raw output line from cargo
    Line(String),
    /// Parsed progress: (current, total) crates
    Progress(u32, u32),
    /// Name of crate currently being compiled
    CurrentCrate(String),
    /// Build completed successfully
    BuildComplete,
    /// Build failed with error message
    BuildFailed(String),
    /// Game process has started
    GameStarted,
    /// Game process has exited
    GameExited(Option<i32>),
}

/// State of a game build/run operation
#[derive(Debug, Clone, Default)]
pub enum GameBuildState {
    /// No build in progress
    #[default]
    Idle,
    /// Currently building
    Building {
        /// Progress as (current, total) crates
        progress: Option<(u32, u32)>,
        /// Name of crate currently compiling
        current_crate: Option<String>,
        /// Recent output lines (last N lines for UI display)
        output_lines: Vec<String>,
        /// Path to the full log file
        log_file_path: Option<PathBuf>,
    },
    /// Build complete, game running
    Running {
        /// Path to the full log file
        log_file_path: Option<PathBuf>,
    },
    /// Build/run finished (success or stopped)
    Finished {
        /// Path to the full log file
        log_file_path: Option<PathBuf>,
    },
    /// Build or run failed
    Failed {
        /// Error message
        message: String,
        /// Path to the full log file
        log_file_path: Option<PathBuf>,
    },
}

/// Get the log file path for the current build
pub fn get_build_log_path() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Use temp directory for build logs
    let temp_dir = std::env::temp_dir();
    temp_dir.join(format!("bevy_map_editor_build_{}.log", timestamp))
}

/// Error type for game launching operations
#[derive(Debug)]
pub enum LaunchError {
    /// IO error during command execution
    IoError(io::Error),
    /// Game project path not configured
    ProjectNotConfigured,
    /// Map file not saved
    MapNotSaved,
    /// Cargo not found in PATH
    CargoNotFound,
    /// Game launch failed
    LaunchFailed(String),
    /// Project directory doesn't exist
    ProjectNotFound(PathBuf),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::IoError(e) => write!(f, "IO error: {}", e),
            LaunchError::ProjectNotConfigured => {
                write!(
                    f,
                    "Game project not configured. Go to Project > Game Settings."
                )
            }
            LaunchError::MapNotSaved => write!(f, "Save the map project before running the game."),
            LaunchError::CargoNotFound => {
                write!(f, "Cargo not found. Please install Rust toolchain.")
            }
            LaunchError::LaunchFailed(msg) => write!(f, "Failed to launch game: {}", msg),
            LaunchError::ProjectNotFound(path) => {
                write!(f, "Game project not found at: {}", path.display())
            }
        }
    }
}

impl std::error::Error for LaunchError {}

impl From<io::Error> for LaunchError {
    fn from(e: io::Error) -> Self {
        LaunchError::IoError(e)
    }
}

/// Options for launching a game
pub struct LaunchOptions {
    /// Path to the game project directory (contains Cargo.toml)
    pub project_path: PathBuf,
    /// Whether to use release mode
    pub release: bool,
    /// Whether to enable hot-reload (adds 'hot_reload' feature)
    pub hot_reload: bool,
}

/// Result of a game launch attempt
pub struct LaunchResult {
    /// The spawned child process (if successful)
    pub child: Option<Child>,
    /// Error if launch failed
    pub error: Option<LaunchError>,
}

impl LaunchResult {
    /// Create a successful launch result
    pub fn success(child: Child) -> Self {
        Self {
            child: Some(child),
            error: None,
        }
    }

    /// Create a failed launch result
    pub fn failure(error: LaunchError) -> Self {
        Self {
            child: None,
            error: Some(error),
        }
    }
}

/// Launch a game project
///
/// Runs `cargo run` in the game project directory with optional flags:
/// - `--release` if release mode is enabled
/// - `--features hot_reload` if hot-reload is enabled
pub fn launch_game(options: &LaunchOptions) -> LaunchResult {
    // Verify project exists
    if !options.project_path.exists() {
        return LaunchResult::failure(LaunchError::ProjectNotFound(options.project_path.clone()));
    }

    if !options.project_path.join("Cargo.toml").exists() {
        return LaunchResult::failure(LaunchError::ProjectNotFound(options.project_path.clone()));
    }

    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    cmd.current_dir(&options.project_path);

    if options.release {
        cmd.arg("--release");
    }

    if options.hot_reload {
        cmd.args(["--features", "hot_reload"]);
    }

    // Don't capture output - let it go to console
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // Spawn the process
    match cmd.spawn() {
        Ok(child) => LaunchResult::success(child),
        Err(e) => LaunchResult::failure(LaunchError::LaunchFailed(e.to_string())),
    }
}

/// Check if a game process is still running
///
/// Returns true if the process is running, false if it has exited.
/// Also clears the Option if the process has exited.
pub fn is_game_running(child: &mut Option<Child>) -> bool {
    if let Some(ref mut c) = child {
        match c.try_wait() {
            Ok(Some(_)) => {
                // Process has exited
                *child = None;
                false
            }
            Ok(None) => true, // Still running
            Err(_) => {
                *child = None;
                false
            }
        }
    } else {
        false
    }
}

/// Kill a running game process
pub fn kill_game(child: &mut Option<Child>) {
    if let Some(ref mut c) = child {
        let _ = c.kill();
        let _ = c.wait(); // Reap the zombie
        *child = None;
    }
}

/// Copy map file to game's assets folder
///
/// Copies the map project file to `{game_project}/assets/maps/{filename}`
pub fn sync_map_to_game(
    map_path: &Path,
    game_project_path: &Path,
) -> Result<PathBuf, std::io::Error> {
    let game_assets = game_project_path.join("assets").join("maps");

    // Create the maps directory if it doesn't exist
    std::fs::create_dir_all(&game_assets)?;

    // Get the map filename
    let map_filename = map_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid map path"))?;

    let dest_path = game_assets.join(map_filename);

    // Copy the file
    std::fs::copy(map_path, &dest_path)?;

    Ok(dest_path)
}

/// Copy tileset images to game's assets folder
///
/// Copies tileset images maintaining relative path structure.
/// Uses read-then-write to work around file locks from the editor's asset system.
/// Handles both absolute and relative tileset paths.
pub fn sync_tileset_to_game(
    tileset_path: &Path,
    source_assets_dir: &Path,
    game_project_path: &Path,
) -> Result<PathBuf, std::io::Error> {
    let game_assets = game_project_path.join("assets");

    // Determine relative path for destination
    let dest_path = if let Ok(rel) = tileset_path.strip_prefix(source_assets_dir) {
        // Path is relative to source_assets_dir
        game_assets.join(rel)
    } else {
        // Path might be absolute - try to find "assets/" or "assets\" and use everything after
        let path_str = tileset_path.to_string_lossy();
        if let Some(pos) = path_str
            .find("assets/")
            .or_else(|| path_str.find("assets\\"))
        {
            let after_assets = &path_str[pos + 7..]; // Skip "assets/"
            game_assets.join(after_assets)
        } else {
            // Just use the filename as fallback
            let filename = tileset_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("unknown"));
            game_assets.join(filename)
        }
    };

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Copy if source exists
    // Use read-then-write instead of fs::copy to work around file locks
    // (Bevy's asset system keeps handles open which blocks fs::copy on Windows)
    if tileset_path.exists() {
        let contents = std::fs::read(tileset_path)?;
        std::fs::write(&dest_path, contents)?;
    }

    Ok(dest_path)
}

/// Sync all assets from the editor project to the game project
///
/// This syncs:
/// 1. The map file to `{game}/assets/maps/`
/// 2. All tileset images to `{game}/assets/` (preserving relative paths)
/// 3. All sprite sheet images to `{game}/assets/` (preserving relative paths)
///
/// Returns the path to the synced map file
pub fn sync_all_assets_to_game(
    project: &crate::project::Project,
    map_path: &Path,
    game_project_path: &Path,
    assets_base_path: &Path,
) -> Result<PathBuf, std::io::Error> {
    // 1. Sync the map file
    let map_dest = sync_map_to_game(map_path, game_project_path)?;
    bevy::log::info!("Synced map to: {}", map_dest.display());

    // 2. Sync all tileset images
    for tileset in &project.tilesets {
        // Sync images from multi-image tilesets
        for image in &tileset.images {
            let image_path = assets_base_path.join(&image.path);
            if image_path.exists() {
                match sync_tileset_to_game(&image_path, assets_base_path, game_project_path) {
                    Ok(dest) => bevy::log::info!("Synced tileset image: {}", dest.display()),
                    Err(e) => {
                        bevy::log::warn!("Failed to sync tileset image {}: {}", image.path, e)
                    }
                }
            } else {
                bevy::log::warn!("Tileset image not found: {}", image_path.display());
            }
        }

        // Sync legacy single-image path if present
        if let Some(path) = &tileset.path {
            let image_path = assets_base_path.join(path);
            if image_path.exists() {
                match sync_tileset_to_game(&image_path, assets_base_path, game_project_path) {
                    Ok(dest) => bevy::log::info!("Synced legacy tileset: {}", dest.display()),
                    Err(e) => bevy::log::warn!("Failed to sync legacy tileset {}: {}", path, e),
                }
            }
        }
    }

    // 3. Sync all sprite sheet images
    for sprite_sheet in &project.sprite_sheets {
        let sheet_path = assets_base_path.join(&sprite_sheet.sheet_path);
        if sheet_path.exists() {
            match sync_tileset_to_game(&sheet_path, assets_base_path, game_project_path) {
                Ok(dest) => bevy::log::info!("Synced sprite sheet: {}", dest.display()),
                Err(e) => {
                    bevy::log::warn!(
                        "Failed to sync sprite sheet {}: {}",
                        sprite_sheet.sheet_path,
                        e
                    )
                }
            }
        } else {
            bevy::log::warn!("Sprite sheet not found: {}", sheet_path.display());
        }
    }

    Ok(map_dest)
}

/// Handle for controlling an async build/run operation
pub struct AsyncBuildHandle {
    /// Receiver for build output messages (wrapped in Mutex for Sync)
    pub receiver: Mutex<Receiver<BuildOutput>>,
    /// Sender to signal cancellation (send any value to cancel)
    pub cancel_sender: Sender<()>,
}

impl AsyncBuildHandle {
    /// Try to receive a message from the build thread
    pub fn try_recv(&self) -> Option<BuildOutput> {
        self.receiver.lock().ok()?.try_recv().ok()
    }

    /// Send a cancellation signal to stop the build/game process
    pub fn cancel(&self) {
        let _ = self.cancel_sender.send(());
    }
}

/// Launch a game project asynchronously with progress reporting
///
/// Spawns a background thread that:
/// 1. Runs `cargo run` with piped stdout/stderr
/// 2. Parses output for progress information
/// 3. Sends BuildOutput messages through the channel
///
/// Returns a handle with a receiver for progress updates and a sender to cancel.
pub fn launch_game_async(options: LaunchOptions) -> Result<AsyncBuildHandle, LaunchError> {
    // Verify project exists before spawning thread
    if !options.project_path.exists() {
        return Err(LaunchError::ProjectNotFound(options.project_path.clone()));
    }

    if !options.project_path.join("Cargo.toml").exists() {
        return Err(LaunchError::ProjectNotFound(options.project_path.clone()));
    }

    let (output_tx, output_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpsc::channel();

    thread::spawn(move || {
        run_build_thread(options, output_tx, cancel_rx);
    });

    Ok(AsyncBuildHandle {
        receiver: Mutex::new(output_rx),
        cancel_sender: cancel_tx,
    })
}

/// Background thread function that runs cargo and parses output
fn run_build_thread(options: LaunchOptions, tx: Sender<BuildOutput>, cancel_rx: Receiver<()>) {
    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    cmd.current_dir(&options.project_path);

    if options.release {
        cmd.arg("--release");
    }

    if options.hot_reload {
        cmd.args(["--features", "hot_reload"]);
    }

    // Force cargo to show progress even when piped
    // CARGO_TERM_PROGRESS_WHEN=always forces progress display
    // CARGO_TERM_PROGRESS_WIDTH sets the width (we use 80 for parsing)
    cmd.env("CARGO_TERM_PROGRESS_WHEN", "always");
    cmd.env("CARGO_TERM_PROGRESS_WIDTH", "80");

    // Pipe stderr to capture build progress (cargo outputs to stderr)
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(BuildOutput::BuildFailed(format!(
                "Failed to spawn cargo: {}",
                e
            )));
            return;
        }
    };

    // Read stderr in a separate thread (cargo build output goes to stderr)
    // Note: Cargo uses \r for progress updates and \n for other messages
    let stderr = child.stderr.take().unwrap();
    let tx_stderr = tx.clone();
    let stderr_handle = thread::spawn(move || {
        use std::io::Read;
        let mut reader = BufReader::new(stderr);
        let mut buffer = String::new();
        let mut byte_buf = [0u8; 1];
        let mut build_complete_sent = false;

        // Read byte by byte to handle both \r and \n as line terminators
        loop {
            match reader.read(&mut byte_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let ch = byte_buf[0] as char;
                    if ch == '\r' || ch == '\n' {
                        if !buffer.is_empty() {
                            let line = buffer.clone();
                            buffer.clear();

                            // Parse progress from line
                            if let Some((current, total)) = parse_progress(&line) {
                                let _ = tx_stderr.send(BuildOutput::Progress(current, total));
                            }

                            // Parse crate name from "Compiling <crate> v<version>" lines
                            if let Some(crate_name) = parse_compiling_crate(&line) {
                                let _ = tx_stderr.send(BuildOutput::CurrentCrate(crate_name));
                            }

                            // Detect build completion: "Finished `dev` profile..." or "Finished `release` profile..."
                            // This happens BEFORE the game starts running
                            if !build_complete_sent && line.trim_start().starts_with("Finished") {
                                let _ = tx_stderr.send(BuildOutput::BuildComplete);
                                build_complete_sent = true;
                            }

                            // Detect game started: "Running `target/debug/...`" or "Running `target/release/...`"
                            if line.trim_start().starts_with("Running `") {
                                let _ = tx_stderr.send(BuildOutput::GameStarted);
                            }

                            // Send the raw line (but skip progress bar updates to reduce noise)
                            if !line.contains("Building [") {
                                let _ = tx_stderr.send(BuildOutput::Line(line));
                            }
                        }
                    } else {
                        buffer.push(ch);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Read stdout in another thread (game output goes to stdout)
    let stdout = child.stdout.take().unwrap();
    let tx_stdout = tx.clone();
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_stdout.send(BuildOutput::Line(line));
        }
    });

    // Wait for process or cancellation
    loop {
        // Check for cancellation
        if cancel_rx.try_recv().is_ok() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = tx.send(BuildOutput::BuildFailed("Build cancelled".to_string()));
            return;
        }

        // Check if process has finished
        match child.try_wait() {
            Ok(Some(status)) => {
                // Wait for output threads to finish
                let _ = stderr_handle.join();
                let _ = stdout_handle.join();

                // GameExited is sent when the process exits (game window closed)
                // BuildComplete and GameStarted are sent earlier from stderr parsing
                if status.success() {
                    let _ = tx.send(BuildOutput::GameExited(status.code()));
                } else {
                    // Could be build failure or game crash
                    let _ = tx.send(BuildOutput::BuildFailed(format!(
                        "Process exited with code: {:?}",
                        status.code()
                    )));
                }
                return;
            }
            Ok(None) => {
                // Still running, sleep briefly
                thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                let _ = tx.send(BuildOutput::BuildFailed(format!(
                    "Error waiting for process: {}",
                    e
                )));
                return;
            }
        }
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip escape sequence: ESC [ ... (letter)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (the terminator)
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parse progress from cargo output line
/// Looks for patterns like: "Building [=====>    ] 113/413: ..."
fn parse_progress(line: &str) -> Option<(u32, u32)> {
    // Strip ANSI codes first
    let clean_line = strip_ansi_codes(line);

    // Look for "Building [" pattern
    if !clean_line.contains("Building [") {
        return None;
    }

    // Find the progress numbers after "] "
    if let Some(bracket_end) = clean_line.find("] ") {
        let after_bracket = &clean_line[bracket_end + 2..];
        // Find the colon that ends the numbers
        if let Some(colon_pos) = after_bracket.find(':') {
            let numbers = &after_bracket[..colon_pos];
            // Parse "current/total"
            let parts: Vec<&str> = numbers.split('/').collect();
            if parts.len() == 2 {
                if let (Ok(current), Ok(total)) = (parts[0].trim().parse(), parts[1].trim().parse())
                {
                    return Some((current, total));
                }
            }
        }
    }

    None
}

/// Parse crate name from "Compiling <crate> v<version>" lines
fn parse_compiling_crate(line: &str) -> Option<String> {
    // Strip ANSI codes first
    let clean_line = strip_ansi_codes(line);
    let trimmed = clean_line.trim();
    if let Some(rest) = trimmed.strip_prefix("Compiling ") {
        // Find the version marker " v"
        if let Some(v_pos) = rest.find(" v") {
            return Some(rest[..v_pos].to_string());
        }
    }
    None
}
