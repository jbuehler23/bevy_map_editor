//! Map project - bundles a level with its tileset metadata for easier loading
//!
//! The MapProject provides a self-contained format that includes both the level
//! data and all tileset information needed to render it, so developers don't have
//! to manually map tileset IDs to textures.
//!
//! Two formats are supported:
//! - `MapProject`: Simple format with HashMap collections (for hand-crafted JSON)
//! - `EditorProject`: Full editor format with array collections (exported by the editor)

use crate::{EntityTypeConfig, Level, Tileset};
use bevy_map_animation::SpriteData;
use bevy_map_dialogue::DialogueTree;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Editor project format - matches what the editor exports
///
/// Use this type to load `.map.json` files created by the editor.
/// It uses arrays for collections (tilesets, levels, sprite_sheets, dialogues).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorProject {
    /// Format version
    pub version: u32,
    /// Schema information (optional, for editor use)
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    /// Tilesets as array
    #[serde(default)]
    pub tilesets: Vec<Tileset>,
    /// Data section (optional, for editor use)
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Levels as array (editor supports multiple levels)
    #[serde(default)]
    pub levels: Vec<Level>,
    /// Autotile configuration (optional)
    #[serde(default)]
    pub autotile_config: Option<serde_json::Value>,
    /// Sprite sheets as array
    #[serde(default)]
    pub sprite_sheets: Vec<SpriteData>,
    /// Dialogues as array
    #[serde(default)]
    pub dialogues: Vec<DialogueTree>,
    /// Entity type component configurations (physics, input, sprite per type)
    #[serde(default)]
    pub entity_type_configs: HashMap<String, EntityTypeConfig>,
}

impl EditorProject {
    /// Get the first level (most common case)
    pub fn first_level(&self) -> Option<&Level> {
        self.levels.first()
    }

    /// Get the first sprite sheet
    pub fn first_sprite_sheet(&self) -> Option<&SpriteData> {
        self.sprite_sheets.first()
    }

    /// Get a sprite sheet by name
    pub fn sprite_sheet_by_name(&self, name: &str) -> Option<&SpriteData> {
        self.sprite_sheets.iter().find(|s| s.name == name)
    }

    /// Get the first dialogue tree
    pub fn first_dialogue(&self) -> Option<&DialogueTree> {
        self.dialogues.first()
    }

    /// Get a dialogue by name
    pub fn dialogue_by_name(&self, name: &str) -> Option<&DialogueTree> {
        self.dialogues.iter().find(|d| d.name == name)
    }

    /// Get a tileset by ID
    pub fn tileset_by_id(&self, id: Uuid) -> Option<&Tileset> {
        self.tilesets.iter().find(|t| t.id == id)
    }

    /// Convert to MapProject (uses first level)
    pub fn to_map_project(&self) -> Option<MapProject> {
        let level = self.first_level()?.clone();
        let tilesets = self.tilesets.iter().map(|t| (t.id, t.clone())).collect();
        let sprite_sheets = self
            .sprite_sheets
            .iter()
            .map(|s| (s.id, s.clone()))
            .collect();
        let dialogues = self
            .dialogues
            .iter()
            .map(|d| (d.id.to_string(), d.clone()))
            .collect();

        Some(MapProject {
            version: self.version,
            level,
            tilesets,
            sprite_sheets,
            dialogues,
            entity_type_configs: self.entity_type_configs.clone(),
        })
    }

    /// Get entity type config by type name
    pub fn get_entity_type_config(&self, type_name: &str) -> Option<&EntityTypeConfig> {
        self.entity_type_configs.get(type_name)
    }
}

/// A self-contained map project that includes level data and tileset metadata
///
/// This format is designed for the editor-to-runtime workflow, embedding
/// all tileset information needed to load and render a level.
///
/// # Example JSON
/// ```json
/// {
///   "version": 1,
///   "level": { ... },
///   "tilesets": { "uuid": { ... } }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bevy", derive(bevy::asset::Asset, bevy::reflect::TypePath))]
pub struct MapProject {
    /// Format version for future compatibility
    pub version: u32,
    /// The level data
    pub level: Level,
    /// Tilesets used by this level, keyed by their UUID
    pub tilesets: HashMap<Uuid, Tileset>,
    /// Sprite sheets used by this level, keyed by their UUID
    #[serde(default)]
    pub sprite_sheets: HashMap<Uuid, SpriteData>,
    /// Dialogue trees used by this level, keyed by their ID
    #[serde(default)]
    pub dialogues: HashMap<String, DialogueTree>,
    /// Entity type component configurations (physics, input, sprite per type)
    #[serde(default)]
    pub entity_type_configs: HashMap<String, EntityTypeConfig>,
}

