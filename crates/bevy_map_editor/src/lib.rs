//! bevy_map_editor - Full-featured map editor for Bevy games
//!
//! This crate provides a complete tilemap editor with:
//! - Project management (save/load)
//! - Level editing with multiple layers
//! - Tileset management with multi-image support
//! - Entity placement
//! - Terrain/autotile painting (Tiled-compatible)
//! - Undo/redo support
//! - Copy/paste/delete
//!
//! # Usage
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map_editor::EditorPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EditorPlugin)
//!         .run();
//! }
//! ```

pub mod commands;
pub mod preferences;
pub mod project;
pub mod render;
pub mod tools;
pub mod ui;

// Re-export core types from bevy_map_* crates
pub use bevy_map_autotile;
pub use bevy_map_core;
pub use bevy_map_schema;

#[cfg(feature = "runtime")]
pub use bevy_map_runtime;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use std::path::PathBuf;

use commands::clipboard::TileSelection;
use commands::{handle_keyboard_shortcuts, CommandHistory, TileClipboard};
use project::Project;
use render::MapRenderPlugin;
use tools::EditorToolsPlugin;
use ui::{
    AnimationEditorState, DialogueEditorState, EditorTool, EditorUiPlugin, EntityPaintState,
    PendingAction, SchemaEditorState, Selection, SpriteSheetEditorState, TerrainPaintState,
    TilesetEditorState, ToolMode,
};

/// Error types for asset path handling
#[derive(Debug, Clone, PartialEq)]
pub enum PathError {
    /// File does not exist at the specified path
    FileNotFound(String),
    /// File is outside the assets directory
    OutsideAssetsDirectory(PathBuf),
    /// Failed to copy file
    CopyFailed(String),
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::FileNotFound(path) => write!(f, "File not found: {}", path),
            PathError::OutsideAssetsDirectory(path) => {
                write!(f, "File is outside assets directory: {}", path.display())
            }
            PathError::CopyFailed(msg) => write!(f, "Failed to copy file: {}", msg),
        }
    }
}

/// Resource storing the base assets path for converting absolute paths to relative
#[derive(Resource, Default)]
pub struct AssetsBasePath(pub PathBuf);

impl AssetsBasePath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Get the assets directory path
    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    /// Convert an absolute path to a path relative to the assets folder.
    /// Returns the relative path if the absolute path is within the assets folder,
    /// otherwise returns the original path (kept for backward compatibility).
    pub fn to_relative(&self, absolute_path: &std::path::Path) -> PathBuf {
        match self.to_relative_checked(absolute_path) {
            Ok(path) => path,
            Err(_) => {
                // Fallback: return original path (will likely fail to load)
                absolute_path.to_path_buf()
            }
        }
    }

    /// Convert an absolute path to a path relative to the assets folder.
    /// Returns an error if the file doesn't exist or is outside the assets directory.
    pub fn to_relative_checked(
        &self,
        absolute_path: &std::path::Path,
    ) -> Result<PathBuf, PathError> {
        // Normalize paths for comparison (handle Windows path quirks)
        let assets_path = self.0.canonicalize().unwrap_or_else(|_| self.0.clone());
        let file_path = absolute_path
            .canonicalize()
            .map_err(|_| PathError::FileNotFound(absolute_path.to_string_lossy().to_string()))?;

        // Try to strip the assets prefix
        if let Ok(relative) = file_path.strip_prefix(&assets_path) {
            // Convert backslashes to forward slashes for Bevy
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            Ok(PathBuf::from(relative_str))
        } else {
            Err(PathError::OutsideAssetsDirectory(
                absolute_path.to_path_buf(),
            ))
        }
    }

    /// Check if a path is inside the assets directory
    pub fn is_inside_assets(&self, absolute_path: &std::path::Path) -> bool {
        self.to_relative_checked(absolute_path).is_ok()
    }

    /// Copy a file from outside assets directory into assets/tiles/
    /// Returns the new relative path on success.
    pub fn copy_to_assets(&self, source_path: &std::path::Path) -> Result<PathBuf, PathError> {
        // Get the filename
        let filename = source_path
            .file_name()
            .ok_or_else(|| PathError::CopyFailed("Invalid filename".to_string()))?;

        // Create destination path: assets/tiles/{filename}
        let tiles_dir = self.0.join("tiles");
        let dest_path = tiles_dir.join(filename);

        // Create tiles directory if it doesn't exist
        std::fs::create_dir_all(&tiles_dir).map_err(|e| {
            PathError::CopyFailed(format!("Failed to create tiles directory: {}", e))
        })?;

        // Check if file already exists at destination
        if dest_path.exists() {
            // File already exists, check if it's the same file
            let source_canon = source_path.canonicalize().ok();
            let dest_canon = dest_path.canonicalize().ok();
            if source_canon == dest_canon {
                // Same file, just return the relative path
                return Ok(PathBuf::from(format!(
                    "tiles/{}",
                    filename.to_string_lossy()
                )));
            }
            // Different file exists - add a suffix to avoid overwriting
            let stem = source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let ext = source_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let unique_name = format!("{}_{}.{}", stem, uuid::Uuid::new_v4().simple(), ext);
            let dest_path = tiles_dir.join(&unique_name);

            std::fs::copy(source_path, &dest_path)
                .map_err(|e| PathError::CopyFailed(format!("Failed to copy file: {}", e)))?;

            return Ok(PathBuf::from(format!("tiles/{}", unique_name)));
        }

        // Copy the file
        std::fs::copy(source_path, &dest_path)
            .map_err(|e| PathError::CopyFailed(format!("Failed to copy file: {}", e)))?;

        // Return relative path with forward slashes
        Ok(PathBuf::from(format!(
            "tiles/{}",
            filename.to_string_lossy()
        )))
    }
}

