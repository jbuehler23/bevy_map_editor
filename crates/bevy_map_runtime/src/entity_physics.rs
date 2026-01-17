//! Entity physics plugin for spawning physics components from EntityTypeConfig
//!
//! This module provides the `MapEntityPhysicsPlugin` which automatically spawns
//! Avian2D physics components on entities based on their type's PhysicsConfig.
//!
//! # Usage
//!
//! ```rust,ignore
//! use bevy_map_runtime::{MapRuntimePlugin, MapEntityPhysicsPlugin};
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MapRuntimePlugin)
//!     .add_plugins(MapEntityPhysicsPlugin)  // Adds physics to entities
//!     .run();
//! ```
//!
//! # How It Works
//!
//! When an entity is spawned with a `MapEntityMarker` component, this plugin:
//! 1. Looks up the entity's type in the project's `entity_type_configs`
//! 2. If a `PhysicsConfig` is defined, spawns the appropriate physics components
//!
//! This is separate from `MapCollisionPlugin` which handles tile colliders.

use bevy::prelude::*;

#[cfg(feature = "physics")]
use avian2d::prelude::*;

use crate::entity_registry::MapEntityMarker;
use crate::MapRoot;

#[cfg(feature = "physics")]
use bevy_map_core::{ColliderConfig, MapProject, PhysicsBodyType, PhysicsConfig};

/// Plugin that spawns physics components on entities based on EntityTypeConfig
///
/// This plugin is optional and modular. Add it to your app to enable automatic
/// physics spawning based on type-level configuration in the editor.
///
/// # Example
///
/// ```rust,ignore
/// use bevy_map_runtime::{MapRuntimePlugin, MapEntityPhysicsPlugin};
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(MapRuntimePlugin)
///     .add_plugins(MapEntityPhysicsPlugin)
///     .run();
/// ```
pub struct MapEntityPhysicsPlugin;

#[cfg(feature = "physics")]
impl Plugin for MapEntityPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_entity_physics.run_if(any_with_component::<MapEntityMarker>),
        );
    }
}

#[cfg(not(feature = "physics"))]
impl Plugin for MapEntityPhysicsPlugin {
    fn build(&self, _app: &mut App) {
        // No-op when physics feature is disabled
        bevy::log::warn!(
            "MapEntityPhysicsPlugin requires the 'physics' feature. Enable it with: \
            bevy_map_runtime = {{ features = [\"physics\"] }}"
        );
    }
}

/// Marker component indicating physics has been spawned for this entity
#[derive(Component)]
pub struct EntityPhysicsSpawned;

/// System that spawns physics components on newly added entities
#[cfg(feature = "physics")]
fn spawn_entity_physics(
    mut commands: Commands,
    // Query for newly added entities that don't have physics yet
    entity_query: Query<
        (Entity, &MapEntityMarker),
        (Added<MapEntityMarker>, Without<EntityPhysicsSpawned>),
    >,
    // Need to find the map project to get entity type configs
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
            // No type config, mark as processed and continue
            commands.entity(entity).insert(EntityPhysicsSpawned);
            continue;
        };

        // Check if physics is configured
        let Some(physics) = &type_config.physics else {
            commands.entity(entity).insert(EntityPhysicsSpawned);
            continue;
        };

        // Spawn physics components
        spawn_physics_components(&mut commands, entity, physics);

        // Mark as processed
        commands.entity(entity).insert(EntityPhysicsSpawned);

        info!(
            "Spawned physics for entity '{}' (type: {})",
            marker.instance_id, marker.type_name
        );
    }
}

/// Spawn physics components on an entity
#[cfg(feature = "physics")]
fn spawn_physics_components(commands: &mut Commands, entity: Entity, config: &PhysicsConfig) {
    // Build the rigid body type
    let rigid_body = match config.body_type {
        PhysicsBodyType::Dynamic => RigidBody::Dynamic,
        PhysicsBodyType::Kinematic => RigidBody::Kinematic,
        PhysicsBodyType::Static => RigidBody::Static,
    };

    // Build the collider
    let collider = match &config.collider {
        ColliderConfig::Box { width, height } => Collider::rectangle(*width, *height),
        ColliderConfig::Capsule { width, height } => {
            // Capsule in Avian2D: radius is half the width, height is the total height
            let radius = width / 2.0;
            // Capsule height in Avian is the "shaft" height (total height minus the caps)
            let shaft_height = (height - width).max(0.0);
            Collider::capsule(shaft_height, radius)
        }
        ColliderConfig::Circle { radius } => Collider::circle(*radius),
    };

    // Insert physics components
    commands.entity(entity).insert((
        rigid_body,
        collider,
        GravityScale(config.gravity_scale),
        Friction::new(config.friction),
        Restitution::new(config.restitution),
        LinearDamping(config.linear_damping),
    ));

    // Lock rotation if configured
    if config.lock_rotation {
        commands.entity(entity).insert(LockedAxes::ROTATION_LOCKED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_compiles() {
        // Just test that the plugin can be created
        let _plugin = MapEntityPhysicsPlugin;
    }
}
