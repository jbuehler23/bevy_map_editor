//! Level/map containing tiles and entities

use crate::{EntityInstance, Layer, LayerData};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A level/map containing tiles and entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub id: Uuid,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub entities: Vec<EntityInstance>,
    /// World X position in pixels (for world view)
    #[serde(default)]
    pub world_x: i32,
    /// World Y position in pixels (for world view)
    #[serde(default)]
    pub world_y: i32,
    /// Background color for world view (hex format, e.g., "#3C3C50")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<String>,
}

impl Level {
    /// Create a new empty level
    pub fn new(name: String, width: u32, height: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            width,
            height,
            layers: Vec::new(),
            entities: Vec::new(),
            world_x: 0,
            world_y: 0,
            bg_color: None,
        }
    }

    /// Create a new level at a specific world position
    pub fn new_at(name: String, width: u32, height: u32, world_x: i32, world_y: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            width,
            height,
            layers: Vec::new(),
            entities: Vec::new(),
            world_x,
            world_y,
            bg_color: None,
        }
    }

    /// Set the world position
    pub fn set_world_position(&mut self, x: i32, y: i32) {
        self.world_x = x;
        self.world_y = y;
    }

    /// Get the world position as a tuple
    pub fn world_position(&self) -> (i32, i32) {
        (self.world_x, self.world_y)
    }

    /// Add a new layer
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Add an entity to the level
    pub fn add_entity(&mut self, entity: EntityInstance) {
        self.entities.push(entity);
    }

    /// Remove an entity by ID
    pub fn remove_entity(&mut self, id: Uuid) -> Option<EntityInstance> {
        self.entities
            .iter()
            .position(|e| e.id == id)
            .map(|pos| self.entities.remove(pos))
    }

    /// Get entity by ID
    pub fn get_entity(&self, id: Uuid) -> Option<&EntityInstance> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Get mutable entity by ID
    pub fn get_entity_mut(&mut self, id: Uuid) -> Option<&mut EntityInstance> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Get tile at position for a specific layer
    pub fn get_tile(&self, layer_index: usize, x: u32, y: u32) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        if let Some(layer) = self.layers.get(layer_index) {
            if let LayerData::Tiles { tiles, .. } = &layer.data {
                let index = (y * self.width + x) as usize;
                return tiles.get(index).copied().flatten();
            }
        }
        None
    }

    /// Set tile at position for a specific layer
    pub fn set_tile(&mut self, layer_index: usize, x: u32, y: u32, tile: Option<u32>) {
        if x >= self.width || y >= self.height {
            return;
        }
        if let Some(layer) = self.layers.get_mut(layer_index) {
            if let LayerData::Tiles { tiles, .. } = &mut layer.data {
                let index = (y * self.width + x) as usize;
                if index < tiles.len() {
                    tiles[index] = tile;
                }
            }
        }
    }

    /// Remove a layer by index
    pub fn remove_layer(&mut self, index: usize) -> Option<Layer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Move a layer up (toward index 0)
    pub fn move_layer_up(&mut self, index: usize) -> bool {
        if index > 0 && index < self.layers.len() {
            self.layers.swap(index, index - 1);
            true
        } else {
            false
        }
    }

    /// Move a layer down (toward higher index)
    pub fn move_layer_down(&mut self, index: usize) -> bool {
        if index < self.layers.len().saturating_sub(1) {
            self.layers.swap(index, index + 1);
            true
        } else {
            false
        }
    }

    /// Toggle layer visibility
    pub fn toggle_layer_visibility(&mut self, index: usize) -> bool {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.visible = !layer.visible;
            true
        } else {
            false
        }
    }

    /// Get layer by index
    pub fn get_layer(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    /// Get mutable layer by index
    pub fn get_layer_mut(&mut self, index: usize) -> Option<&mut Layer> {
        self.layers.get_mut(index)
    }

    /// Get all tiles in a region (for undo/redo snapshots)
    pub fn get_tiles_in_region(
        &self,
        layer_index: usize,
        min_x: u32,
        max_x: u32,
        min_y: u32,
        max_y: u32,
    ) -> Vec<((u32, u32), Option<u32>)> {
        let mut tiles = Vec::new();
        let min_x = min_x.min(self.width.saturating_sub(1));
        let max_x = max_x.min(self.width.saturating_sub(1));
        let min_y = min_y.min(self.height.saturating_sub(1));
        let max_y = max_y.min(self.height.saturating_sub(1));

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let tile = self.get_tile(layer_index, x, y);
                tiles.push(((x, y), tile));
            }
        }
        tiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_level() {
        let level = Level::new("Test Level".to_string(), 10, 10);
        assert_eq!(level.name, "Test Level");
        assert_eq!(level.width, 10);
        assert_eq!(level.height, 10);
        assert!(level.layers.is_empty());
        assert!(level.entities.is_empty());
    }

    #[test]
    fn test_tile_operations() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let tileset_id = Uuid::new_v4();
        level.add_layer(Layer::new_tile_layer(
            "Ground".to_string(),
            tileset_id,
            10,
            10,
        ));

        // Initially empty
        assert_eq!(level.get_tile(0, 5, 5), None);

        // Set a tile
        level.set_tile(0, 5, 5, Some(42));
        assert_eq!(level.get_tile(0, 5, 5), Some(42));

        // Clear a tile
        level.set_tile(0, 5, 5, None);
        assert_eq!(level.get_tile(0, 5, 5), None);
    }

    #[test]
    fn test_entity_operations() {
        let mut level = Level::new("Test".to_string(), 10, 10);
        let entity = EntityInstance::new("NPC".to_string(), [100.0, 100.0]);
        let entity_id = entity.id;

        level.add_entity(entity);
        assert!(level.get_entity(entity_id).is_some());

        let removed = level.remove_entity(entity_id);
        assert!(removed.is_some());
        assert!(level.get_entity(entity_id).is_none());
    }
}