/// Convert a path to a format Bevy's AssetServer can load.
/// - Absolute paths are converted to forward slashes
/// - Relative paths are returned as-is with forward slashes
pub fn to_asset_path(path: &str) -> String {
    // Normalize backslashes to forward slashes
    let normalized = path.replace('\\', "/");

    // Check if it's an absolute Windows path (e.g., C:/...)
    if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
        // Already in correct format for absolute path
        normalized
    } else {
        normalized
    }
}

/// Check if a path is absolute (works for both Windows and Unix paths)
pub fn is_absolute_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    path.is_absolute()
}

/// Callback type for file copy confirmation
#[derive(Debug, Clone, Default, PartialEq)]
pub enum CopyFileCallback {
    #[default]
    None,
    /// Copy file for new tileset creation
    NewTileset,
    /// Copy file for adding image to existing tileset
    AddTilesetImage,
}

/// Preview state for terrain painting
/// Shows which tiles will be affected and what they will become
#[derive(Default)]
pub struct TerrainPreviewState {
    /// Preview tiles: (position, tile_id) - what tiles would be placed
    pub preview_tiles: Vec<((i32, i32), u32)>,
    /// Whether preview is currently active
    pub active: bool,
    /// Tileset ID for rendering the preview tiles
    pub tileset_id: Option<uuid::Uuid>,
}

/// Preview state for brush (tile) painting
/// Shows the selected tile at cursor position before placing
#[derive(Default)]
pub struct BrushPreviewState {
    /// Current tile position in tile coordinates
    pub position: Option<(i32, i32)>,
    /// Whether preview is currently active
    pub active: bool,
}

/// Item currently being renamed inline
#[derive(Clone, Debug, PartialEq)]
pub enum RenamingItem {
    /// Renaming a data instance
    DataInstance(uuid::Uuid),
    /// Renaming a level entity (level_id, entity_id)
    Entity(uuid::Uuid, uuid::Uuid),
    /// Renaming a level
    Level(uuid::Uuid),
    /// Renaming a layer (level_id, layer_index)
    Layer(uuid::Uuid, usize),
    /// Renaming a tileset
    Tileset(uuid::Uuid),
    /// Renaming a sprite sheet
    SpriteSheet(uuid::Uuid),
    /// Renaming a dialogue (uses String ID)
    Dialogue(String),
}

/// Configuration for initial editor state
///
/// Use this to customize the editor's initial settings when embedding it in your application.
#[derive(Clone, Debug)]
pub struct EditorStateConfig {
    /// Whether to show the grid on startup. Default: true
    pub show_grid: bool,
    /// Whether to show collision overlays on startup. Default: false
    pub show_collisions: bool,
    /// Whether to snap to grid on startup. Default: true
    pub snap_to_grid: bool,
    /// Initial zoom level (0.25 to 4.0). Default: 1.0
    pub initial_zoom: f32,
    /// Initial tool selection. Default: Select
    pub initial_tool: EditorTool,
}

impl Default for EditorStateConfig {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_collisions: false,
            snap_to_grid: true,
            initial_zoom: 1.0,
            initial_tool: EditorTool::Select,
        }
    }
}

