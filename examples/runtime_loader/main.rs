//! Runtime loader example with custom entities
//!
//! Demonstrates how to load a map at runtime and spawn custom entity types.
//!
//! Run with: cargo run --example runtime_loader -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_map_editor - Runtime Loader Example".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        // Register custom entity types
        .register_map_entity::<Npc>()
        .register_map_entity::<Chest>()
        .register_map_entity::<SpawnPoint>()
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controls)
        .add_systems(Update, highlight_entities)
        .run();
}

/// Example NPC entity that can be placed in the map editor
#[derive(Component, MapEntity, Debug)]
#[map_entity(type_name = "NPC")]
pub struct Npc {
    #[map_prop]
    pub name: String,
    #[map_prop(default = 100)]
    pub health: i32,
}

/// Example chest entity with loot
#[derive(Component, MapEntity, Debug)]
#[map_entity(type_name = "Chest")]
pub struct Chest {
    #[map_prop]
    pub loot_table: String,
    #[map_prop(default = false)]
    pub locked: bool,
    #[map_prop(default = 1)]
    pub tier: i32,
}

/// Player spawn point
#[derive(Component, MapEntity, Debug)]
#[map_entity(type_name = "SpawnPoint")]
pub struct SpawnPoint {
    #[map_prop(default = 0)]
    pub spawn_id: i32,
    #[map_prop(default = false)]
    pub is_default: bool,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut spawn_events: bevy::ecs::message::MessageWriter<SpawnMapProjectEvent>,
) {
    // Spawn camera
    commands.spawn((Camera2d, Transform::from_xyz(128.0, 128.0, 0.0)));

    // Load map project from embedded JSON
    // In a real game, you would load this from a file or asset
    let json = include_str!("../assets/maps/example_project.map.json");
    let project: MapProject = serde_json::from_str(json).expect("Failed to parse map JSON");

    // Load tileset textures
    let mut textures = TilesetTextures::new();
    textures.load_from_project(&project, &asset_server);

    // Send event to spawn the map
    // Custom entities (NPC, Chest, SpawnPoint) will be automatically spawned
    // based on entities defined in the map
    spawn_events.write(SpawnMapProjectEvent {
        project,
        textures,
        transform: Transform::default(),
    });

    info!("Map loaded with custom entity types registered!");
    info!("- NPC entities will have Npc component");
    info!("- Chest entities will have Chest component");
    info!("- SpawnPoint entities will have SpawnPoint component");
}

/// Simple camera controls: WASD to pan, Q/E to zoom
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

/// Highlight spawned entities in the console when pressing Space
fn highlight_entities(
    keyboard: Res<ButtonInput<KeyCode>>,
    npcs: Query<(Entity, &Npc)>,
    chests: Query<(Entity, &Chest)>,
    spawn_points: Query<(Entity, &SpawnPoint)>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("=== Spawned Entities ===");

        for (entity, npc) in npcs.iter() {
            info!("NPC {:?}: {} (HP: {})", entity, npc.name, npc.health);
        }

        for (entity, chest) in chests.iter() {
            info!(
                "Chest {:?}: loot={}, locked={}, tier={}",
                entity, chest.loot_table, chest.locked, chest.tier
            );
        }

        for (entity, spawn) in spawn_points.iter() {
            info!(
                "SpawnPoint {:?}: id={}, default={}",
                entity, spawn.spawn_id, spawn.is_default
            );
        }
    }
}
