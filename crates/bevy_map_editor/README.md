# bevy_map_editor

Visual map editor for Bevy 0.17 games. Create tilemaps, place entities, design dialogue trees, and define animations.

Part of [bevy_map_editor](https://github.com/jbuehler23/bevy_map_editor).

<!-- TODO: Add screenshot -->
![Editor Screenshot](../../docs/images/editor_screenshot.png)

## Features

- Project management (new, open, save, recent projects)
- Preferences with auto-save (persisted to user config directory)
- Auto-open last project on startup
- Multi-level support with hierarchical view
- Layer system (tile and object layers)
- Tileset management with multi-image support
- Terrain painting with autotiling
- Entity placement and property editing
- Dialogue tree editor with visual node graph
- Animation/sprite sheet editor
- Undo/redo support
- Keyboard shortcuts

### Run Game & Live Editing

Launch and test your game directly from the editor:

- **Run Game** - Build and launch your game with one click
- **Live editing** - Edit maps while the game runs (non-blocking)
- **Auto-sync** - Save (Ctrl+S) automatically syncs to running game
- **Hot-reload** - Changes appear instantly via Bevy's file watcher
- **Game Settings** - Configure game project path and starting level

<!-- TODO: Add GIF -->
![Run Game](../../docs/gifs/run_game.gif)

### Tile Flipping

- Press **X** to toggle horizontal flip while painting
- Press **Y** to toggle vertical flip while painting
- Tiled-compatible flip flags exported in map files

### Stamps

- **Ctrl+Shift+S** - Create reusable stamp from tile selection
- Access saved stamps from Stamp Library panel

## Installation (Standalone Binary)

### From crates.io

```bash
cargo install bevy_map_editor
```

### From source

```bash
git clone https://github.com/jbuehler23/bevy_map_editor
cd bevy_map_editor
cargo install --path crates/bevy_map_editor
```

### Run the editor

```bash
bevy_map_editor
```

## Usage (As a Library)

```rust
use bevy::prelude::*;
use bevy_map_editor::EditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EditorPlugin)
        .run();
}
```

## Feature Flags

| Flag      | Description                                                  |
|-----------|--------------------------------------------------------------|
| `runtime` | Enable viewport rendering via bevy_ecs_tilemap (recommended) |

```toml
[dependencies]
bevy_map_editor = { version = "0.1", features = ["runtime"] }
```

## UI Panels

| Panel           | Purpose                                                    |
|-----------------|------------------------------------------------------------|
| Menu Bar        | File, Edit, View, Project, Tools, Help menus               |
| Toolbar         | Tool selection (Select, Paint, Erase, Fill, Entity)        |
| Project Tree    | Hierarchical view of levels, layers, dialogues, animations |
| Inspector       | Property editing for selected items                        |
| Terrain Palette | Terrain set and terrain selection for autotiling           |
| Tileset Panel   | Tile selection from loaded tilesets                        |
| Viewport        | Map preview and editing canvas                             |
| Settings Dialog | Preferences for startup, view defaults, and tools          |

## Keyboard Shortcuts

| Shortcut       | Action                        |
|----------------|-------------------------------|
| `Ctrl+N`       | New Project                   |
| `Ctrl+O`       | Open Project                  |
| `Ctrl+S`       | Save (+ sync if game running) |
| `Ctrl+Shift+S` | Create Stamp from Selection   |
| `Ctrl+Z`       | Undo                          |
| `Ctrl+Y`       | Redo                          |
| `Ctrl+C`       | Copy                          |
| `Ctrl+V`       | Paste                         |
| `Ctrl+X`       | Cut                           |
| `G`            | Toggle Grid                   |
| `X`            | Toggle Horizontal Flip        |
| `Y`            | Toggle Vertical Flip          |
| `W`            | Toggle World View             |
| `L`            | Switch to Level View          |

## License

MIT OR Apache-2.0
