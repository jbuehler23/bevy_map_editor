//! # bevy_map
//!
//! Complete 2D tilemap editor and runtime for Bevy games.
//!
//! This crate provides everything you need to load and render tile-based maps
//! created with bevy_map_editor.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(MapRuntimePlugin)
//!         .add_systems(Startup, load_map)
//!         .run();
//! }
//!
//! fn load_map(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     // Load map with hot-reload support
//!     commands.spawn(MapHandle(asset_server.load("maps/level1.map.json")));
//! }
//! ```
//!
//! ## Features
//!
//! - `runtime` (default) - Includes map loading and bevy_ecs_tilemap rendering
//! - `physics` - Adds avian2d collision support
//! - `hot-reload` (default) - File watching for development
//!
//! ## Crate Structure
//!
//! This umbrella crate re-exports all bevy_map_* sub-crates:
//!
//! - [`core`] - Core data types (Level, Layer, Tileset, etc.)
//! - [`animation`] - Sprite animation system
//! - [`dialogue`] - Branching dialogue trees
//! - [`autotile`] - Terrain auto-tiling (Wang tiles)
//! - [`runtime`] - Map loading and rendering (requires `runtime` feature)

// =============================================================================
// Re-export derive macro at top level for ergonomics
// =============================================================================
pub use bevy_map_derive::MapEntity;

// =============================================================================
// Core module - fundamental data structures
// =============================================================================

/// Core data types for representing tile-based maps.
///
/// This module provides the fundamental types:
/// - [`Level`] - A complete map with layers and entities
/// - [`Layer`] - A single layer (tiles or objects)
/// - [`Tileset`] - Tile atlas configuration
/// - [`EntityInstance`] - Placed entities with properties
/// - [`MapProject`] - Self-contained map format
pub mod core {
    pub use bevy_map_core::*;
}

// Core type re-exports at crate root
pub use bevy_map_core::{
    CollisionData, CollisionShape, EditorProject, EntityInstance, Layer, LayerData, LayerType,
    Level, MapProject, MapProjectBuilder, OneWayDirection, PhysicsBody, TileProperties, Tileset,
    TilesetImage, Value, OCCUPIED_CELL,
};

// =============================================================================
// Animation module - sprite animation system
// =============================================================================

/// Sprite animation system with frame-based animations.
///
/// Provides:
/// - [`SpriteData`] - Sprite sheet definitions
/// - [`AnimationDef`] - Animation configurations
/// - [`AnimatedSprite`] - Runtime animation component
/// - [`SpriteAnimationPlugin`] - Bevy plugin for animation
pub mod animation {
    pub use bevy_map_animation::*;
}

pub use bevy_map_animation::{
    AnimatedSprite, AnimationCustomEvent, AnimationDef, AnimationEventExt, AnimationParticleEvent,
    AnimationSoundEvent, AnimationTrigger, AnimationTriggerEvent, AnimationTriggerRegistry,
    AnimationTriggerType, AnimationTriggered, AnimationWindow, AnimationWindowChanged,
    AnimationWindowEvent, AnimationWindowRegistry, AnimationWindowType, LoopMode,
    SpriteAnimationPlugin, SpriteData, TriggerPayload, WindowPhase, WindowTracker,
};

// =============================================================================
// Dialogue module - branching dialogue trees
// =============================================================================

/// Branching dialogue tree system.
///
/// Provides:
/// - [`DialogueTree`] - Complete dialogue definition
/// - [`DialogueNode`] - Individual dialogue nodes
/// - [`DialogueRunner`] - Runtime dialogue state
/// - [`DialoguePlugin`] - Bevy plugin for dialogue
pub mod dialogue {
    pub use bevy_map_dialogue::*;
}

pub use bevy_map_dialogue::{
    DialogueChoice, DialogueChoiceEvent, DialogueEndEvent, DialogueHandle, DialogueNode,
    DialogueNodeType, DialoguePlugin, DialogueRunner, DialogueTree, StartDialogueEvent,
};

// =============================================================================
// Autotile module - terrain auto-tiling
// =============================================================================

/// Tiled-compatible terrain auto-tiling system.
///
/// Provides:
/// - [`TerrainSet`] - Terrain definitions for a tileset
/// - [`WangFiller`] - Automatic tile selection algorithm
/// - [`paint_terrain`] - Runtime terrain painting
pub mod autotile {
    pub use bevy_map_autotile::*;
}

pub use bevy_map_autotile::{
    apply_autotile_to_region, calculate_bitmask, erase_autotile, neighbors, optimize_bitmask,
    paint_autotile, paint_terrain, paint_terrain_at_target, update_tile_with_neighbors,
    AutotileConfig, CellInfo, Color, LegacyTerrainType, PaintTarget, Terrain, TerrainBrush,
    TerrainId, TerrainSet, TerrainSetType, TerrainType, TileConstraints, TileTerrainData,
    WangFiller, WangId, WangPosition,
};

// =============================================================================
// Schema module - entity validation
// =============================================================================

/// Schema validation for entity types.
pub mod schema {
    pub use bevy_map_schema::*;
}

// =============================================================================
// Runtime module - map loading and rendering (optional)
// =============================================================================

/// Runtime map loading and rendering via bevy_ecs_tilemap.
///
/// This module provides:
/// - [`MapRuntimePlugin`] - Main plugin for map loading
/// - [`MapHandle`] - Component for asset-based map loading
/// - [`spawn_map_project`] - Manual map spawning
/// - [`EntityRegistry`] - Custom entity type registration
///
/// Requires the `runtime` feature (enabled by default).
#[cfg(feature = "runtime")]
pub mod runtime {
    pub use bevy_map_runtime::*;
}

#[cfg(feature = "runtime")]
pub use bevy_map_runtime::{
    attach_dialogues, complete_sprite_loads, spawn_map_project, spawn_sprite_components, Dialogue,
    EntityProperties, EntityRegistry, MapCollider, MapCollisionPlugin, MapEntityExt,
    MapEntityMarker, MapEntityType, MapHandle, MapLoadError, MapProjectLoader, MapRoot,
    MapRuntimePlugin, MapSpawnedEvent, SpawnMapEvent, SpawnMapProjectEvent, SpriteSlot,
    TilesetTextures,
};

// =============================================================================
// Prelude - import everything commonly needed
// =============================================================================

/// Commonly used types and traits.
///
/// Import with:
/// ```rust,ignore
/// use bevy_map::prelude::*;
/// ```
pub mod prelude {
    // Derive macro
    pub use crate::MapEntity;

    // Core types
    pub use crate::{
        CollisionData, CollisionShape, EntityInstance, Layer, LayerData, Level, MapProject,
        Tileset, Value,
    };

    // Animation
    pub use crate::{AnimatedSprite, AnimationDef, LoopMode, SpriteAnimationPlugin, SpriteData};

    // Dialogue
    pub use crate::{
        DialogueChoice, DialogueChoiceEvent, DialogueEndEvent, DialogueHandle, DialogueNode,
        DialoguePlugin, DialogueRunner, DialogueTree, StartDialogueEvent,
    };

    // Autotile
    pub use crate::{AutotileConfig, Terrain, TerrainSet, TerrainSetType};

    // Runtime (if enabled)
    #[cfg(feature = "runtime")]
    pub use crate::{
        spawn_map_project, EntityRegistry, MapEntityExt, MapHandle, MapRoot, MapRuntimePlugin,
        SpawnMapEvent, SpawnMapProjectEvent, TilesetTextures,
    };
}
