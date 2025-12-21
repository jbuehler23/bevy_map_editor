//! Runtime loader example - the simplest way to load a map
//!
//! Demonstrates loading a map with just MapHandle and registering custom entities.
//!
//! Controls:
//! - WASD: Pan camera
//! - Q/E: Zoom in/out
//! - Space: Print spawned entities
//!
//! Run with: cargo run --example runtime_loader -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_map - Runtime Loader".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        // Register custom entity types that match our map's data types
        .register_map_entity::<Npc>()
        .register_map_entity::<Enemy>()
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_controls, print_entities))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn((Camera2d, Transform::from_xyz(200.0, 300.0, 0.0)));

    // Load the map - that's it!
    commands.spawn(MapHandle(
        asset_server.load("maps/example_project.map.json"),
    ));

    info!("Loading map...");
    info!("Press SPACE to see spawned entities");
}

// =============================================================================
// Custom Entity Types - these match the data types defined in the editor
// =============================================================================

/// NPC entity - matches the "NPC" data type in example_project.map.json
#[derive(Component, MapEntity, Debug)]
#[map_entity(type_name = "NPC")]
pub struct Npc {
    #[map_prop]
    pub name: String,
    #[map_prop]
    pub npc_type: String,
}

/// Enemy entity - matches the "Enemy" data type in example_project.map.json
#[derive(Component, MapEntity, Debug)]
#[map_entity(type_name = "Enemy")]
pub struct Enemy {
    #[map_prop]
    pub name: String,
    #[map_prop(default = 1.0)]
    pub level: f32,
    #[map_prop(default = 0.0)]
    pub exp: f32,
}

// =============================================================================
// Systems
// =============================================================================

/// Print all spawned entities when Space is pressed
fn print_entities(
    keyboard: Res<ButtonInput<KeyCode>>,
    npcs: Query<(Entity, &Transform, &Npc)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("=== Spawned Entities ===");

        for (entity, transform, npc) in npcs.iter() {
            info!(
                "NPC {:?} at ({:.0}, {:.0}): {} ({})",
                entity, transform.translation.x, transform.translation.y, npc.name, npc.npc_type
            );
        }

        for (entity, transform, enemy) in enemies.iter() {
            info!(
                "Enemy {:?} at ({:.0}, {:.0}): {} (lvl {}, {} exp)",
                entity,
                transform.translation.x,
                transform.translation.y,
                enemy.name,
                enemy.level,
                enemy.exp
            );
        }

        if npcs.is_empty() && enemies.is_empty() {
            info!("No entities spawned yet - map may still be loading");
        }
    }
}

/// Camera controls: WASD to pan, Q/E to zoom
fn camera_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = query.single_mut() else {
        return;
    };

    let speed = 200.0 * time.delta_secs();

    if keyboard.pressed(KeyCode::KeyW) {
        transform.translation.y += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        transform.translation.y -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        transform.translation.x -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        transform.translation.x += speed;
    }

    if let Projection::Orthographic(ref mut ortho) = *projection {
        if keyboard.pressed(KeyCode::KeyQ) {
            ortho.scale *= 1.0 + time.delta_secs();
        }
        if keyboard.pressed(KeyCode::KeyE) {
            ortho.scale *= 1.0 - time.delta_secs();
        }
        ortho.scale = ortho.scale.clamp(0.25, 4.0);
    }
}