impl MapProject {
    /// Create a new map project from a level and its tilesets
    pub fn new(level: Level, tilesets: Vec<Tileset>) -> Self {
        let tileset_map = tilesets.into_iter().map(|t| (t.id, t)).collect();
        Self {
            version: 1,
            level,
            tilesets: tileset_map,
            sprite_sheets: HashMap::new(),
            dialogues: HashMap::new(),
            entity_type_configs: HashMap::new(),
        }
    }

    /// Create a new map project with sprite sheets
    pub fn new_with_sprite_sheets(
        level: Level,
        tilesets: Vec<Tileset>,
        sprite_sheets: Vec<SpriteData>,
    ) -> Self {
        let tileset_map = tilesets.into_iter().map(|t| (t.id, t)).collect();
        let sprite_sheet_map = sprite_sheets.into_iter().map(|s| (s.id, s)).collect();
        Self {
            version: 1,
            level,
            tilesets: tileset_map,
            sprite_sheets: sprite_sheet_map,
            dialogues: HashMap::new(),
            entity_type_configs: HashMap::new(),
        }
    }

    /// Get entity type config by type name
    pub fn get_entity_type_config(&self, type_name: &str) -> Option<&EntityTypeConfig> {
        self.entity_type_configs.get(type_name)
    }

    /// Get a sprite sheet by ID
    pub fn get_sprite_sheet(&self, id: Uuid) -> Option<&SpriteData> {
        self.sprite_sheets.get(&id)
    }

    /// Get a sprite sheet by name
    pub fn sprite_sheet_by_name(&self, name: &str) -> Option<&SpriteData> {
        self.sprite_sheets.values().find(|s| s.name == name)
    }

    /// Get all sprite sheet image paths
    pub fn sprite_sheet_paths(&self) -> Vec<(Uuid, &str)> {
        self.sprite_sheets
            .iter()
            .filter(|(_, s)| !s.sheet_path.is_empty())
            .map(|(id, s)| (*id, s.sheet_path.as_str()))
            .collect()
    }

    /// Get a dialogue tree by ID
    pub fn get_dialogue(&self, id: &str) -> Option<&DialogueTree> {
        self.dialogues.get(id)
    }

    /// Get all dialogue IDs
    pub fn dialogue_ids(&self) -> impl Iterator<Item = &str> {
        self.dialogues.keys().map(|s| s.as_str())
    }

    /// Get a dialogue by name (searches by DialogueTree.name field)
    pub fn dialogue_by_name(&self, name: &str) -> Option<&DialogueTree> {
        self.dialogues.values().find(|d| d.name == name)
    }

    /// Get a tileset by ID
    pub fn get_tileset(&self, id: Uuid) -> Option<&Tileset> {
        self.tilesets.get(&id)
    }

