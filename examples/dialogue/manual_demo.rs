//! Dialogue Manual Demo - Full Control Loading
//!
//! This example demonstrates the **manual loading** approach for dialogue trees.
//! You have full control over the loading process and can access dialogue data directly.
//!
//! This approach is useful when you need:
//! - Access to dialogue data before attaching it
//! - Custom dialogue processing or validation
//! - More complex initialization logic
//!
//! Controls:
//! - Space: Start dialogue / Advance text
//! - A/B/C: Select choice options
//!
//! Run with: cargo run --example dialogue_manual_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::dialogue::{DialogueNodeType, DialogueTree};
use bevy_map::prelude::*;
use bevy_map::runtime::MapProjectLoader;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dialogue Manual Demo - Full Control".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DialoguePlugin)
        // Register MapProject asset manually (not using MapRuntimePlugin)
        .init_asset::<MapProject>()
        .init_asset_loader::<MapProjectLoader>()
        .init_resource::<ManualLoadingState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (setup_dialogue_when_ready, handle_input, update_display),
        )
        .run();
}

/// State for manual loading approach
#[derive(Resource, Default)]
struct ManualLoadingState {
    map_handle: Option<Handle<MapProject>>,
    tree: Option<DialogueTree>,
    initialized: bool,
}

#[derive(Component)]
struct DialogueDisplay;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<ManualLoadingState>,
) {
    commands.spawn(Camera2d);

    // =========================================================================
    // Load the MapProject asset
    // =========================================================================
    // We load the MapProject and will manually extract the dialogue tree
    let map_handle = asset_server.load("maps/example_project.map.json");
    state.map_handle = Some(map_handle);

    // Display
    commands.spawn((
        Text::new("Dialogue Manual Demo\n\nManual Loading Approach\n\nLoading..."),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(50.0),
            top: Val::Px(50.0),
            max_width: Val::Px(700.0),
            ..default()
        },
        DialogueDisplay,
    ));

    info!("Dialogue Manual Demo - full control over loading process!");
}

/// Manual loading: extracts dialogue tree once MapProject is loaded
fn setup_dialogue_when_ready(
    map_assets: Res<Assets<MapProject>>,
    mut state: ResMut<ManualLoadingState>,
) {
    if state.initialized {
        return;
    }

    let Some(handle) = &state.map_handle else {
        return;
    };
    let Some(project) = map_assets.get(handle) else {
        return;
    };

    // =========================================================================
    // Get the first dialogue tree from the project
    // =========================================================================
    // You have access to the raw DialogueTree data here
    if let Some(tree) = project.dialogues.values().next() {
        info!(
            "Manual approach: loaded dialogue '{}' with {} nodes",
            tree.name,
            tree.nodes.len()
        );
        state.tree = Some(tree.clone());
    } else {
        warn!("No dialogues found in project");
    }

    state.initialized = true;
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<ManualLoadingState>,
    mut runner: ResMut<DialogueRunner>,
) {
    let Some(tree) = &state.tree else { return };

    // Space to start/advance dialogue
    if keyboard.just_pressed(KeyCode::Space) {
        if !runner.is_active() {
            runner.active = true;
            runner.current_node_id = Some(tree.start_node.clone());
        } else if let Some(current_id) = &runner.current_node_id {
            if let Some(node) = tree.get_node(current_id) {
                match node.node_type {
                    DialogueNodeType::Text => {
                        if let Some(next) = &node.next_node {
                            runner.current_node_id = Some(next.clone());
                        } else {
                            runner.end();
                        }
                    }
                    DialogueNodeType::End => {
                        runner.end();
                    }
                    _ => {}
                }
            }
        }
    }

    // Choice selection (A/B/C keys)
    if runner.is_active() {
        if let Some(current_id) = &runner.current_node_id {
            if let Some(node) = tree.get_node(current_id) {
                if node.node_type == DialogueNodeType::Choice {
                    for (i, key) in [KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC]
                        .iter()
                        .enumerate()
                    {
                        if keyboard.just_pressed(*key) && node.choices.len() > i {
                            if let Some(next) = &node.choices[i].next_node {
                                runner.current_node_id = Some(next.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_display(
    state: Res<ManualLoadingState>,
    runner: Res<DialogueRunner>,
    mut display_query: Query<&mut Text, With<DialogueDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
        return;
    };

    let Some(tree) = &state.tree else {
        *text = Text::new("Dialogue Manual Demo\n\nManual Loading Approach\n\nLoading dialogue...");
        return;
    };

    let mut display = "Dialogue Manual Demo\n\n\
        Manual Loading Approach\n\n\
        Space: Start/Advance | A/B/C: Choose\n\n\
        ----------------------------------------\n\n"
        .to_string();

    if !runner.is_active() {
        display.push_str("Press SPACE to start the dialogue...");
    } else if let Some(current_id) = &runner.current_node_id {
        if let Some(node) = tree.get_node(current_id) {
            display.push_str(&format!("[{}]\n\n", node.speaker));
            display.push_str(&format!("\"{}\"\n\n", node.text));

            match node.node_type {
                DialogueNodeType::Choice => {
                    for (i, choice) in node.choices.iter().enumerate() {
                        let key = ['A', 'B', 'C'][i];
                        display.push_str(&format!("[{}] {}\n", key, choice.text));
                    }
                }
                DialogueNodeType::Text => {
                    display.push_str("\nPress SPACE to continue...");
                }
                DialogueNodeType::End => {
                    display.push_str("\n[End of dialogue]\nPress SPACE to close");
                }
                _ => {}
            }
        }
    }

    *text = Text::new(display);
}
