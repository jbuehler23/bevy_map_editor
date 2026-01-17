//! Entity sprite plugin for spawning sprites from EntityTypeConfig
//!
//! This module provides the `MapEntitySpritePlugin` which automatically spawns
//! sprite and animation components on entities based on their type's SpriteConfig.
//!
//! # Usage
//!
//! ```rust,ignore
//! use bevy_map_runtime::{MapRuntimePlugin, MapEntitySpritePlugin};
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MapRuntimePlugin)
//!     .add_plugins(MapEntitySpritePlugin)  // Adds sprites to entities
//!     .run();
//! ```
//!
//! # How It Works
//!
//! When an entity is spawned with a `MapEntityMarker` component, this plugin:
//! 1. Looks up the entity's type in the project's `entity_type_configs`
//! 2. If a `SpriteConfig` is defined with a sprite_sheet_id, loads and spawns the sprite
//! 3. If animations are defined, auto-plays the default animation

use crate::entity_registry::MapEntityMarker;
use crate::MapRoot;
use bevy::prelude::*;
use bevy_map_animation::{AnimatedSprite, SpriteData};
use bevy_map_core::{MapProject, SpriteConfig};

/// Plugin that spawns sprite components on entities based on EntityTypeConfig
///
/// This plugin is optional and modular. Add it to your app to enable automatic
/// sprite spawning based on type-level configuration in the editor.
///
/// # Example
///
/// ```rust,ignore
/// use bevy_map_runtime::{MapRuntimePlugin, MapEntitySpritePlugin};
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(MapRuntimePlugin)
///     .add_plugins(MapEntitySpritePlugin)
///     .run();
/// ```
pub struct MapEntitySpritePlugin;

impl Plugin for MapEntitySpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_entity_sprites, complete_entity_sprite_loads).chain(),
        );
    }
}

/// Marker component indicating sprite setup has started for this entity
#[derive(Component)]
pub struct EntitySpriteSetup {
    /// The sprite sheet ID from the config
    pub sprite_sheet_id: uuid::Uuid,
    /// Default animation to play
    pub default_animation: Option<String>,
    /// Optional scale
    pub scale: Option<f32>,
    /// Flip sprite based on movement direction
    pub flip_with_direction: bool,
    /// Offset from entity position
    pub offset: [f32; 2],
}

/// Marker component indicating sprite loading is complete
#[derive(Component)]
pub struct EntitySpriteSpawned;

/// System that initiates sprite loading for newly added entities
fn spawn_entity_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    entity_query: Query<
        (Entity, &MapEntityMarker),
        (
            Added<MapEntityMarker>,
            Without<EntitySpriteSetup>,
            Without<EntitySpriteSpawned>,
        ),
    >,
    map_root_query: Query<&MapRoot>,
    map_assets: Res<Assets<MapProject>>,
) {
    // Try to get the first available map project
    let project = map_root_query
        .iter()
        .find_map(|root| map_assets.get(&root.handle));

    let Some(project) = project else {
        return;
    };

    for (entity, marker) in entity_query.iter() {
        // Look up the type config for this entity
        let Some(type_config) = project.get_entity_type_config(&marker.type_name) else {
            continue;
        };

        // Check if sprite is configured
        let Some(sprite_config) = &type_config.sprite else {
            continue;
        };

        // Need a sprite sheet ID
        let Some(sprite_sheet_id) = sprite_config.sprite_sheet_id else {
            continue;
        };

        // Find the sprite sheet in the project
        let Some(sprite_data) = project.get_sprite_sheet(sprite_sheet_id) else {
            warn!(
                "Sprite sheet {} not found for entity type '{}'",
                sprite_sheet_id, marker.type_name
            );
            continue;
        };

        // Load the texture
        let texture_path = &sprite_data.sheet_path;
        if texture_path.is_empty() {
            warn!(
                "Sprite sheet {} has no path for entity type '{}'",
                sprite_sheet_id, marker.type_name
            );
            continue;
        }

        // Normalize path for asset loading
        let asset_path = normalize_asset_path(texture_path);
        let texture_handle: Handle<Image> = asset_server.load(asset_path);

        // Calculate initial rect
        let initial_rect = calculate_initial_rect(sprite_data, sprite_config);

        // Calculate custom size based on scale
        let custom_size = sprite_config.scale.map(|s| {
            Vec2::new(
                sprite_data.frame_width as f32 * s,
                sprite_data.frame_height as f32 * s,
            )
        });

        // Add components
        commands.entity(entity).insert((
            EntitySpriteSetup {
                sprite_sheet_id,
                default_animation: sprite_config.default_animation.clone(),
                scale: sprite_config.scale,
                flip_with_direction: sprite_config.flip_with_direction,
                offset: sprite_config.offset,
            },
            Sprite {
                image: texture_handle,
                rect: initial_rect,
                custom_size,
                ..default()
            },
        ));

        info!(
            "Starting sprite load for entity '{}' (type: {}, sheet: {})",
            marker.instance_id, marker.type_name, sprite_data.name
        );
    }
}

