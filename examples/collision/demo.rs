//! Collision Demo - Minimal Platformer with Map Editor Integration
//!
//! Demonstrates tile-based collisions with a simple platformer character.
//! The map defines collision shapes on tiles, and a Player entity for the spawn point.
//!
//! Controls:
//! - WASD/Arrow keys: Move
//! - Space: Jump (when on ground)
//!
//! Run with: cargo run --example collision_demo -p bevy_map_editor_examples

use avian2d::prelude::*;
use bevy::asset::{AssetPlugin, UnapprovedPathMode};
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::MapCollisionPlugin;
use bevy_map::AnimatedSprite;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Collision Demo - bevy_map_editor".to_string(),
                        resolution: (800, 600).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)
        // Platformer gravity
        .insert_resource(Gravity(Vec2::new(0.0, -800.0)))
        // Register Player entity type
        .register_map_entity::<Player>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                setup_player_physics,
                player_movement,
                update_player_animation,
                camera_follow,
            ),
        )
        .run();
}

/// Player component - spawned from map entity
#[derive(Component, MapEntity, Debug, Clone)]
#[map_entity(type_name = "Player")]
pub struct Player {
    /// Optional sprite reference
    #[map_prop(default = "")]
    pub sprite: String,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn((Camera2d, Transform::from_xyz(200.0, 300.0, 0.0)));

    // Load map - Player entity and collisions spawn automatically
    commands.spawn(MapHandle(
        asset_server.load("maps/platformer_example.map.json"),
    ));

    info!("Collision Demo - WASD to move, Space to jump");
}

/// Add physics components when Player entity spawns from map
fn setup_player_physics(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &mut Sprite), Added<Player>>,
) {
    for (entity, transform, mut sprite) in query.iter_mut() {
        // Make player sprite visible
        sprite.color = Color::srgb(0.2, 0.9, 0.2);
        sprite.custom_size = Some(Vec2::new(16.0, 24.0));

        // Set up physics
        let mut player_transform = *transform;
        player_transform.translation.z = 10.0;

        commands.entity(entity).insert((
            player_transform,
            RigidBody::Dynamic,
            // Capsule prevents catching on tile edges
            Collider::capsule(7.0, 8.0),
            CollisionLayers::from_bits(1, 1),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.1),
            Restitution::new(0.0),
            LinearVelocity::ZERO,
            // Track collisions for ground detection
            CollidingEntities::default(),
        ));

        info!("Player spawned at {:?}", transform.translation);
    }
}

/// Simple platformer movement
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut LinearVelocity, &CollidingEntities), With<Player>>,
) {
    const MOVE_SPEED: f32 = 150.0;
    const JUMP_FORCE: f32 = 380.0;

    for (mut velocity, colliding) in query.iter_mut() {
        // Horizontal movement
        let mut direction = 0.0;
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction += 1.0;
        }
        velocity.x = direction * MOVE_SPEED;

        // Simple grounded check - if colliding with anything and not moving up fast
        let grounded = !colliding.is_empty() && velocity.y <= 1.0;

        // Jump
        if grounded && keyboard.just_pressed(KeyCode::Space) {
            velocity.y = JUMP_FORCE;
        }
    }
}

/// Camera follows player with smooth lerp
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

    let target = player_transform.translation.xy();
    let current = camera_transform.translation.xy();
    let new_pos = current.lerp(target, 0.1);
    camera_transform.translation.x = new_pos.x;
    camera_transform.translation.y = new_pos.y;
}

/// Update player animation based on movement state
///
/// Switches between "idle" and "run" animations based on velocity,
/// and flips the sprite based on movement direction.
fn update_player_animation(
    mut query: Query<(&LinearVelocity, Option<&mut AnimatedSprite>, &mut Sprite), With<Player>>,
) {
    for (velocity, animated, mut sprite) in query.iter_mut() {
        // Flip sprite based on movement direction
        if velocity.x < -10.0 {
            sprite.flip_x = true;
        } else if velocity.x > 10.0 {
            sprite.flip_x = false;
        }

        // Switch animation if AnimatedSprite is present
        if let Some(mut animated) = animated {
            let is_moving = velocity.x.abs() > 10.0;
            let current = animated.current_animation.as_deref();

            if is_moving && current != Some("run") {
                animated.play("run");
            } else if !is_moving && current != Some("idle") {
                animated.play("idle");
            }
        }
    }
}
