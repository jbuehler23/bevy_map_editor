//! Camera bounds and utilities for map-based games
//!
//! This module provides camera bounds clamping to keep the camera within level bounds.
//!
//! # Usage
//!
//! Camera bounds are automatically set up when a map loads if you use `MapRuntimePlugin`.
//! You can also manually add `CameraBounds` to any camera:
//!
//! ```rust,ignore
//! use bevy_map_runtime::camera::CameraBounds;
//!
//! commands.spawn((
//!     Camera2d,
//!     CameraBounds::from_level(32, 24, 16.0), // 32x24 tiles at 16px each
//! ));
//! ```

use bevy::prelude::*;

use crate::MapRoot;
use bevy_map_core::MapProject;

/// Camera bounds configuration component
///
/// Attach this to a camera to clamp its position within the specified bounds.
/// The bounds are automatically adjusted based on the camera's visible area
/// to prevent showing areas outside the level.
#[derive(Component, Debug, Clone)]
pub struct CameraBounds {
    /// Minimum world position (bottom-left corner)
    pub min: Vec2,
    /// Maximum world position (top-right corner)
    pub max: Vec2,
    /// Optional padding from edges (positive = shrink bounds)
    pub padding: f32,
}

impl Default for CameraBounds {
    fn default() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::new(1000.0, 1000.0),
            padding: 0.0,
        }
    }
}

impl CameraBounds {
    /// Create bounds from level dimensions
    ///
    /// # Arguments
    /// * `width` - Level width in tiles
    /// * `height` - Level height in tiles
    /// * `tile_size` - Size of each tile in pixels
    pub fn from_level(width: u32, height: u32, tile_size: f32) -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::new(width as f32 * tile_size, height as f32 * tile_size),
            padding: 0.0,
        }
    }

    /// Set padding from edges
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

/// System that clamps camera position to stay within bounds
///
/// This system accounts for the camera's visible area (based on OrthographicProjection)
/// to ensure the edges of the view don't show areas outside the bounds.
pub fn clamp_camera_to_bounds(
    mut cameras: Query<(&mut Transform, &CameraBounds, &Projection), With<Camera2d>>,
) {
    for (mut transform, bounds, projection) in cameras.iter_mut() {
        // Get visible area from orthographic projection
        let Projection::Orthographic(ortho) = projection else {
            continue;
        };

        // Calculate visible area half-size from the projection
        let half_width = ortho.area.width() / 2.0;
        let half_height = ortho.area.height() / 2.0;

        // Calculate clamping bounds (account for camera viewport size)
        let min_x = bounds.min.x + half_width + bounds.padding;
        let max_x = bounds.max.x - half_width - bounds.padding;
        let min_y = bounds.min.y + half_height + bounds.padding;
        let max_y = bounds.max.y - half_height - bounds.padding;

        // Only clamp if the level is larger than the viewport
        if max_x > min_x {
            transform.translation.x = transform.translation.x.clamp(min_x, max_x);
        } else {
            // Center the camera if level is smaller than viewport
            transform.translation.x = (bounds.min.x + bounds.max.x) / 2.0;
        }

        if max_y > min_y {
            transform.translation.y = transform.translation.y.clamp(min_y, max_y);
        } else {
            // Center the camera if level is smaller than viewport
            transform.translation.y = (bounds.min.y + bounds.max.y) / 2.0;
        }
    }
}

/// System that automatically sets up camera bounds when a map loads
///
/// This system detects when a `MapRoot` component is added and configures
/// `CameraBounds` on all cameras based on the level dimensions.
pub fn setup_camera_bounds_from_map(
    mut commands: Commands,
    map_query: Query<&MapRoot, Added<MapRoot>>,
    map_assets: Res<Assets<MapProject>>,
    camera_query: Query<Entity, (With<Camera2d>, Without<CameraBounds>)>,
) {
    for map_root in map_query.iter() {
        let Some(project) = map_assets.get(&map_root.handle) else {
            continue;
        };

        let level = &project.level;
        let tile_size = map_root.textures.tile_size;

        // Add bounds to all cameras that don't have them
        for camera_entity in camera_query.iter() {
            commands
                .entity(camera_entity)
                .insert(CameraBounds::from_level(
                    level.width,
                    level.height,
                    tile_size,
                ));

            info!(
                "Set camera bounds to {}x{} pixels ({}x{} tiles)",
                level.width as f32 * tile_size,
                level.height as f32 * tile_size,
                level.width,
                level.height
            );
        }
    }
}
