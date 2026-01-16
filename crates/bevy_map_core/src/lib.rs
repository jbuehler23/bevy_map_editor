//! Core data structures for bevy_map_editor
//!
//! This crate provides the fundamental types for representing tile-based maps:
//! - `Level` - A complete map with layers and entities
//! - `Layer` - A single layer (tiles or objects)
//! - `Tileset` - Tile atlas configuration with multi-image support
//! - `EntityInstance` - Placed entities with properties
//! - `Value` - Generic property value type
//! - `MapProject` - Self-contained format bundling level and tilesets

mod collision;
mod entity;
mod layer;
mod level;
mod project;
mod tileset;
mod value;
mod world;

pub use collision::{CollisionData, CollisionShape, OneWayDirection, PhysicsBody};
pub use entity::EntityInstance;
pub use layer::{
    tile_flip_x, tile_flip_y, tile_index, tile_with_flips, toggle_flip_x, toggle_flip_y, Layer,
    LayerData, LayerType, OCCUPIED_CELL, TILE_FLIP_MASK, TILE_FLIP_X, TILE_FLIP_Y, TILE_INDEX_MASK,
};
pub use level::Level;
pub use project::{EditorProject, MapProject, MapProjectBuilder};
pub use tileset::{TileProperties, Tileset, TilesetImage};
pub use value::Value;
pub use world::{ConnectionDirection, LevelConnection, WorldConfig, WorldLayout};
