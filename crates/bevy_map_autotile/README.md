# bevy_map_autotile

Tiled-like terrain autotiling using Wang tiles.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

## Features

- **Corner Mode** - 16-tile terrains for smooth ground transitions
- **Edge Mode** - 16-tile terrains for linear features (walls, paths)
- **Mixed Mode** - 48-tile terrains for full coverage
- Similar to Tiled's Wang tile system

## Terrain Set Types

| Type   | Tiles | Use Case                            |
|--------|-------|-------------------------------------|
| Corner | 16    | Ground transitions (grass/dirt)     |
| Edge   | 16    | Linear features (walls, rivers)     |
| Mixed  | 48    | Full terrain with corners and edges |

## Usage

```rust
use bevy_map::autotile::{TerrainSet, TerrainSetType, Terrain, WangFiller};

// Create a terrain set
let mut terrain_set = TerrainSet::new(
    "Ground Transitions",
    tileset_id,
    TerrainSetType::Corner,
);

// Add terrains (each terrain maps to tiles in the tileset)
terrain_set.add_terrain(Terrain::new("Grass", 0));
terrain_set.add_terrain(Terrain::new("Dirt", 1));

// Fill tiles automatically based on neighbors
let filler = WangFiller::new(&terrain_set);
filler.fill_area(&mut level, layer_index, terrain_index, x, y, width, height);
```

## How It Works

The autotile system examines neighboring tiles to determine which tile variant to place:

```
Corner Mode (16 tiles):
┌───┬───┬───┐    Each corner can be terrain A or B
│ A │ A │ B │    giving 2^4 = 16 combinations
├───┼───┼───┤
│ A │ ? │ B │
├───┼───┼───┤
│ A │ B │ B │
└───┴───┴───┘
```

## Integration with Editor

The editor provides a terrain palette for painting with autotile support. Terrains are configured per-tileset and stored in the project file.

## License

MIT OR Apache-2.0
