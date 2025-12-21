//! Animation Manual Demo - Full Control Loading
//!
//! This example demonstrates the **manual loading** approach for animated sprites.
//! You have full control over the loading process, asset management, and component setup.
//!
//! This approach is useful when you need:
//! - Access to sprite data before spawning
//! - Custom texture processing
//! - More complex initialization logic
//!
//! Controls:
//! - 1: Play "walk_down" animation
//! - Space: Stop animation
//!
//! Run with: cargo run --example animation_manual_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Animation Manual Demo - Full Control".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .add_plugins(SpriteAnimationPlugin)
        .init_resource::<ManualLoadingState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (setup_sprite_when_ready, handle_input, update_hud))
        .run();
}

#[derive(Component)]
struct AnimationHud;

/// State for manual loading approach
#[derive(Resource, Default)]
struct ManualLoadingState {
    map_handle: Option<Handle<MapProject>>,
    initialized: bool,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<ManualLoadingState>,
) {
    commands.spawn(Camera2d);

    // =========================================================================
    // MANUAL LOADING: Load the MapProject asset
    // =========================================================================
    // We load the MapProject and will manually extract the sprite data
    // and set up the components ourselves in setup_sprite_when_ready()
    let map_handle = asset_server.load("maps/example_project.map.json");
    state.map_handle = Some(map_handle);

    // HUD
    commands.spawn((
        Text::new("Animation Manual Demo\n\nManual Loading Approach\n\n1: Play walk_down\nSpace: Stop\n\nLoading..."),
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

    info!("Animation Manual Demo - full control over loading process!");
}

/// Manual loading: sets up sprite once the MapProject is loaded
fn setup_sprite_when_ready(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map_assets: Res<Assets<MapProject>>,
    mut sprite_data_assets: ResMut<Assets<SpriteData>>,
    mut state: ResMut<ManualLoadingState>,
) {
    // Only run once
    if state.initialized {
        return;
    }

    // Wait for asset to load
    let Some(handle) = &state.map_handle else {
        return;
    };
    let Some(project) = map_assets.get(handle) else {
        return;
    };

    // =========================================================================
    // MANUAL: Get the sprite sheet from the project
    // =========================================================================
    let Some(sprite_data) = project.sprite_sheet_by_name("Slime") else {
        warn!("Sprite sheet 'Slime' not found in project");
        return;
    };

    // Add sprite data to assets to get a handle
    let sprite_data_handle = sprite_data_assets.add(sprite_data.clone());

    // Load the spritesheet texture manually
    let texture: Handle<Image> = asset_server.load(&sprite_data.sheet_path);

    // Create the animated sprite component
    let mut animated = AnimatedSprite::new(sprite_data_handle);
    animated.play("walk_down");

    // =========================================================================
    // MANUAL: Spawn with full control over all properties
    // =========================================================================
    commands.spawn((
        Sprite {
            image: texture,
            rect: Some(Rect::new(
                0.0,
                0.0,
                sprite_data.frame_width as f32,
                sprite_data.frame_height as f32,
            )),
            custom_size: Some(Vec2::new(
                sprite_data.frame_width as f32 * 4.0,
                sprite_data.frame_height as f32 * 4.0,
            )),
            ..default()
        },
        animated,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    state.initialized = true;
    info!("Manual sprite created from loaded map project");
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
        Err(_) => "Setting up...".to_string(),
    };

    *text = Text::new(format!(
        "Animation Manual Demo\n\n\
        Manual Loading Approach\n\n\
        Status: {}\n\n\
        1: walk_down\n\
        Space: Stop",
        status
    ));
}
