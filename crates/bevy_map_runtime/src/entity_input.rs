//! Entity input plugin for spawning input components from EntityTypeConfig
//!
//! This module provides the `MapEntityInputPlugin` which automatically spawns
//! input handling components on entities based on their type's InputConfig.
//!
//! # Usage
//!
//! ```rust,ignore
//! use bevy_map_runtime::{MapRuntimePlugin, MapEntityInputPlugin};
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MapRuntimePlugin)
//!     .add_plugins(MapEntityInputPlugin)  // Adds input to entities
//!     .run();
//! ```
//!
//! # Built-in Input Profiles
//!
//! - **Platformer**: A/D for horizontal movement, Space to jump
//! - **TopDown**: WASD for 8-directional movement
//! - **TwinStick**: WASD for movement, mouse for aiming (component only, no built-in system)
//!
//! # Custom Profiles
//!
//! For custom input profiles, the plugin adds a marker component. You can then
//! implement your own input systems that query for that component.

use crate::entity_registry::MapEntityMarker;
use crate::MapRoot;
use bevy::prelude::*;
use bevy_map_core::{InputConfig, InputProfile, MapProject};

/// Plugin that spawns input components on entities based on EntityTypeConfig
///
/// This plugin is optional and modular. Add it to your app to enable automatic
/// input handling based on type-level configuration in the editor.
///
/// # Example
///
/// ```rust,ignore
/// use bevy_map_runtime::{MapRuntimePlugin, MapEntityInputPlugin};
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(MapRuntimePlugin)
///     .add_plugins(MapEntityInputPlugin)
///     .run();
/// ```
pub struct MapEntityInputPlugin;

impl Plugin for MapEntityInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_entity_input)
            // Only add built-in input systems when physics feature is enabled
            // (input requires velocity components from physics)
            .add_systems(
                Update,
                (platformer_input_system, top_down_input_system).run_if(
                    any_with_component::<PlatformerInput>.or(any_with_component::<TopDownInput>),
                ),
            );
    }
}

// ============================================================================
// Input marker components
// ============================================================================

/// Marker component for entities with platformer input
///
/// Entities with this component will respond to:
/// - A/D for horizontal movement
/// - Space for jumping (requires physics)
#[derive(Component, Debug, Clone)]
pub struct PlatformerInput {
    /// Movement speed in pixels per second
    pub speed: f32,
    /// Jump force (vertical impulse)
    pub jump_force: f32,
    /// Maximum fall speed (terminal velocity)
    pub max_fall_speed: f32,
    /// Is the entity currently grounded?
    pub grounded: bool,
}

impl Default for PlatformerInput {
    fn default() -> Self {
        Self {
            speed: 200.0,
            jump_force: 400.0,
            max_fall_speed: 600.0,
            grounded: false,
        }
    }
}

/// Marker component for entities with top-down input
///
/// Entities with this component will respond to:
/// - WASD for 8-directional movement
#[derive(Component, Debug, Clone)]
pub struct TopDownInput {
    /// Movement speed in pixels per second
    pub speed: f32,
}

impl Default for TopDownInput {
    fn default() -> Self {
        Self { speed: 200.0 }
    }
}

/// Marker component for entities with twin-stick input
///
/// This is a marker only - you need to implement your own
/// system to handle aiming with mouse/right stick.
#[derive(Component, Debug, Clone)]
pub struct TwinStickInput {
    /// Movement speed in pixels per second
    pub speed: f32,
}

impl Default for TwinStickInput {
    fn default() -> Self {
        Self { speed: 200.0 }
    }
}

/// Marker component for entities with custom input profiles
#[derive(Component, Debug, Clone)]
pub struct CustomInput {
    /// Name of the custom profile
    pub profile_name: String,
    /// Movement speed in pixels per second
    pub speed: f32,
}

/// Marker component indicating input has been spawned for this entity
#[derive(Component)]
pub struct EntityInputSpawned;

// ============================================================================
// Input spawning system
// ============================================================================

