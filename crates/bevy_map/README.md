# bevy_map

Complete 2D tilemap editor and runtime for Bevy games.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_map = "0.1"
```

Load and display a map:

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

## Features

- `runtime` (default) - Map loading and bevy_ecs_tilemap rendering
- `physics` - Avian2D collision integration
- `hot-reload` (default) - File watching for development

```toml
# With physics support
bevy_map = { version = "0.1", features = ["physics"] }
```

## Modules

| Module      | Description                          |
|-------------|--------------------------------------|
| `prelude`   | Common imports for most use cases    |
| `core`      | Data types (Level, Layer, Tileset)   |
| `runtime`   | Runtime loading and rendering        |
| `animation` | Sprite sheet animations              |
| `dialogue`  | Branching dialogue trees             |
| `autotile`  | Wang tile terrain system             |

## Custom Entities

Define game entities that spawn from map data:

```rust
use bevy::prelude::*;
use bevy_map::prelude::*;

#[derive(Component, MapEntity)]
#[map_entity(type_name = "NPC")]
pub struct Npc {
    #[map_prop]
    pub name: String,
    #[map_prop(default = 100)]
    pub health: i32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapRuntimePlugin)
        .register_map_entity::<Npc>()
        .run();
}
```

## License

MIT OR Apache-2.0
