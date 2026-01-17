//! Project management for the map editor
//!
//! This module handles project file save/load and the Project resource.

mod file;

pub use file::*;

use bevy::prelude::Resource;
use bevy_map_animation::SpriteData;
use bevy_map_autotile::AutotileConfig;
use bevy_map_core::{EntityTypeConfig, Level, Tileset, WorldConfig};
use bevy_map_dialogue::DialogueTree;
use bevy_map_schema::Schema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// A saved tile stamp (reusable tile arrangement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileStamp {
    /// Unique identifier for this stamp
    pub id: Uuid,
    /// User-friendly name
    pub name: String,
    /// Width of the stamp in tiles
    pub width: u32,
    /// Height of the stamp in tiles
    pub height: u32,
    /// The tileset this stamp uses
    pub tileset_id: Uuid,
    /// Tile data in row-major order (None = empty cell)
    /// Values include flip flags encoded in the tile index
    pub tiles: Vec<Option<u32>>,
}

impl TileStamp {
    /// Create a new stamp with the given name and dimensions
    pub fn new(name: String, width: u32, height: u32, tileset_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            width,
            height,
            tileset_id,
            tiles: vec![None; (width * height) as usize],
        }
    }

    /// Get tile at position (returns None if out of bounds or empty)
    pub fn get_tile(&self, x: u32, y: u32) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y * self.width + x) as usize;
        self.tiles.get(idx).copied().flatten()
    }
}

/// Configuration for an associated game project
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameProjectConfig {
    /// Path to the game project directory (contains Cargo.toml)
    pub project_path: Option<PathBuf>,
    /// ID of the level to load when the game starts
    pub starting_level: Option<Uuid>,
    /// Project name (used for Cargo.toml package name)
    pub project_name: String,
    /// Whether to use release mode when building
    pub use_release_build: bool,
}

/// The entire editor project
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct Project {
    pub version: u32,
    #[serde(skip)]
    pub path: Option<PathBuf>,
    #[serde(skip)]
    pub schema_path: Option<PathBuf>,
    pub schema: Schema,
    pub tilesets: Vec<Tileset>,
    pub data: DataStore,
    pub levels: Vec<Level>,
    /// Autotile terrain configuration
    #[serde(default)]
    pub autotile_config: AutotileConfig,
    /// Sprite sheet assets (reusable sprite/animation definitions)
    #[serde(default, alias = "animations")]
    pub sprite_sheets: Vec<SpriteData>,
    /// Dialogue tree assets
    #[serde(default)]
    pub dialogues: Vec<DialogueTree>,
    /// World configuration (layout mode, connections)
    #[serde(default)]
    pub world_config: WorldConfig,
    /// Saved tile stamps (reusable tile arrangements)
    #[serde(default)]
    pub stamps: Vec<TileStamp>,
    /// Associated game project configuration
    #[serde(default)]
    pub game_config: GameProjectConfig,
    /// Entity type component configurations (physics, input, sprite per type)
    #[serde(default)]
    pub entity_type_configs: HashMap<String, EntityTypeConfig>,
    #[serde(skip)]
    pub dirty: bool,

    // Performance indices - O(1) lookups instead of O(n) iter().find()
    #[serde(skip)]
    level_index: HashMap<Uuid, usize>,
    #[serde(skip)]
    tileset_index: HashMap<Uuid, usize>,
    #[serde(skip)]
    sprite_sheet_index: HashMap<Uuid, usize>,
    #[serde(skip)]
    dialogue_index: HashMap<String, usize>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: 1,
            path: None,
            schema_path: None,
            schema: Schema::default(),
            tilesets: Vec::new(),
            data: DataStore::default(),
            levels: Vec::new(),
            autotile_config: AutotileConfig::default(),
            sprite_sheets: Vec::new(),
            dialogues: Vec::new(),
            world_config: WorldConfig::default(),
            stamps: Vec::new(),
            game_config: GameProjectConfig::default(),
            entity_type_configs: HashMap::new(),
            dirty: false,
            level_index: HashMap::new(),
            tileset_index: HashMap::new(),
            sprite_sheet_index: HashMap::new(),
            dialogue_index: HashMap::new(),
        }
    }
}