/// System that completes sprite setup once assets are loaded
fn complete_entity_sprite_loads(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sprite_data_assets: ResMut<Assets<SpriteData>>,
    map_root_query: Query<&MapRoot>,
    map_assets: Res<Assets<MapProject>>,
    query: Query<(Entity, &EntitySpriteSetup, &Sprite), Without<EntitySpriteSpawned>>,
) {
    use bevy::asset::LoadState;

    // Try to get the first available map project
    let project = map_root_query
        .iter()
        .find_map(|root| map_assets.get(&root.handle));

    let Some(project) = project else {
        return;
    };

    for (entity, setup, sprite) in query.iter() {
        // Check if texture is loaded
        let load_state = asset_server.get_load_state(&sprite.image);
        if !matches!(load_state, Some(LoadState::Loaded)) {
            continue;
        }

        // Get sprite data
        let Some(sprite_data) = project.get_sprite_sheet(setup.sprite_sheet_id) else {
            commands.entity(entity).insert(EntitySpriteSpawned);
            continue;
        };

        // Add AnimatedSprite if there are animations
        if !sprite_data.animations.is_empty() {
            let sprite_data_handle = sprite_data_assets.add(sprite_data.clone());
            let mut animated = AnimatedSprite::new(sprite_data_handle);

            // Determine which animation to play
            let initial_anim = setup
                .default_animation
                .as_ref()
                .filter(|name| sprite_data.animations.contains_key(*name))
                .cloned()
                .or_else(|| {
                    // Fall back to "idle" or first animation
                    if sprite_data.animations.contains_key("idle") {
                        Some("idle".to_string())
                    } else {
                        sprite_data.animations.keys().next().cloned()
                    }
                });

            if let Some(ref anim_name) = initial_anim {
                animated.play(anim_name);
            }

            commands.entity(entity).insert(animated);
        }

        // Mark as complete
        commands.entity(entity).insert(EntitySpriteSpawned);
        info!(
            "Completed sprite setup for entity with sheet {}",
            sprite_data.name
        );
    }
}

/// Calculate the initial sprite rect from sprite data and config
fn calculate_initial_rect(sprite_data: &SpriteData, config: &SpriteConfig) -> Option<Rect> {
    // Try to get the default animation's first frame
    let first_frame = config
        .default_animation
        .as_ref()
        .and_then(|name| sprite_data.animations.get(name))
        .and_then(|anim| anim.frames.first().copied())
        .or_else(|| {
            // Fall back to "idle" animation
            sprite_data
                .animations
                .get("idle")
                .and_then(|anim| anim.frames.first().copied())
        })
        .or_else(|| {
            // Fall back to first animation's first frame
            sprite_data
                .animations
                .values()
                .next()
                .and_then(|anim| anim.frames.first().copied())
        })
        .unwrap_or(0);

    let columns = sprite_data.columns;
    if columns == 0 {
        return None;
    }

    let row = first_frame as u32 / columns;
    let col = first_frame as u32 % columns;

    Some(Rect::new(
        col as f32 * sprite_data.frame_width as f32,
        row as f32 * sprite_data.frame_height as f32,
        (col + 1) as f32 * sprite_data.frame_width as f32,
        (row + 1) as f32 * sprite_data.frame_height as f32,
    ))
}

/// Convert an absolute file path to a relative asset path
fn normalize_asset_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_lowercase();

    if let Some(idx) = lower.find("/assets/") {
        return normalized[idx + 8..].to_string();
    }

    if lower.starts_with("assets/") {
        return normalized[7..].to_string();
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_compiles() {
        let _plugin = MapEntitySpritePlugin;
    }

    #[test]
    fn test_normalize_asset_path() {
        assert_eq!(
            normalize_asset_path("C:/project/assets/sprites/player.png"),
            "sprites/player.png"
        );
        assert_eq!(
            normalize_asset_path("assets/sprites/player.png"),
            "sprites/player.png"
        );
        assert_eq!(
            normalize_asset_path("sprites/player.png"),
            "sprites/player.png"
        );
    }
}