    /// Get all unique tileset IDs used by tile layers in this level
    pub fn used_tileset_ids(&self) -> Vec<Uuid> {
        use crate::LayerData;
        let mut ids: Vec<Uuid> = self
            .level
            .layers
            .iter()
            .filter_map(|layer| {
                if let LayerData::Tiles { tileset_id, .. } = &layer.data {
                    Some(*tileset_id)
                } else {
                    None
                }
            })
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Get all image paths needed to render this level
    ///
    /// Returns a list of (tileset_id, image_index, path) tuples for each
    /// tileset image that needs to be loaded.
    pub fn image_paths(&self) -> Vec<(Uuid, usize, &str)> {
        let mut paths = Vec::new();
        for tileset_id in self.used_tileset_ids() {
            if let Some(tileset) = self.tilesets.get(&tileset_id) {
                for (idx, image) in tileset.images.iter().enumerate() {
                    paths.push((tileset_id, idx, image.path.as_str()));
                }
                // Handle legacy single-image format
                if tileset.images.is_empty() {
                    if let Some(path) = &tileset.path {
                        paths.push((tileset_id, 0, path.as_str()));
                    }
                }
            }
        }
        paths
    }

    /// Validate that all tileset references in the level are satisfied
    pub fn validate(&self) -> Result<(), String> {
        use crate::LayerData;
        for (layer_idx, layer) in self.level.layers.iter().enumerate() {
            if let LayerData::Tiles { tileset_id, .. } = &layer.data {
                if !self.tilesets.contains_key(tileset_id) {
                    return Err(format!(
                        "Layer {} references missing tileset {}",
                        layer_idx, tileset_id
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Builder for creating a MapProject from separate level and tileset sources
#[derive(Debug, Default)]
pub struct MapProjectBuilder {
    level: Option<Level>,
    tilesets: Vec<Tileset>,
}

impl MapProjectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = Some(level);
        self
    }

    pub fn tileset(mut self, tileset: Tileset) -> Self {
        self.tilesets.push(tileset);
        self
    }

    pub fn tilesets(mut self, tilesets: impl IntoIterator<Item = Tileset>) -> Self {
        self.tilesets.extend(tilesets);
        self
    }

    pub fn build(self) -> Result<MapProject, &'static str> {
        let level = self.level.ok_or("Level is required")?;
        Ok(MapProject::new(level, self.tilesets))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Layer;

    #[test]
    fn test_map_project_creation() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let tileset = Tileset::new("Ground".to_string(), "tiles.png".to_string(), 32, 10, 10);
        let tileset_id = tileset.id;

        level.add_layer(Layer::new_tile_layer(
            "Ground".to_string(),
            tileset_id,
            10,
            10,
        ));

        let project = MapProject::new(level, vec![tileset]);

        assert_eq!(project.version, 1);
        assert!(project.get_tileset(tileset_id).is_some());
        assert!(project.validate().is_ok());
    }

    #[test]
    fn test_used_tileset_ids() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let tileset1 = Tileset::new("Ground".to_string(), "ground.png".to_string(), 32, 10, 10);
        let tileset2 = Tileset::new("Objects".to_string(), "objects.png".to_string(), 32, 8, 8);

        level.add_layer(Layer::new_tile_layer(
            "Ground".to_string(),
            tileset1.id,
            10,
            10,
        ));
        level.add_layer(Layer::new_tile_layer(
            "Objects".to_string(),
            tileset2.id,
            10,
            10,
        ));

        let project = MapProject::new(level, vec![tileset1.clone(), tileset2.clone()]);
        let used_ids = project.used_tileset_ids();

        assert_eq!(used_ids.len(), 2);
        assert!(used_ids.contains(&tileset1.id));
        assert!(used_ids.contains(&tileset2.id));
    }

    #[test]
    fn test_image_paths() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let tileset = Tileset::new(
            "Ground".to_string(),
            "tiles/ground.png".to_string(),
            32,
            10,
            10,
        );
        let tileset_id = tileset.id;

        level.add_layer(Layer::new_tile_layer(
            "Ground".to_string(),
            tileset_id,
            10,
            10,
        ));

        let project = MapProject::new(level, vec![tileset]);
        let paths = project.image_paths();

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].2, "tiles/ground.png");
    }

    #[test]
    fn test_validation_fails_missing_tileset() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let missing_tileset_id = Uuid::new_v4();

        level.add_layer(Layer::new_tile_layer(
            "Ground".to_string(),
            missing_tileset_id,
            10,
            10,
        ));

        let project = MapProject::new(level, vec![]);
        assert!(project.validate().is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let level = Level::new("Test".to_string(), 10, 10);
        let tileset = Tileset::new("Ground".to_string(), "tiles.png".to_string(), 32, 10, 10);

        let project = MapProjectBuilder::new()
            .level(level)
            .tileset(tileset)
            .build()
            .unwrap();

        assert_eq!(project.level.name, "Test");
        assert_eq!(project.tilesets.len(), 1);
    }
}