impl Project {
    pub fn new(schema: Schema) -> Self {
        Self {
            version: 1,
            path: None,
            schema_path: None,
            schema,
            tilesets: Vec::new(),
            data: DataStore::default(),
            levels: Vec::new(),
            autotile_config: AutotileConfig::default(),
            sprite_sheets: Vec::new(),
            dialogues: Vec::new(),
            world_config: WorldConfig::default(),
            stamps: Vec::new(),
            game_config: GameProjectConfig::default(),
            entity_type_configs: HashMap::new(),
            dirty: false,
            level_index: HashMap::new(),
            tileset_index: HashMap::new(),
            sprite_sheet_index: HashMap::new(),
            dialogue_index: HashMap::new(),
        }
    }

    /// Get entity type config by type name
    pub fn get_entity_type_config(&self, type_name: &str) -> Option<&EntityTypeConfig> {
        self.entity_type_configs.get(type_name)
    }

    /// Get mutable entity type config by type name
    pub fn get_entity_type_config_mut(&mut self, type_name: &str) -> Option<&mut EntityTypeConfig> {
        self.dirty = true;
        self.entity_type_configs.get_mut(type_name)
    }

    /// Set entity type config for a type
    pub fn set_entity_type_config(&mut self, type_name: &str, config: EntityTypeConfig) {
        self.entity_type_configs
            .insert(type_name.to_string(), config);
        self.dirty = true;
    }

    /// Remove entity type config for a type
    pub fn remove_entity_type_config(&mut self, type_name: &str) -> Option<EntityTypeConfig> {
        self.dirty = true;
        self.entity_type_configs.remove(type_name)
    }

    /// Ensure a type has an entity type config (creates default if missing)
    pub fn ensure_entity_type_config(&mut self, type_name: &str) -> &mut EntityTypeConfig {
        if !self.entity_type_configs.contains_key(type_name) {
            self.entity_type_configs
                .insert(type_name.to_string(), EntityTypeConfig::default());
            self.dirty = true;
        }
        self.entity_type_configs.get_mut(type_name).unwrap()
    }

    /// Rebuild all lookup indices. Call after loading or bulk modifications.
    pub fn rebuild_indices(&mut self) {
        self.level_index.clear();
        for (idx, level) in self.levels.iter().enumerate() {
            self.level_index.insert(level.id, idx);
        }

        self.tileset_index.clear();
        for (idx, tileset) in self.tilesets.iter().enumerate() {
            self.tileset_index.insert(tileset.id, idx);
        }

        self.sprite_sheet_index.clear();
        for (idx, sprite_sheet) in self.sprite_sheets.iter().enumerate() {
            self.sprite_sheet_index.insert(sprite_sheet.id, idx);
        }

        self.dialogue_index.clear();
        for (idx, dialogue) in self.dialogues.iter().enumerate() {
            self.dialogue_index.insert(dialogue.id.clone(), idx);
        }
    }

