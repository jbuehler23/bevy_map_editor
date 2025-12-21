//! Animation Auto Demo - Using AnimatedSpriteHandle
//!
//! This example demonstrates the **automatic loading** approach using `AnimatedSpriteHandle`.
//! With just one spawn call, the sprite sheet is automatically loaded from the MapProject.
//!
//! Controls:
//! - 1: Play "walk_down" animation
//! - Space: Stop animation
//!
//! Run with: cargo run --example animation_auto_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::AnimatedSpriteHandle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Animation Auto Demo - AnimatedSpriteHandle".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .add_plugins(SpriteAnimationPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_hud))
        .run();
}

#[derive(Component)]
struct AnimationHud;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // AnimatedSpriteHandle handles everything automatically
    // - Waits for MapProject to load
    // - Finds sprite sheet by name
    // - Loads the texture
    // - Adds Sprite and AnimatedSprite components automatically
    // - WindowTracker is auto-required by AnimatedSprite
    commands.spawn((
        AnimatedSpriteHandle::new(
            asset_server.load("maps/example_project.map.json"),
            "Slime",     // sprite sheet name in the editor
            "walk_down", // initial animation to play
        ),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(4.0)),
    ));

    // HUD
    commands.spawn((
        Text::new("Animation Auto Demo\n\nUsing AnimatedSpriteHandle\n\n1: Play walk_down\nSpace: Stop\n\nLoading..."),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
        AnimationHud,
    ));

    info!("Animation Auto Demo - using AnimatedSpriteHandle for minimal boilerplate!");
}

fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut AnimatedSprite>) {
    let animation = if keyboard.just_pressed(KeyCode::Digit1) {
        Some("walk_down")
    } else {
        None
    };

    let stop = keyboard.just_pressed(KeyCode::Space);

    if let Ok(mut animated) = query.single_mut() {
        if let Some(name) = animation {
            animated.play(name);
            info!("Playing: {}", name);
        }
        if stop {
            animated.stop();
            info!("Animation stopped");
        }
    }
}

fn update_hud(
    query: Query<Option<&AnimatedSprite>>,
    mut hud_query: Query<&mut Text, With<AnimationHud>>,
) {
    let Ok(mut text) = hud_query.single_mut() else {
        return;
    };

    let status = match query.single() {
        Ok(Some(a)) => format!(
            "{} ({})",
            a.current_animation.as_deref().unwrap_or("none"),
            if a.playing { "playing" } else { "stopped" }
        ),
        Ok(None) => "Loading...".to_string(),
        Err(_) => "Not found".to_string(),
    };

    *text = Text::new(format!(
        "Animation Auto Demo\n\n\
        Using AnimatedSpriteHandle\n\n\
        Status: {}\n\n\
        1: walk_down\n\
        Space: Stop",
        status
    ));
}
