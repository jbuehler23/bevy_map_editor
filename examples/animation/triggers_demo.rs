//! Animation Triggers Demo - Observer Pattern
//!
//! This example demonstrates both approaches for handling animation events:
//!
//! 1. **Observer Pattern** (Entity-Scoped): Using `.observe()` for entity-specific handling
//! 2. **Message Pattern** (Global): Using `MessageReader` for system-wide handling
//!
//! The "walk_down" animation has a trigger defined in example_project.map.json.
//!
//! Controls:
//! - 1: Play "walk_down" animation
//! - Space: Stop animation
//!
//! Run with: cargo run --example animation_triggers_demo -p bevy_map_editor_examples

use bevy::prelude::*;
use bevy_map::animation::{
    AnimationTriggerEvent, AnimationTriggered, AnimationWindowChanged, AnimationWindowEvent,
    WindowPhase,
};
use bevy_map::prelude::*;
use bevy_map::runtime::AnimatedSpriteHandle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Animation Triggers Demo - Observer Pattern".to_string(),
                resolution: (800, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapRuntimePlugin)
        .init_resource::<EventLog>()
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_global_events, handle_input, update_display))
        .run();
}

#[derive(Component)]
struct EventDisplay;

#[derive(Resource, Default)]
struct EventLog {
    messages: Vec<String>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // ==========================================================================
    // OBSERVER PATTERN: Entity-scoped event handling
    // ==========================================================================
    // Spawn animated sprite and attach observers directly to the entity.
    // WindowTracker is auto-required by AnimatedSprite - no need to add it!
    commands
        .spawn((
            AnimatedSpriteHandle::new(
                asset_server.load("maps/example_project.map.json"),
                "Slime",
                "walk_down", // Animation with a trigger defined
            ),
            Transform::from_xyz(0.0, 50.0, 0.0).with_scale(Vec3::splat(4.0)),
        ))
        // Observer for triggers - fires only for THIS entity
        .observe(
            |trigger: On<AnimationTriggered>, mut log: ResMut<EventLog>| {
                let event = trigger.event();
                log.messages.push(format!(
                    "[Observer] TRIGGER: '{}' at {}ms ({})",
                    event.name, event.time_ms, event.animation
                ));
                info!(
                    "Observer received trigger '{}' at {}ms",
                    event.name, event.time_ms
                );
            },
        )
        // Observer for window phase changes - fires only for THIS entity
        .observe(
            |trigger: On<AnimationWindowChanged>, mut log: ResMut<EventLog>| {
                let event = trigger.event();
                // Only log Begin/End, not every Tick
                if event.phase != WindowPhase::Tick {
                    log.messages.push(format!(
                        "[Observer] WINDOW: '{}' {:?} ({})",
                        event.name, event.phase, event.animation
                    ));
                    info!(
                        "Observer received window '{}' phase {:?}",
                        event.name, event.phase
                    );
                }
            },
        );

    // Event log display
    commands.spawn((
        Text::new("Loading..."),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
        EventDisplay,
    ));

    info!("Triggers Demo - demonstrates Observer pattern for animation events!");
}

// ==========================================================================
// MESSAGE PATTERN: Global event handling (alternative approach)
// ==========================================================================
// This system receives ALL animation events from ALL entities.
// Use this when you need centralized event handling (logging, analytics, etc.)
fn handle_global_events(
    mut log: ResMut<EventLog>,
    mut triggers: MessageReader<AnimationTriggerEvent>,
    mut windows: MessageReader<AnimationWindowEvent>,
) {
    for event in triggers.read() {
        log.messages.push(format!(
            "[Global] TRIGGER: '{}' ({})",
            event.trigger_name, event.animation
        ));
    }

    for event in windows.read() {
        if event.phase != WindowPhase::Tick {
            log.messages.push(format!(
                "[Global] WINDOW: '{}' {:?} ({})",
                event.window_name, event.phase, event.animation
            ));
        }
    }

    // Keep last 12 messages
    while log.messages.len() > 12 {
        log.messages.remove(0);
    }
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
            info!("Stopped");
        }
    }
}

fn update_display(
    log: Res<EventLog>,
    query: Query<Option<&AnimatedSprite>>,
    mut display_query: Query<&mut Text, With<EventDisplay>>,
) {
    let Ok(mut text) = display_query.single_mut() else {
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

    let events_text = if log.messages.is_empty() {
        "No events yet - play 'walk_down' animation!".to_string()
    } else {
        log.messages.join("\n")
    };

    *text = Text::new(format!(
        "Animation Triggers Demo\n\n\
        This demo shows BOTH event patterns:\n\
        - [Observer] Entity-scoped via .observe()\n\
        - [Global] System-wide via MessageReader\n\n\
        Status: {}\n\n\
        Controls:\n\
        1: walk_down | Space: Stop\n\n\
        Events:\n\
        {}",
        status, events_text
    ));
}
