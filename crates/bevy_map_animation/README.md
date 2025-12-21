# bevy_map_animation

Sprite sheet animations for the bevy_map_editor ecosystem.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Features

- Define sprite sheets with frame dimensions
- Multiple named animations per sheet
- Loop modes: Loop, Once, PingPong
- Frame-based timing
- Automatic sprite rect updates
- **Animation Triggers**: One-shot events at specific times (sounds, VFX, game events)
- **Animation Windows**: Duration-based events with Begin/Tick/End phases (hitboxes, combo windows)
- **Custom Event Types**: Type-safe extensible trigger/window system with Bevy Observers

## Types

| Type                       | Description                                       |
|----------------------------|---------------------------------------------------|
| `SpriteData`               | Sprite sheet definition with animations           |
| `AnimationDef`             | Single animation (frames, timing, loop mode)      |
| `AnimatedSprite`           | Component for playing animations                  |
| `AnimationTrigger`         | One-shot trigger definition (time + payload)      |
| `AnimationWindow`          | Duration-based window definition (start/end + payload) |
| `LoopMode`                 | Loop, Once, or PingPong                           |
| `WindowTracker`            | Component to enable window event tracking         |

## Events

| Event                      | Description                                       |
|----------------------------|---------------------------------------------------|
| `AnimationTriggerEvent`    | Fired when a trigger fires (generic)              |
| `AnimationWindowEvent`     | Fired for window phase changes (Begin/Tick/End)   |
| `AnimationSoundEvent`      | Convenience event for sound payloads              |
| `AnimationParticleEvent`   | Convenience event for particle/VFX payloads       |
| `AnimationCustomEvent`     | Convenience event for custom payloads             |

## Usage

### Defining Animations (Code)

```rust
use bevy_map::animation::{SpriteData, AnimationDef, LoopMode};

let mut sprite = SpriteData::new("sprites/character.png", 32, 32);

sprite.add_animation("idle", AnimationDef {
    frames: vec![0, 1, 2, 3],
    frame_duration_ms: 200,
    loop_mode: LoopMode::Loop,
    ..Default::default()
});

sprite.add_animation("attack", AnimationDef {
    frames: vec![4, 5, 6, 7, 8],
    frame_duration_ms: 80,
    loop_mode: LoopMode::Once,
    ..Default::default()
});
```

### Adding Triggers and Windows

```rust
use bevy_map::animation::{AnimationDef, AnimationTrigger, AnimationWindow, TriggerPayload};

let mut attack_anim = AnimationDef::new(vec![4, 5, 6, 7, 8], 80, LoopMode::Once);

// One-shot trigger at 240ms - play impact sound
attack_anim.add_trigger(AnimationTrigger::with_payload(
    "impact",
    240,
    TriggerPayload::Sound {
        path: "sounds/impact.ogg".into(),
        volume: 0.8,
    },
));

// Duration-based window from 160-320ms - hitbox active
attack_anim.add_window(AnimationWindow::with_payload(
    "hitbox",
    160,
    320,
    TriggerPayload::Custom {
        event_name: "attack_hitbox".into(),
        params: [("damage".into(), serde_json::json!(25))].into(),
    },
));
```

### Handling Animation Events

```rust
use bevy::prelude::*;
use bevy_map::animation::{AnimationTriggerEvent, AnimationWindowEvent, WindowPhase};

fn handle_triggers(mut events: MessageReader<AnimationTriggerEvent>) {
    for event in events.read() {
        info!("Trigger: {} at {}", event.trigger_name, event.animation);
    }
}

fn handle_windows(mut events: MessageReader<AnimationWindowEvent>) {
    for event in events.read() {
        match event.phase {
            WindowPhase::Begin => info!("Window {} started", event.window_name),
            WindowPhase::Tick => { /* every frame while active */ },
            WindowPhase::End => info!("Window {} ended", event.window_name),
        }
    }
}
```

### Custom Type-Safe Triggers

Define custom trigger types for type-safe event handling with Bevy Observers:

```rust
use bevy::prelude::*;
use bevy_map::animation::{AnimationTriggerType, AnimationEventExt};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Event, Clone)]
pub struct AttackHitbox {
    pub damage: i32,
    pub knockback: f32,
}

impl AnimationTriggerType for AttackHitbox {
    fn trigger_name() -> &'static str {
        "attack_hitbox"  // matches event_name in TriggerPayload::Custom
    }

    fn from_params(params: &HashMap<String, Value>) -> Option<Self> {
        Some(Self {
            damage: params.get("damage")?.as_i64()? as i32,
            knockback: params.get("knockback")
                .and_then(|v| v.as_f64())
                .unwrap_or(5.0) as f32,
        })
    }
}

// Register the custom trigger type
app.register_animation_trigger::<AttackHitbox>();

// Handle with Bevy Observer
fn setup(mut commands: Commands) {
    commands.spawn((
        AnimatedSprite::new(/* ... */),
        Observer::new(|trigger: Trigger<AttackHitbox>| {
            let hitbox = trigger.event();
            info!("Attack! Damage: {}", hitbox.damage);
        }),
    ));
}
```

### Playing Animations

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

fn play_animation(mut query: Query<&mut AnimatedSprite>) {
    for mut animated in query.iter_mut() {
        animated.play("idle");
    }
}
```

### Enabling Window Events

Add `WindowTracker` component to entities that need window event tracking:

```rust
commands.spawn((
    AnimatedSprite::new(sprite_data_handle),
    WindowTracker::default(),  // Required for window events
    // ... other components
));
```

### Auto-Loading from Maps

Use `AnimatedSpriteHandle` for automatic loading:

```rust
use bevy_map::runtime::AnimatedSpriteHandle;

commands.spawn((
    AnimatedSpriteHandle::new(
        asset_server.load("maps/game.map.json"),
        "player_sprite",
        "idle",
    ),
    Transform::default(),
));
```

## Plugin

Add `SpriteAnimationPlugin` for automatic animation updates:

```rust
use bevy_map::prelude::*;

app.add_plugins(SpriteAnimationPlugin);
```

Note: `MapRuntimePlugin` includes this automatically.

## License

MIT OR Apache-2.0