/// Main editor plugin with configurable assets path and initial state
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use bevy_map_editor::{EditorPlugin, EditorTool};
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(
///         EditorPlugin::new()
///             .with_assets_path("my_assets")
///             .with_initial_grid(true)
///             .with_initial_zoom(2.0)
///             .with_initial_tool(EditorTool::Paint)
///     )
///     .run();
/// ```
pub struct EditorPlugin {
    /// Custom assets path. If None, auto-detects based on environment.
    pub assets_path: Option<PathBuf>,
    /// Initial editor state configuration.
    pub initial_state: EditorStateConfig,
}

impl Default for EditorPlugin {
    fn default() -> Self {
        Self {
            assets_path: None,
            initial_state: EditorStateConfig::default(),
        }
    }
}

impl EditorPlugin {
    /// Create an editor plugin with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the assets directory path
    /// This should match where Bevy's AssetServer looks for files.
    pub fn with_assets_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.assets_path = Some(path.into());
        self
    }

    /// Set whether to show the grid on startup (default: true)
    pub fn with_initial_grid(mut self, show: bool) -> Self {
        self.initial_state.show_grid = show;
        self
    }

    /// Set whether to show collision overlays on startup (default: false)
    pub fn with_show_collisions(mut self, show: bool) -> Self {
        self.initial_state.show_collisions = show;
        self
    }

    /// Set whether to snap to grid on startup (default: true)
    pub fn with_snap_to_grid(mut self, snap: bool) -> Self {
        self.initial_state.snap_to_grid = snap;
        self
    }

    /// Set the initial zoom level (0.25 to 4.0, default: 1.0)
    pub fn with_initial_zoom(mut self, zoom: f32) -> Self {
        self.initial_state.initial_zoom = zoom.clamp(0.25, 4.0);
        self
    }

    /// Set the initial tool selection (default: Select)
    pub fn with_initial_tool(mut self, tool: EditorTool) -> Self {
        self.initial_state.initial_tool = tool;
        self
    }

    /// Auto-detect the assets path.
    /// Uses custom path if set via `with_assets_path()`, otherwise defaults to
    /// current working directory + "assets".
    fn detect_assets_path(&self) -> PathBuf {
        // Use custom path if provided
        if let Some(path) = &self.assets_path {
            return path.clone();
        }

        // Default to current directory + assets
        std::env::current_dir()
            .map(|p| p.join("assets"))
            .unwrap_or_else(|_| PathBuf::from("assets"))
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        let assets_path = self.detect_assets_path();

        // Log the assets path for debugging
        bevy::log::info!("EditorPlugin: Using assets path: {:?}", assets_path);

        // Load user preferences
        let preferences = preferences::EditorPreferences::load();
        bevy::log::info!("Loaded editor preferences");

        // Create EditorState with preferences and initial configuration
        // Plugin config takes precedence over saved preferences
        let mut editor_state = EditorState::default();
        editor_state.show_grid = self.initial_state.show_grid;
        editor_state.show_collisions = self.initial_state.show_collisions;
        editor_state.snap_to_grid = self.initial_state.snap_to_grid;
        editor_state.zoom = self.initial_state.initial_zoom;
        editor_state.current_tool = self.initial_state.initial_tool;

        app.add_plugins(EguiPlugin::default())
            .add_plugins(EditorUiPlugin)
            .add_plugins(MapRenderPlugin)
            .add_plugins(EditorToolsPlugin)
            .insert_resource(editor_state)
            .insert_resource(preferences)
            .init_resource::<CommandHistory>()
            .init_resource::<TileClipboard>()
            .insert_resource(Project::default())
            .insert_resource(AssetsBasePath::new(assets_path))
            .add_systems(Startup, setup_editor_camera)
            .add_systems(Update, handle_keyboard_shortcuts)
            .add_systems(Update, handle_recent_projects);
    }
}

