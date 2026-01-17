# bevy_map_runtime

Runtime map loading and rendering for Bevy 0.17 using bevy_ecs_tilemap.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Features

- Efficient tilemap rendering via bevy_ecs_tilemap 0.17
- Asset-based map loading with hot reload support
- Custom entity spawning with `#[derive(MapEntity)]`
- Autoloading for animations and dialogues
- **Collision integration** with Avian2D physics (optional `physics` feature)
- Runtime tile modification

## Quick Start

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_systems(Startup, load_map)
        .run();
}

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(MapHandle(asset_server.load("maps/level.map.json")));
}
```

## Custom Entities

Register entity types to spawn game objects from map data:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

#[derive(Component, MapEntity)]
#[map_entity(type_name = "Chest")]
pub struct Chest {
    #[map_prop]
    pub loot_table: String,
    #[map_prop(default = false)]
    pub locked: bool,
}

// Register in your app builder
App::new()
    .add_plugins(MapRuntimePlugin)
    .register_map_entity::<Chest>()
    // ... other setup
```

## Auto-Loading Animations

Use `AnimatedSpriteHandle` to autoload sprite animations from a map project:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::AnimatedSpriteHandle;

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AnimatedSpriteHandle::new(
            asset_server.load("maps/game.map.json"),
            "player_sprite",
            "idle",  // animation name defined in editor
        ),
        Transform::default(),
    ));
}
```

## Autoloading Dialogues

Use `DialogueTreeHandle` to auto-load dialogues:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::DialogueTreeHandle;

fn spawn_npc(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DialogueTreeHandle::new(
        asset_server.load("maps/game.map.json"),
        "merchant_greeting",  // dialogue name defined in editor
    ));
}
```

## Manual Animation Control

For direct control over sprites, use `#[map_sprite]`:

```rust
#[derive(Component, MapEntity)]
#[map_entity(type_name = "Player")]
pub struct Player {
    #[map_prop]
    pub speed: f32,

    #[map_sprite("player_sprite")]
    pub sprite: Option<Handle<Image>>,
}
```

## Collision Integration (Avian2D)

Enable the `physics` feature to automatically spawn colliders from tile collision data:

```toml
bevy_map = { version = "0.1", features = ["physics"] }
```

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::MapCollisionPlugin;
use avian2d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)  // Auto-spawns Avian2D colliders!
        .add_plugins(PhysicsPlugins::default())
        .add_systems(Startup, load_map)
        .run();
}

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    // Collisions are spawned automatically from tile properties!
    commands.spawn(MapHandle(asset_server.load("maps/level.map.json")));
}
```

The `MapCollisionPlugin` reads collision shapes defined in the tileset editor and spawns corresponding Avian2D `Collider` components. Query `MapCollider` to access original collision data.

## Entity Type Components (Zero-Code Physics & Input)

Configure physics, input, and sprites at the **entity type level** in the editor - no Rust code needed!

### Automatic Physics

With `MapEntityPhysicsPlugin`, entities spawn with physics components based on their type configuration:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::{MapEntityPhysicsPlugin, MapCollisionPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)       // Tile colliders
        .add_plugins(MapEntityPhysicsPlugin)   // Entity physics from type config!
        .add_systems(Startup, load_map)
        .run();
}
```

In the Schema Editor, go to the "Components" tab and configure:
- **Body Type**: Dynamic, Kinematic, or Static
- **Collider**: Box, Capsule, or Circle with custom dimensions
- **Physics properties**: Gravity scale, friction, restitution, etc.

### Automatic Input

With `MapEntityInputPlugin`, entities respond to input based on their type configuration:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::{MapEntityInputPlugin, MapEntityPhysicsPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapEntityPhysicsPlugin)   // Physics for movement
        .add_plugins(MapEntityInputPlugin)     // Input from type config!
        .add_systems(Startup, load_map)
        .run();
}
```

Built-in input profiles:
- **Platformer**: A/D movement, Space to jump
- **Top-Down**: WASD 8-directional movement
- **Twin-Stick**: WASD move, mouse aim (marker only)
- **Custom**: Add your own systems that query the marker component

### Automatic Sprites

With `MapEntitySpritePlugin`, entities spawn with sprites from type configuration:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::MapEntitySpritePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapEntitySpritePlugin)    // Sprites from type config!
        .add_systems(Startup, load_map)
        .run();
}
```

In the Schema Editor Components tab, select a sprite sheet and default animation.

### Full Zero-Code Setup

For a complete platformer with no entity code:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;
use bevy_map::runtime::{MapCollisionPlugin, MapEntityPhysicsPlugin, MapEntityInputPlugin, MapEntitySpritePlugin};
use avian2d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .add_plugins(MapCollisionPlugin)        // Tile colliders
        .add_plugins(MapEntityPhysicsPlugin)    // Entity physics
        .add_plugins(MapEntityInputPlugin)      // Entity input
        .add_plugins(MapEntitySpritePlugin)     // Entity sprites
        .insert_resource(Gravity(Vec2::new(0.0, -800.0)))
        .add_systems(Startup, load_map)
        .run();
}

fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(MapHandle(asset_server.load("maps/level.map.json")));
    // Player, NPCs, crates - all configured in editor, no code needed!
}
```

## Re-exported Types

For convenience, use `bevy_map::prelude::*` which includes all commonly used types:

```rust
use bevy_map::prelude::*;
// Includes: MapRuntimePlugin, MapHandle, MapEntity, AnimatedSprite,
// DialogueRunner, DialogueTree, and more
```

## License

MIT OR Apache-2.0
