//! Standalone Bevy Map Editor binary
//!
//! Install with: cargo install bevy_map_editor
//! Run with: bevy_map_editor

use bevy::asset::{AssetPlugin, UnapprovedPathMode};
use bevy::image::{ImageFilterMode, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_map_editor::preferences::EditorPreferences;
use bevy_map_editor::project::Project;
use bevy_map_editor::EditorPlugin;
use std::path::PathBuf;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Map Editor".to_string(),
                        // High DPI support: prevent OS-level scaling that causes blurriness
                        resolution: WindowResolution::new(1920, 1080)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    // Pixel-perfect rendering: use Nearest (point) sampling for crisp pixel art
                    default_sampler: ImageSamplerDescriptor {
                        mag_filter: ImageFilterMode::Nearest,
                        min_filter: ImageFilterMode::Nearest,
                        mipmap_filter: ImageFilterMode::Nearest,
                        ..default()
                    },
                })
                .set(AssetPlugin {
                    // Allow loading assets from any path (absolute paths, outside assets folder)
                    // This is needed for a map editor where users can place assets anywhere
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(EditorPlugin::default())
        .add_systems(Startup, auto_open_last_project)
        .run();
}

/// System to auto-open the last project on startup if enabled in preferences
fn auto_open_last_project(
    mut project: ResMut<Project>,
    preferences: Res<EditorPreferences>,
) {
    if !preferences.auto_open_last_project {
        return;
    }

    if let Some(recent) = preferences.last_project() {
        let path = PathBuf::from(&recent.path);
        if path.exists() {
            match Project::load(&path) {
                Ok(loaded) => {
                    *project = loaded;
                    info!("Auto-opened last project: {}", recent.name);
                }
                Err(e) => {
                    warn!("Failed to auto-open project '{}': {}", recent.name, e);
                }
            }
        } else {
            warn!(
                "Last project file not found: {} ({})",
                recent.name, recent.path
            );
        }
    }
}
