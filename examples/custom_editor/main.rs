//! Custom Editor Example
//!
//! Demonstrates how to embed and customize the bevy_map_editor in your own Bevy application.
//!
//! For standard usage, install and run the editor directly:
//! ```bash
//! cargo install bevy_map_editor
//! bevy_map_editor
//! ```
//!
//! This example shows various customization options developers can use when embedding
//! the editor in their game or tooling pipeline.
//!
//! Run with: cargo run --example custom_editor -p bevy_map_editor_examples

use bevy::asset::AssetPlugin;
use bevy::image::{ImageFilterMode, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_map_editor::ui::EditorTool;
use bevy_map_editor::{EditorPlugin, EditorState};
use std::path::PathBuf;

fn main() {
    // Determine the assets path - for examples, this is in the examples/ directory
    let assets_path = get_assets_path();

    App::new()
        .add_plugins(
            DefaultPlugins
                // -------------------------------------------------------------------------------
                // CUSTOMIZATION 1: Window Configuration
                // -------------------------------------------------------------------------------
                // Configure window title, size, and scaling behavior.
                // Use scale_factor_override(1.0) for high DPI displays to prevent
                // OS-level scaling that can cause blurriness.
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "My Game - Custom Map Editor".to_string(),
                        // High DPI support: prevent OS-level scaling
                        resolution: WindowResolution::new(1920, 1080)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                // -------------------------------------------------------------------------------
                // CUSTOMIZATION 2: Pixel Rendering
                // -------------------------------------------------------------------------------
                // For pixel art games, use Nearest (point) filtering to keep
                // sprites crisp when zooming. This prevents blurry sprites.
                // Similar to Godot's "Nearest" texture filter setting.
                .set(ImagePlugin {
                    default_sampler: ImageSamplerDescriptor {
                        mag_filter: ImageFilterMode::Nearest,
                        min_filter: ImageFilterMode::Nearest,
                        mipmap_filter: ImageFilterMode::Nearest,
                        ..default()
                    },
                })
                // -------------------------------------------------------------------------------
                // CUSTOMIZATION 3: Custom Assets Path
                // -------------------------------------------------------------------------------
                // Point to your game's assets directory
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().to_string(),
                    ..default()
                }),
        )
        // -------------------------------------------------------------------------------════════
        // CUSTOMIZATION 4: Editor Plugin Configuration
        // -------------------------------------------------------------------------------════════
        // Configure the editor's initial state using builder methods.
        // All of these have sensible defaults, so only override what you need.
        .add_plugins(
            EditorPlugin::new()
                // Asset path for map file management (must match AssetPlugin)
                .with_assets_path(&assets_path)
                // Initial viewport settings
                .with_initial_grid(true) // Show grid overlay (default: true)
                .with_show_collisions(false) // Show collision shapes (default: false)
                .with_snap_to_grid(true) // Snap entities to grid (default: true)
                .with_initial_zoom(1.0) // Zoom level 0.25 to 4.0 (default: 1.0)
                // Initial tool selection
                .with_initial_tool(EditorTool::Select), // Starting tool (default: Select)
        )
        // -------------------------------------------------------------------------------════════
        // CUSTOMIZATION 5: Runtime State Modifications
        // -------------------------------------------------------------------------------════════
        // For advanced customization, you can modify EditorState directly at
        // startup or during runtime via systems.
        .add_systems(Startup, customize_editor_on_startup)
        .run();
}

/// Example: Modify editor state after startup
///
/// This demonstrates runtime customization of the editor state.
/// You can use this pattern to:
/// - Set tool based on user preferences
/// - Load saved editor settings
/// - Respond to game state changes
#[allow(unused_mut)] // mut shown for educational purposes - uncomment lines below to use
fn customize_editor_on_startup(mut editor_state: ResMut<EditorState>) {
    // Example: Start with Paint tool and collision view enabled for level designers
    // Uncomment to enable:
    // editor_state.current_tool = EditorTool::Paint;
    // editor_state.show_collisions = true;
    // editor_state.zoom = 2.0;

    // Log that custom startup completed
    bevy::log::info!(
        "Custom editor initialized - Grid: {}, Collisions: {}, Zoom: {}",
        editor_state.show_grid,
        editor_state.show_collisions,
        editor_state.zoom
    );
}

/// Determine the correct assets path for this example
fn get_assets_path() -> PathBuf {
    // First, check if we're running from the examples directory
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(&manifest_dir);
        let assets_in_manifest = manifest_path.join("assets");
        if assets_in_manifest.exists() {
            return assets_in_manifest;
        }
    }

    // Check for examples/assets from workspace root
    if let Ok(cwd) = std::env::current_dir() {
        let examples_assets = cwd.join("examples").join("assets");
        if examples_assets.exists() {
            return examples_assets;
        }

        // Fallback to regular assets folder
        let assets = cwd.join("assets");
        if assets.exists() {
            return assets;
        }
    }

    // Last resort
    PathBuf::from("assets")
}
