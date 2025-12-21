//! Collision Demo - Physics Integration with Map Editor
//!
//! Demonstrates how easy it is to integrate tile-based collisions into Bevy games.
//! The map defines collision shapes on tiles, and a Player entity for the spawn point.
//!
//! This example shows:
//! - Automatic collision spawning via `MapCollisionPlugin`
//! - Data-driven Player spawn from map entity
//! - Simple physics-based movement
//! - Debug visualization of collision shapes
//!
//! Controls:
//! - WASD: Move player
//! - Arrow keys: Pan camera
//!
//! Run with: cargo run --example collision_demo -p bevy_map_editor_examples

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_map::core::CollisionShape;
use bevy_map::prelude::*;
use bevy_map::runtime::{MapCollider, MapCollisionPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Collision Demo - bevy_map_editor".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin) // Auto-spawns Avian2D colliders!
        .add_plugins(PhysicsPlugins::default())
        // Disable gravity for top-down movement
        .insert_resource(Gravity(Vec2::ZERO))
        // Register Player entity type - will auto-spawn from map data
        .register_map_entity::<Player>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                setup_player_physics,
                player_movement,
                camera_follow,
                draw_collision_debug,
                update_info_display,
            ),
        )
        .run();
}

/// Player component - derived from map entity
/// Place a "Player" entity in the map editor to define spawn point
#[derive(Component, MapEntity, Debug, Clone)]
#[map_entity(type_name = "Player")]
pub struct Player {
    #[map_prop(default = "Player1")]
    pub name: String,
}

#[derive(Component)]
struct PlayerSprite;

#[derive(Component)]
struct InfoDisplay;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn((Camera2d, Transform::from_xyz(160.0, 120.0, 0.0)));

    // Load map - Player entity and collisions spawn automatically!
    commands.spawn(MapHandle(asset_server.load("maps/collision_demo.map.json")));

    // Spawn info display
    commands.spawn((
        Text::new("Collision Demo\n\nLoading map..."),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            max_width: Val::Px(250.0),
            ..default()
        },
        InfoDisplay,
    ));

    info!("Collision Demo Started - loading map with collisions!");
}

/// When Player entity spawns from map, add physics components and sprite
fn setup_player_physics(
    mut commands: Commands,
    query: Query<(Entity, &Player, &Transform), Added<Player>>,
) {
    for (entity, player, transform) in query.iter() {
        info!(
            "Player '{}' spawned at {:?}",
            player.name,
            transform.translation.xy()
        );

        // Add physics body and collider
        commands.entity(entity).insert((
            RigidBody::Kinematic,
            Collider::rectangle(12.0, 12.0),
            // Visual representation
            Sprite {
                color: Color::srgb(0.2, 0.8, 0.2),
                custom_size: Some(Vec2::new(14.0, 14.0)),
                ..default()
            },
            PlayerSprite,
        ));
    }
}

/// Simple top-down player movement
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut LinearVelocity, With<Player>>,
) {
    let speed = 100.0;

    for mut velocity in query.iter_mut() {
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

        // Normalize to prevent faster diagonal movement
        if direction != Vec2::ZERO {
            direction = direction.normalize();
        }

        velocity.0 = direction * speed;
    }
}

/// Camera follows player
fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    // Smooth camera follow
    let target = player_transform.translation.xy();
    let current = camera_transform.translation.xy();
    let new_pos = current.lerp(target, 0.1);
    camera_transform.translation.x = new_pos.x;
    camera_transform.translation.y = new_pos.y;
}

/// Draw collision shapes using Gizmos for debug visualization
fn draw_collision_debug(
    mut gizmos: Gizmos,
    colliders: Query<(&Transform, &Collider, &MapCollider)>,
) {
    for (transform, collider, map_collider) in colliders.iter() {
        let pos = transform.translation.xy();
        let color = Color::srgba(1.0, 0.3, 0.3, 0.5);

        // Draw based on collider shape
        match &map_collider.data.shape {
            CollisionShape::Full => {
                // Full tile - draw as rectangle
                if let Some(rect) = collider.shape().as_cuboid() {
                    let half = rect.half_extents;
                    gizmos.rect_2d(
                        Isometry2d::from_translation(pos),
                        Vec2::new(half.x * 2.0, half.y * 2.0),
                        color,
                    );
                }
            }
            CollisionShape::Rectangle { .. } => {
                if let Some(rect) = collider.shape().as_cuboid() {
                    let half = rect.half_extents;
                    gizmos.rect_2d(
                        Isometry2d::from_translation(pos),
                        Vec2::new(half.x * 2.0, half.y * 2.0),
                        color,
                    );
                }
            }
            CollisionShape::Circle { .. } => {
                if let Some(circle) = collider.shape().as_ball() {
                    gizmos.circle_2d(Isometry2d::from_translation(pos), circle.radius, color);
                }
            }
            CollisionShape::Polygon { .. } => {
                // For polygons, just draw a marker
                gizmos.circle_2d(
                    Isometry2d::from_translation(pos),
                    4.0,
                    Color::srgba(0.3, 1.0, 0.3, 0.5),
                );
            }
            CollisionShape::None => {}
        }
    }
}

/// Update info display
fn update_info_display(
    player_query: Query<&Transform, With<Player>>,
    colliders: Query<&MapCollider>,
    mut display_query: Query<&mut Text, With<InfoDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
        return;
    };

    let player_pos = player_query
        .single()
        .map(|t| t.translation.xy())
        .unwrap_or(Vec2::ZERO);

    let collider_count = colliders.iter().count();

    let display = format!(
        "Collision Demo\n\n\
        WASD: Move player\n\n\
        Player: ({:.0}, {:.0})\n\
        Colliders: {}\n\n\
        Red shapes = collision",
        player_pos.x, player_pos.y, collider_count
    );

    *text = Text::new(display);
}