    /// Mark project as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if project has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get project name (from path or schema)
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.schema.project.name.clone())
    }

    /// Add a new data instance
    pub fn add_data_instance(&mut self, instance: DataInstance) {
        self.data.add(instance);
        self.dirty = true;
    }

    /// Remove a data instance by ID
    pub fn remove_data_instance(&mut self, id: Uuid) -> Option<DataInstance> {
        let result = self.data.remove(id);
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    /// Get data instance by ID
    pub fn get_data_instance(&self, id: Uuid) -> Option<&DataInstance> {
        self.data.get(id)
    }

    /// Get mutable data instance by ID
    pub fn get_data_instance_mut(&mut self, id: Uuid) -> Option<&mut DataInstance> {
        let result = self.data.get_mut(id);
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    /// Count entities of a given type across all levels
    pub fn count_entities_of_type(&self, type_name: &str) -> usize {
        self.levels
            .iter()
            .map(|level| {
                level
                    .entities
                    .iter()
                    .filter(|e| e.type_name == type_name)
                    .count()
            })
            .sum()
    }

    /// Add a new level
    pub fn add_level(&mut self, level: Level) {
        let id = level.id;
        let idx = self.levels.len();
        self.levels.push(level);
        self.level_index.insert(id, idx);
        self.dirty = true;
    }

    /// Get level by ID (O(1) lookup)
    pub fn get_level(&self, id: Uuid) -> Option<&Level> {
        self.level_index
            .get(&id)
            .and_then(|&idx| self.levels.get(idx))
    }

    /// Get mutable level by ID (O(1) lookup)
    pub fn get_level_mut(&mut self, id: Uuid) -> Option<&mut Level> {
        self.dirty = true;
        self.level_index
            .get(&id)
            .copied()
            .and_then(|idx| self.levels.get_mut(idx))
    }

    /// Get tileset by ID (O(1) lookup)
    pub fn get_tileset(&self, id: Uuid) -> Option<&Tileset> {
        self.tileset_index
            .get(&id)
            .and_then(|&idx| self.tilesets.get(idx))
    }

    /// Get mutable tileset by ID (O(1) lookup)
    pub fn get_tileset_mut(&mut self, id: Uuid) -> Option<&mut Tileset> {
        self.dirty = true;
        self.tileset_index
            .get(&id)
            .copied()
            .and_then(|idx| self.tilesets.get_mut(idx))
    }

    /// Add a new tileset
    pub fn add_tileset(&mut self, tileset: Tileset) {
        let id = tileset.id;
        let idx = self.tilesets.len();
        self.tilesets.push(tileset);
        self.tileset_index.insert(id, idx);
        self.dirty = true;
    }

    /// Remove a tileset by ID
    pub fn remove_tileset(&mut self, id: Uuid) -> Option<Tileset> {
        if let Some(&idx) = self.tileset_index.get(&id) {
            self.tileset_index.remove(&id);
            let removed = self.tilesets.remove(idx);
            // Rebuild indices after removal (indices shifted)
            self.tileset_index.clear();
            for (i, tileset) in self.tilesets.iter().enumerate() {
                self.tileset_index.insert(tileset.id, i);
            }
            self.dirty = true;
            Some(removed)
        } else {
            None
        }
    }

    /// Duplicate a data instance by ID, returns the new instance's ID
    pub fn duplicate_data_instance(&mut self, id: Uuid) -> Option<Uuid> {
        let original = self.data.get(id)?.clone();
        let mut duplicate = original;
        duplicate.id = Uuid::new_v4();

        // Append " (Copy)" to the name if there's a name property
        if let Some(bevy_map_core::Value::String(name)) = duplicate.properties.get_mut("name") {
            name.push_str(" (Copy)");
        }

        let new_id = duplicate.id;
        self.data.add(duplicate);
        self.dirty = true;
        Some(new_id)
    }

    /// Remove a level by ID
    pub fn remove_level(&mut self, id: Uuid) -> Option<Level> {
        if let Some(&idx) = self.level_index.get(&id) {
            self.level_index.remove(&id);
            let removed = self.levels.remove(idx);
            // Rebuild indices after removal (indices shifted)
            self.level_index.clear();
            for (i, level) in self.levels.iter().enumerate() {
                self.level_index.insert(level.id, i);
            }
            self.dirty = true;
            Some(removed)
        } else {
            None
        }
    }

    /// Duplicate a level by ID, returns the new level's ID
    pub fn duplicate_level(&mut self, id: Uuid) -> Option<Uuid> {
        let original = self.get_level(id)?.clone();
        let mut duplicate = original;
        duplicate.id = Uuid::new_v4();
        duplicate.name = format!("{} (Copy)", duplicate.name);

        // Also assign new IDs to all entities
        for entity in &mut duplicate.entities {
            entity.id = Uuid::new_v4();
        }

        let new_id = duplicate.id;
        let idx = self.levels.len();
        self.levels.push(duplicate);
        self.level_index.insert(new_id, idx);
        self.dirty = true;
        Some(new_id)
    }

    // Sprite sheet methods

    /// Add a new sprite sheet asset
    pub fn add_sprite_sheet(&mut self, sprite_sheet: SpriteData) {
        let id = sprite_sheet.id;
        let idx = self.sprite_sheets.len();
        self.sprite_sheets.push(sprite_sheet);
        self.sprite_sheet_index.insert(id, idx);
        self.dirty = true;
    }

    /// Get a sprite sheet by ID (O(1) lookup)
    pub fn get_sprite_sheet(&self, id: Uuid) -> Option<&SpriteData> {
        self.sprite_sheet_index
            .get(&id)
            .and_then(|&idx| self.sprite_sheets.get(idx))
    }

    /// Get mutable sprite sheet by ID (O(1) lookup)
    pub fn get_sprite_sheet_mut(&mut self, id: Uuid) -> Option<&mut SpriteData> {
        self.dirty = true;
        self.sprite_sheet_index
            .get(&id)
            .copied()
            .and_then(|idx| self.sprite_sheets.get_mut(idx))
    }

    /// Remove a sprite sheet by ID
    pub fn remove_sprite_sheet(&mut self, id: Uuid) -> Option<SpriteData> {
        if let Some(&idx) = self.sprite_sheet_index.get(&id) {
            self.sprite_sheet_index.remove(&id);
            let removed = self.sprite_sheets.remove(idx);
            // Rebuild indices after removal
            self.sprite_sheet_index.clear();
            for (i, ss) in self.sprite_sheets.iter().enumerate() {
                self.sprite_sheet_index.insert(ss.id, i);
            }
            self.dirty = true;
            Some(removed)
        } else {
            None
        }
    }

    // Dialogue methods

    /// Add a new dialogue tree
    pub fn add_dialogue(&mut self, dialogue: DialogueTree) {
        let id = dialogue.id.clone();
        let idx = self.dialogues.len();
        self.dialogues.push(dialogue);
        self.dialogue_index.insert(id, idx);
        self.dirty = true;
    }

    /// Get a dialogue by ID (O(1) lookup)
    pub fn get_dialogue(&self, id: &str) -> Option<&DialogueTree> {
        self.dialogue_index
            .get(id)
            .and_then(|&idx| self.dialogues.get(idx))
    }

    /// Get mutable dialogue by ID (O(1) lookup)
    pub fn get_dialogue_mut(&mut self, id: &str) -> Option<&mut DialogueTree> {
        self.dirty = true;
        self.dialogue_index
            .get(id)
            .copied()
            .and_then(|idx| self.dialogues.get_mut(idx))
    }

    /// Remove a dialogue by ID
    pub fn remove_dialogue(&mut self, id: &str) -> Option<DialogueTree> {
        if let Some(&idx) = self.dialogue_index.get(id) {
            self.dialogue_index.remove(id);
            let removed = self.dialogues.remove(idx);
            // Rebuild indices after removal
            self.dialogue_index.clear();
            for (i, d) in self.dialogues.iter().enumerate() {
                self.dialogue_index.insert(d.id.clone(), i);
            }
            self.dirty = true;
            Some(removed)
        } else {
            None
        }
    }

    /// Validate and clean up orphaned references in the project
    ///
    /// This removes terrain sets that reference non-existent tilesets,
    /// which can happen if a tileset was deleted before cascade delete was implemented.
    pub fn validate_and_cleanup(&mut self) {
        use std::collections::HashSet;

        let valid_tileset_ids: HashSet<Uuid> = self.tilesets.iter().map(|t| t.id).collect();

        // Remove terrain sets that reference non-existent tilesets
        let original_count = self.autotile_config.terrain_sets.len();
        self.autotile_config.terrain_sets.retain(|ts| {
            let exists = valid_tileset_ids.contains(&ts.tileset_id);
            if !exists {
                bevy::log::warn!(
                    "Removing orphaned terrain set '{}' - tileset {} no longer exists",
                    ts.name,
                    ts.tileset_id
                );
            }
            exists
        });

        let removed = original_count - self.autotile_config.terrain_sets.len();
        if removed > 0 {
            bevy::log::info!(
                "Cleaned up {} orphaned terrain set(s) from project",
                removed
            );
            self.dirty = true;
        }
    }
}

/// A data instance (non-placeable thing like an Item, Quest, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataInstance {
    pub id: Uuid,
    pub type_name: String,
    pub properties: HashMap<String, bevy_map_core::Value>,
}

