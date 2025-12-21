//! Tileset Demo - Loading Maps with Tilesets
//!
//! Demonstrates loading a map with tileset from a .map.json file.
//! Create your map in the editor, export to JSON, then run this to display it.
//!
//! This example shows the **recommended approach**: use `MapHandle` to load maps
//! via the Bevy asset system. Hot-reload is supported automatically.
//!
//! Controls:
//! - Arrow keys: Move cursor over tiles
//! - WASD: Pan camera
//!
//! Run with: cargo run --example tileset_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::core::{CollisionShape, LayerData, OneWayDirection};
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tileset Demo - bevy_map_editor".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .init_resource::<CursorState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_cursor, update_display))
        .run();
}

/// Tracks cursor position and the map handle for tile queries
#[derive(Resource, Default)]
struct CursorState {
    map_handle: Option<Handle<MapProject>>,
    cursor_x: u32,
    cursor_y: u32,
}

#[derive(Component)]
struct TileCursor;

#[derive(Component)]
struct InfoDisplay;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cursor_state: ResMut<CursorState>,
) {
    // Spawn camera
    commands.spawn((Camera2d, Transform::from_xyz(64.0, 64.0, 0.0)));

    // Load and spawn map - Hot reloading should also work here.
    let map_handle = asset_server.load("maps/example_project.map.json");
    cursor_state.map_handle = Some(map_handle.clone());
    commands.spawn(MapHandle(map_handle));

    // Spawn cursor
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 1.0, 0.0, 0.5),
            custom_size: Some(Vec2::new(16.0, 16.0)),
            ..default()
        },
        Transform::from_xyz(8.0, 8.0, 100.0),
        TileCursor,
    ));

    // Spawn info display
    commands.spawn((
        Text::new("Tileset Demo\n\nLoading..."),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            max_width: Val::Px(350.0),
            ..default()
        },
        InfoDisplay,
    ));

    info!("Tileset Demo Started - map loading via asset system!");
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cursor_state: ResMut<CursorState>,
    map_assets: Res<Assets<MapProject>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    // Get project bounds from the loaded asset
    let (max_x, max_y) = match cursor_state
        .map_handle
        .as_ref()
        .and_then(|h| map_assets.get(h))
    {
        Some(p) => (
            p.level.width.saturating_sub(1),
            p.level.height.saturating_sub(1),
        ),
        None => return,
    };

    // Move cursor
    if keyboard.just_pressed(KeyCode::ArrowUp) && cursor_state.cursor_y < max_y {
        cursor_state.cursor_y += 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && cursor_state.cursor_y > 0 {
        cursor_state.cursor_y -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) && cursor_state.cursor_x > 0 {
        cursor_state.cursor_x -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) && cursor_state.cursor_x < max_x {
        cursor_state.cursor_x += 1;
    }

    // Pan camera
    if let Ok(mut transform) = camera_query.single_mut() {
        let speed = 100.0 * time.delta_secs();
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
    }
}

fn update_cursor(
    cursor_state: Res<CursorState>,
    mut cursor_query: Query<&mut Transform, With<TileCursor>>,
) {
    let Ok(mut transform) = cursor_query.single_mut() else {
        return;
    };
    transform.translation.x = cursor_state.cursor_x as f32 * 16.0 + 8.0;
    transform.translation.y = cursor_state.cursor_y as f32 * 16.0 + 8.0;
}

fn update_display(
    cursor_state: Res<CursorState>,
    map_assets: Res<Assets<MapProject>>,
    mut display_query: Query<&mut Text, With<InfoDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
        return;
    };

    // Get the loaded MapProject asset
    let Some(project) = cursor_state
        .map_handle
        .as_ref()
        .and_then(|h| map_assets.get(h))
    else {
        *text = Text::new("Loading map...");
        return;
    };

    let x = cursor_state.cursor_x;
    let y = cursor_state.cursor_y;

    let mut display = format!(
        "Tileset Demo (Asset-based)\n\nArrows: Move cursor\nWASD: Pan camera\n\nCursor: ({}, {})\nMap: {}x{}\n\n",
        x, y, project.level.width, project.level.height
    );

    // Show tile info at cursor
    for layer in &project.level.layers {
        if let LayerData::Tiles {
            tileset_id, tiles, ..
        } = &layer.data
        {
            let idx =
                (project.level.height - 1 - y) as usize * project.level.width as usize + x as usize;
            if let Some(tile_idx) = tiles.get(idx).and_then(|t| *t) {
                display.push_str(&format!("{}: tile {}\n", layer.name, tile_idx));

                if let Some(tileset) = project.tilesets.get(tileset_id) {
                    if let Some(props) = tileset.tile_properties.get(&tile_idx) {
                        if props.collision.shape != CollisionShape::None {
                            display.push_str("  [COLLISION]\n");
                        }
                        if props.collision.one_way != OneWayDirection::None {
                            display
                                .push_str(&format!("  [ONE-WAY: {:?}]\n", props.collision.one_way));
                        }
                        for (key, value) in &props.custom {
                            display.push_str(&format!("  {}: {}\n", key, value));
                        }
                    }
                }
            } else {
                display.push_str(&format!("{}: (empty)\n", layer.name));
            }
        }
    }

    *text = Text::new(display);
}