/// System to handle recent projects updates
fn handle_recent_projects(
    mut editor_state: ResMut<EditorState>,
    mut preferences: ResMut<preferences::EditorPreferences>,
    mut project: ResMut<Project>,
) {
    // Add project to recent list
    if let Some(path) = editor_state.pending_add_recent_project.take() {
        let name = project.name().to_string();
        preferences.add_recent_project(path, name);
        if let Err(e) = preferences.save() {
            bevy::log::error!("Failed to save preferences: {}", e);
        }
    }

    // Open recent project
    if let Some(path) = editor_state.pending_open_recent_project.take() {
        match Project::load(&path) {
            Ok(loaded) => {
                *project = loaded;
                let name = project.name().to_string();
                preferences.add_recent_project(path, name);
                if let Err(e) = preferences.save() {
                    bevy::log::error!("Failed to save preferences: {}", e);
                }
            }
            Err(e) => {
                editor_state.error_message = Some(format!("Failed to load project: {}", e));
                // Remove from recent if file not found
                preferences.remove_recent_project(&path.to_string_lossy());
                if let Err(e) = preferences.save() {
                    bevy::log::error!("Failed to save preferences: {}", e);
                }
            }
        }
    }

    // Clear recent projects
    if editor_state.pending_clear_recent_projects {
        editor_state.pending_clear_recent_projects = false;
        preferences.clear_recent_projects();
        if let Err(e) = preferences.save() {
            bevy::log::error!("Failed to save preferences: {}", e);
        }
    }
}

/// Spawns the editor camera if one doesn't exist
fn setup_editor_camera(mut commands: Commands, camera_query: Query<&Camera2d>) {
    // Only spawn if no Camera2d exists
    if camera_query.is_empty() {
        commands.spawn(Camera2d);
    }
}

/// Global editor state
#[derive(Resource)]
pub struct EditorState {
    // Selection
    pub selection: Selection,
    pub selected_layer: Option<usize>,
    pub selected_tileset: Option<uuid::Uuid>,
    pub selected_tile: Option<u32>,
    pub selected_level: Option<uuid::Uuid>,

    // Tools
    pub current_tool: EditorTool,
    pub tool_mode: ToolMode,
    pub show_grid: bool,
    pub show_collisions: bool,
    pub snap_to_grid: bool,
    pub zoom: f32,
    pub camera_offset: bevy::math::Vec2,

    // Dialogs
    pub show_new_project_dialog: bool,
    pub show_new_level_dialog: bool,
    pub show_new_tileset_dialog: bool,
    pub show_about_dialog: bool,
    pub show_schema_editor: bool,
    pub schema_editor_state: SchemaEditorState,
    pub error_message: Option<String>,

    // New project dialog state
    pub new_project_name: String,
    pub new_project_schema_path: Option<PathBuf>,
    pub new_project_save_path: Option<PathBuf>,

    // Settings dialog
    pub show_settings_dialog: bool,

    // Recent projects handling
    pub pending_add_recent_project: Option<PathBuf>,
    pub pending_open_recent_project: Option<PathBuf>,
    pub pending_clear_recent_projects: bool,

    // New level dialog state
    pub new_level_name: String,
    pub new_level_width: u32,
    pub new_level_height: u32,

    // New tileset dialog state
    pub new_tileset_name: String,
    pub new_tileset_path: String,
    pub new_tileset_tile_size: u32,

    // Add image to tileset dialog state
    pub show_add_tileset_image_dialog: bool,
    pub add_image_name: String,
    pub add_image_path: String,

    // Pending actions
    pub pending_action: Option<PendingAction>,
    pub create_new_instance: Option<String>,

    // Tile painting
    pub is_painting: bool,
    pub last_painted_tile: Option<(u32, u32)>,

    // Autotile / Terrain (Legacy 47-tile blob)
    pub selected_terrain: Option<uuid::Uuid>,
    pub show_new_terrain_dialog: bool,
    pub new_terrain_name: String,
    pub new_terrain_first_tile: u32,

    // Tiled-Style Terrain System
    pub selected_terrain_set: Option<uuid::Uuid>,
    pub selected_terrain_in_set: Option<usize>,
    pub show_new_terrain_set_dialog: bool,
    pub new_terrain_set_type: bevy_map_autotile::TerrainSetType,
    pub show_add_terrain_to_set_dialog: bool,
    pub new_terrain_color: [f32; 3],

    // Tileset & Terrain Editor
    pub show_tileset_editor: bool,
    pub tileset_editor_state: TilesetEditorState,

    // SpriteSheet Editor (for spritesheet setup: image loading, grid config)
    pub show_spritesheet_editor: bool,
    pub spritesheet_editor_state: SpriteSheetEditorState,

    // Animation Editor (for animation definition: frames, timing, triggers, windows)
    pub show_animation_editor: bool,
    pub animation_editor_state: AnimationEditorState,

    // Dialogue Editor
    pub show_dialogue_editor: bool,
    pub dialogue_editor_state: DialogueEditorState,
    /// ID of dialogue asset being edited (vs inline property)
    pub dialogue_editor_asset_id: Option<String>,

    // Terrain painting palette
    pub terrain_paint_state: TerrainPaintState,