impl DataInstance {
    pub fn new(type_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            type_name,
            properties: HashMap::new(),
        }
    }
}

/// Stores all data_type instances (non-placeable things like Items, Quests)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataStore {
    /// Key: type name (e.g., "Item", "Quest")
    /// Value: list of instances of that type
    pub instances: HashMap<String, Vec<DataInstance>>,
}

impl DataStore {
    pub fn add(&mut self, instance: DataInstance) {
        self.instances
            .entry(instance.type_name.clone())
            .or_default()
            .push(instance);
    }

    pub fn remove(&mut self, id: Uuid) -> Option<DataInstance> {
        for instances in self.instances.values_mut() {
            if let Some(pos) = instances.iter().position(|i| i.id == id) {
                return Some(instances.remove(pos));
            }
        }
        None
    }

    pub fn get(&self, id: Uuid) -> Option<&DataInstance> {
        for instances in self.instances.values() {
            if let Some(instance) = instances.iter().find(|i| i.id == id) {
                return Some(instance);
            }
        }
        None
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut DataInstance> {
        for instances in self.instances.values_mut() {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == id) {
                return Some(instance);
            }
        }
        None
    }

    pub fn get_by_type(&self, type_name: &str) -> &[DataInstance] {
        self.instances
            .get(type_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn all_instances(&self) -> impl Iterator<Item = &DataInstance> {
        self.instances.values().flatten()
    }
}