/// System that spawns input components on newly added entities
fn spawn_entity_input(
    mut commands: Commands,
    entity_query: Query<
        (Entity, &MapEntityMarker),
        (Added<MapEntityMarker>, Without<EntityInputSpawned>),
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
            commands.entity(entity).insert(EntityInputSpawned);
            continue;
        };

        // Check if input is configured
        let Some(input) = &type_config.input else {
            commands.entity(entity).insert(EntityInputSpawned);
            continue;
        };

        // Spawn input component based on profile
        spawn_input_component(&mut commands, entity, input);

        // Mark as processed
        commands.entity(entity).insert(EntityInputSpawned);

        info!(
            "Spawned input for entity '{}' (type: {}, profile: {})",
            marker.instance_id,
            marker.type_name,
            input.profile.display_name()
        );
    }
}

/// Spawn the appropriate input component based on profile
fn spawn_input_component(commands: &mut Commands, entity: Entity, config: &InputConfig) {
    match &config.profile {
        InputProfile::Platformer => {
            commands.entity(entity).insert(PlatformerInput {
                speed: config.speed,
                jump_force: config.jump_force.unwrap_or(400.0),
                max_fall_speed: config.max_fall_speed.unwrap_or(600.0),
                grounded: false,
            });
        }
        InputProfile::TopDown => {
            commands.entity(entity).insert(TopDownInput {
                speed: config.speed,
            });
        }
        InputProfile::TwinStick => {
            commands.entity(entity).insert(TwinStickInput {
                speed: config.speed,
            });
        }
        InputProfile::Custom { name } => {
            commands.entity(entity).insert(CustomInput {
                profile_name: name.clone(),
                speed: config.speed,
            });
        }
        InputProfile::None => {
            // No input component needed
        }
    }
}

// ============================================================================
// Built-in input systems (require physics feature)
// ============================================================================

/// Platformer input system: A/D for movement, Space for jump
#[cfg(feature = "physics")]
fn platformer_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&PlatformerInput, &mut avian2d::prelude::LinearVelocity)>,
) {
    for (input, mut velocity) in query.iter_mut() {
        // Horizontal movement
        let mut direction = 0.0;
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction += 1.0;
        }

        velocity.x = direction * input.speed;

        // Jumping - only when grounded (simplified, would need ground detection)
        if keyboard.just_pressed(KeyCode::Space) {
            velocity.y = input.jump_force;
        }

        // Clamp fall speed
        if velocity.y < -input.max_fall_speed {
            velocity.y = -input.max_fall_speed;
        }
    }
}

#[cfg(not(feature = "physics"))]
fn platformer_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&PlatformerInput, &mut Transform)>,
    time: Res<Time>,
) {
    for (input, mut transform) in query.iter_mut() {
        let mut direction = Vec2::ZERO;
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        transform.translation.x += direction.x * input.speed * time.delta_secs();
    }
}

/// Top-down input system: WASD for 8-directional movement
#[cfg(feature = "physics")]
fn top_down_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&TopDownInput, &mut avian2d::prelude::LinearVelocity)>,
) {
    for (input, mut velocity) in query.iter_mut() {
        let mut direction = Vec2::ZERO;

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        let movement = direction.normalize_or_zero() * input.speed;
        velocity.x = movement.x;
        velocity.y = movement.y;
    }
}

#[cfg(not(feature = "physics"))]
fn top_down_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&TopDownInput, &mut Transform)>,
    time: Res<Time>,
) {
    for (input, mut transform) in query.iter_mut() {
        let mut direction = Vec2::ZERO;

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        let movement = direction.normalize_or_zero() * input.speed * time.delta_secs();
        transform.translation.x += movement.x;
        transform.translation.y += movement.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_compiles() {
        let _plugin = MapEntityInputPlugin;
    }

    #[test]
    fn test_input_components() {
        let platformer = PlatformerInput::default();
        assert_eq!(platformer.speed, 200.0);

        let top_down = TopDownInput::default();
        assert_eq!(top_down.speed, 200.0);
    }
}
