//! Custom Entities Demo - Loading Entities from JSON
//!
//! Demonstrates loading entities with properties from a .map.json file.
//! Define entity types in your game, then place them in the editor.
//!
//! This example shows the **recommended approach**: use `MapHandle` to load maps
//! and register custom entity types with `register_map_entity`. Entities are
//! automatically spawned with their components when the map loads.
//!
//! Controls:
//! - Space: List all entities in console
//! - Tab: Cycle through entity type filters
//!
//! Run with: cargo run --example custom_entities_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Custom Entities Demo - bevy_map_editor".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        // Register your entity types - maps JSON type_name to Rust component
        .register_map_entity::<Npc>()
        .register_map_entity::<Enemy>()
        .init_resource::<FilterState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_display))
        .run();
}

/// NPC entity - matches example_project.map.json schema
#[derive(Component, MapEntity, Debug, Clone)]
#[map_entity(type_name = "NPC")]
pub struct Npc {
    #[map_prop]
    pub name: String,
    #[map_prop(default = "")]
    pub npc_type: String,
}

/// Enemy entity - matches example_project.map.json schema
#[derive(Component, MapEntity, Debug, Clone)]
#[map_entity(type_name = "Enemy")]
pub struct Enemy {
    #[map_prop]
    pub name: String,
    #[map_prop(default = 1.0)]
    pub level: f32,
    #[map_prop(default = 0.0)]
    pub exp: f32,
}

#[derive(Resource, Default)]
struct FilterState {
    current: usize, // 0=all, 1=NPC, 2=Enemy
}

#[derive(Component)]
struct InfoDisplay;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, Transform::from_xyz(300.0, 400.0, 0.0)));

    // Load and spawn map - ONE LINE! Entities are auto-spawned based on registered types.
    commands.spawn(MapHandle(
        asset_server.load("maps/example_project.map.json"),
    ));

    // Spawn info display
    commands.spawn((
        Text::new("Custom Entities Demo\n\nLoading..."),
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

    info!("Custom Entities Demo Started - loading via asset system!");
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut filter: ResMut<FilterState>,
    npcs: Query<(Entity, &Npc, &Transform)>,
    enemies: Query<(Entity, &Enemy, &Transform)>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("=== Entity List ===");
        for (e, npc, t) in npcs.iter() {
            info!(
                "{:?} at {:?}: NPC '{}' type:{}",
                e,
                t.translation.xy(),
                npc.name,
                npc.npc_type
            );
        }
        for (e, enemy, t) in enemies.iter() {
            info!(
                "{:?} at {:?}: Enemy '{}' lvl:{} exp:{}",
                e,
                t.translation.xy(),
                enemy.name,
                enemy.level,
                enemy.exp
            );
        }
    }

    if keyboard.just_pressed(KeyCode::Tab) {
        filter.current = (filter.current + 1) % 3;
    }
}

fn update_display(
    filter: Res<FilterState>,
    npcs: Query<&Npc>,
    enemies: Query<&Enemy>,
    mut display_query: Query<&mut Text, With<InfoDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
        return;
    };

    let filter_name = match filter.current {
        0 => "All",
        1 => "NPCs",
        2 => "Enemies",
        _ => "?",
    };

    let mut display = format!(
        "Custom Entities Demo\n\n\
        SPACE: List (console)\n\
        TAB: Filter\n\n\
        Filter: {}\n\n",
        filter_name
    );

    if filter.current == 0 || filter.current == 1 {
        display.push_str(&format!("NPCs ({})\n", npcs.iter().count()));
        for npc in npcs.iter() {
            let npc_type = if npc.npc_type.is_empty() {
                "generic"
            } else {
                &npc.npc_type
            };
            display.push_str(&format!("  {} [{}]\n", npc.name, npc_type));
        }
    }

    if filter.current == 0 || filter.current == 2 {
        display.push_str(&format!("\nEnemies ({})\n", enemies.iter().count()));
        for enemy in enemies.iter() {
            display.push_str(&format!(
                "  {} Lv{:.0} ({}xp)\n",
                enemy.name, enemy.level, enemy.exp
            ));
        }
    }

    *text = Text::new(display);
}