    // Entity placement
    pub entity_paint_state: EntityPaintState,
    pub selected_entity_type: Option<String>,

    // Tile selection (for copy/paste/delete)
    pub tile_selection: TileSelection,

    // Clipboard/paste state
    pub is_pasting: bool,
    pub pending_delete_selection: bool,

    // File copy confirmation dialog
    pub show_copy_file_dialog: bool,
    pub pending_copy_source: Option<PathBuf>,
    pub pending_copy_callback: CopyFileCallback,

    // Terrain painting preview
    pub terrain_preview: TerrainPreviewState,

    // Brush painting preview (for regular tiles)
    pub brush_preview: BrushPreviewState,

    // Inline rename state
    /// Item currently being renamed (None when not in rename mode)
    pub renaming_item: Option<RenamingItem>,
    /// Buffer for the rename text input
    pub rename_buffer: String,

    // Move operation state
    /// Whether currently dragging to move something
    pub is_moving: bool,
    /// Starting world position of drag
    pub move_drag_start: Option<bevy::math::Vec2>,
    /// Entity's original position before drag (for undo/cancel)
    pub entity_original_position: Option<[f32; 2]>,
    /// Original tiles being moved: (x, y) -> (layer_idx, tile_index)
    pub tile_move_original: Option<std::collections::HashMap<(u32, u32), (usize, Option<u32>)>>,
    /// Current drag offset in tile coordinates
    pub tile_move_offset: Option<(i32, i32)>,
    /// Flag to cancel move operation (set by Escape key, processed by tools system)
    pub pending_cancel_move: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selection: Selection::None,
            selected_layer: None,
            selected_tileset: None,
            selected_tile: None,
            selected_level: None,

            current_tool: EditorTool::Select,
            tool_mode: ToolMode::Point,
            show_grid: true,
            show_collisions: false,
            snap_to_grid: true,
            zoom: 1.0,
            camera_offset: bevy::math::Vec2::ZERO,

            show_new_project_dialog: false,
            show_new_level_dialog: false,
            show_new_tileset_dialog: false,
            show_about_dialog: false,
            show_schema_editor: false,
            schema_editor_state: SchemaEditorState::default(),
            error_message: None,

            new_project_name: String::new(),
            new_project_schema_path: None,
            new_project_save_path: None,

            show_settings_dialog: false,

            pending_add_recent_project: None,
            pending_open_recent_project: None,
            pending_clear_recent_projects: false,

            new_level_name: "New Level".to_string(),
            new_level_width: 50,
            new_level_height: 50,

            new_tileset_name: "New Tileset".to_string(),
            new_tileset_path: String::new(),
            new_tileset_tile_size: 32,

            show_add_tileset_image_dialog: false,
            add_image_name: String::new(),
            add_image_path: String::new(),

            pending_action: None,
            create_new_instance: None,

            is_painting: false,
            last_painted_tile: None,

            selected_terrain: None,
            show_new_terrain_dialog: false,
            new_terrain_name: String::new(),
            new_terrain_first_tile: 0,

            selected_terrain_set: None,
            selected_terrain_in_set: None,
            show_new_terrain_set_dialog: false,
            new_terrain_set_type: bevy_map_autotile::TerrainSetType::Corner,
            show_add_terrain_to_set_dialog: false,
            new_terrain_color: [0.0, 1.0, 0.0], // Default: green

            show_tileset_editor: false,
            tileset_editor_state: TilesetEditorState::default(),

            show_spritesheet_editor: false,
            spritesheet_editor_state: SpriteSheetEditorState::new(),

            show_animation_editor: false,
            animation_editor_state: AnimationEditorState::new(),

            show_dialogue_editor: false,
            dialogue_editor_state: DialogueEditorState::new(),
            dialogue_editor_asset_id: None,

            terrain_paint_state: TerrainPaintState::new(),

            entity_paint_state: EntityPaintState::new(),
            selected_entity_type: None,

            tile_selection: TileSelection::default(),
            is_pasting: false,
            pending_delete_selection: false,

            show_copy_file_dialog: false,
            pending_copy_source: None,
            pending_copy_callback: CopyFileCallback::None,

            terrain_preview: TerrainPreviewState::default(),
            brush_preview: BrushPreviewState::default(),

            renaming_item: None,
            rename_buffer: String::new(),

            is_moving: false,
            move_drag_start: None,
            entity_original_position: None,
            tile_move_original: None,
            tile_move_offset: None,
            pending_cancel_move: false,
        }
    }
}
