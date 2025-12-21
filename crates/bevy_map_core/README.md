# bevy_map_core

Core data types for the bevy_map_editor ecosystem.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Types

| Type             | Description                                                    |
|------------------|----------------------------------------------------------------|
| `MapProject`     | Complete project with levels, tilesets, dialogues, animations  |
| `Level`          | Single map level with layers and entities                      |
| `Layer`          | Tile or object layer within a level                            |
| `Tileset`        | Tileset definition with multi-image support                    |
| `TilesetImage`   | Individual image within a tileset                              |
| `EntityInstance` | Placed entity with position and properties                     |
| `Value`          | Dynamic property value (String, Int, Float, Bool, Color, etc.) |

## Usage

```rust
use bevy_map::core::{Level, Layer, LayerData, Tileset};

// Create a level
let mut level = Level::new("My Level", 32, 32);

// Add a tile layer
let layer = Layer::new_tile_layer("Ground", tileset_id, 32, 32);
level.layers.push(layer);

// Set tiles
level.set_tile(0, 0, 0, Some(1)); // layer 0, x=0, y=0, tile index 1
```

## MapProject Structure

```rust
pub struct MapProject {
    pub version: u32,
    pub schema: Schema,
    pub tilesets: Vec<Tileset>,
    pub levels: Vec<Level>,
    pub sprite_sheets: Vec<SpriteData>,
    pub dialogues: Vec<DialogueTree>,
    pub autotile_config: AutotileConfig,
}
```

## License

MIT OR Apache-2.0
