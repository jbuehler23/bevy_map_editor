//! Dialogue Auto Demo - Using DialogueTreeHandle
//!
//! This example demonstrates the **automatic loading** approach using `DialogueTreeHandle`.
//! With just one spawn call, the dialogue tree is automatically loaded from the MapProject.
//!
//! Controls:
//! - Space: Start dialogue / Advance text
//! - A/B/C: Select choice options
//!
//! Run with: cargo run --example dialogue_auto_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::dialogue::{DialogueNodeType, DialogueTree};
use bevy_map::prelude::*;
use bevy_map::runtime::DialogueTreeHandle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dialogue Auto Demo - DialogueTreeHandle".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_display))
        .run();
}

#[derive(Component)]
struct DialogueDisplay;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // =========================================================================
    // DialogueTreeHandle handles everything automatically
    // =========================================================================
    // - Waits for MapProject to load
    // - Finds dialogue tree by name
    // - Adds DialogueHandle component automatically
    commands.spawn(DialogueTreeHandle::new(
        asset_server.load("maps/example_project.map.json"),
        "merchant_greeting", // dialogue name in the editor
    ));

    // Display
    commands.spawn((
        Text::new("Dialogue Auto Demo\n\nUsing DialogueTreeHandle\n\nLoading..."),
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

    info!("Dialogue Auto Demo - using DialogueTreeHandle for minimal boilerplate!");
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut runner: ResMut<DialogueRunner>,
    dialogue_query: Query<&DialogueHandle>,
    dialogue_assets: Res<Assets<DialogueTree>>,
) {
    // Get the dialogue tree from the auto-loaded handle
    let Ok(handle) = dialogue_query.single() else {
        return;
    };
    let Some(tree) = dialogue_assets.get(&handle.0) else {
        return;
    };

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
    runner: Res<DialogueRunner>,
    dialogue_query: Query<Option<&DialogueHandle>>,
    dialogue_assets: Res<Assets<DialogueTree>>,
    mut display_query: Query<&mut Text, With<DialogueDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
        return;
    };

    // Check if dialogue is loaded
    let tree = match dialogue_query.single() {
        Ok(Some(h)) => dialogue_assets.get(&h.0),
        _ => None,
    };

    let Some(tree) = tree else {
        *text = Text::new("Dialogue Auto Demo\n\nUsing DialogueTreeHandle\n\nLoading dialogue...");
        return;
    };

    let mut display = "Dialogue Auto Demo\n\n\
        Using DialogueTreeHandle\n\n\
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
