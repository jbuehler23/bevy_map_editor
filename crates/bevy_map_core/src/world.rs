//! World configuration for multi-level projects
//!
//! This module provides types for organizing multiple levels in a world view,
//! inspired by LDtk's world layout system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// World layout mode determining how levels are organized
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorldLayout {
    /// Levels positioned freely in 2D space
    #[default]
    Free,
    /// Levels snap to a world grid (like LDtk's GridVania)
    GridVania,
    /// Levels arranged horizontally in sequence
    LinearHorizontal,
    /// Levels arranged vertically in sequence
    LinearVertical,
}

impl WorldLayout {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            WorldLayout::Free => "Free",
            WorldLayout::GridVania => "Grid-vania",
            WorldLayout::LinearHorizontal => "Linear (Horizontal)",
            WorldLayout::LinearVertical => "Linear (Vertical)",
        }
    }

    /// Returns all layout variants for UI enumeration
    pub fn all() -> &'static [WorldLayout] {
        &[
            WorldLayout::Free,
            WorldLayout::GridVania,
            WorldLayout::LinearHorizontal,
            WorldLayout::LinearVertical,
        ]
    }
}

/// Direction/edge of a level connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionDirection {
    /// Top edge (exit goes up)
    #[default]
    North,
    /// Bottom edge (exit goes down)
    South,
    /// Right edge (exit goes right)
    East,
    /// Left edge (exit goes left)
    West,
}

impl ConnectionDirection {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ConnectionDirection::North => "North",
            ConnectionDirection::South => "South",
            ConnectionDirection::East => "East",
            ConnectionDirection::West => "West",
        }
    }

    /// Get short name for UI (single letter)
    pub fn short_name(&self) -> &'static str {
        match self {
            ConnectionDirection::North => "N",
            ConnectionDirection::South => "S",
            ConnectionDirection::East => "E",
            ConnectionDirection::West => "W",
        }
    }

    /// Get the opposite direction
    pub fn opposite(&self) -> ConnectionDirection {
        match self {
            ConnectionDirection::North => ConnectionDirection::South,
            ConnectionDirection::South => ConnectionDirection::North,
            ConnectionDirection::East => ConnectionDirection::West,
            ConnectionDirection::West => ConnectionDirection::East,
        }
    }

    /// Returns all direction variants for UI enumeration
    pub fn all() -> &'static [ConnectionDirection] {
        &[
            ConnectionDirection::North,
            ConnectionDirection::South,
            ConnectionDirection::East,
            ConnectionDirection::West,
        ]
    }
}

/// A connection/exit point between two levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelConnection {
    /// Unique ID for this connection
    pub id: Uuid,
    /// Source level ID
    pub from_level: Uuid,
    /// Which edge of the source level the exit is on
    pub from_direction: ConnectionDirection,
    /// Target level ID
    pub to_level: Uuid,
    /// Which edge of the target level the entrance is on
    pub to_direction: ConnectionDirection,
}

impl LevelConnection {
    /// Create a new connection between two levels with specified directions
    pub fn new(
        from_level: Uuid,
        from_direction: ConnectionDirection,
        to_level: Uuid,
        to_direction: ConnectionDirection,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_level,
            from_direction,
            to_level,
            to_direction,
        }
    }

    /// Create a connection with automatic opposite direction
    /// (e.g., East exit from level A leads to West entrance of level B)
    pub fn auto_direction(
        from_level: Uuid,
        from_direction: ConnectionDirection,
        to_level: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_level,
            from_direction,
            to_level,
            to_direction: from_direction.opposite(),
        }
    }
}

/// World configuration stored in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Layout mode for level organization
    pub layout: WorldLayout,
    /// Grid cell width in pixels (for GridVania mode)
    pub grid_width: u32,
    /// Grid cell height in pixels (for GridVania mode)
    pub grid_height: u32,
    /// Connections between levels
    #[serde(default)]
    pub connections: Vec<LevelConnection>,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            layout: WorldLayout::Free,
            grid_width: 256,
            grid_height: 256,
            connections: Vec::new(),
        }
    }
}

impl WorldConfig {
    /// Create a new world config with the specified layout
    pub fn new(layout: WorldLayout) -> Self {
        Self {
            layout,
            ..Default::default()
        }
    }

    /// Create a GridVania world config with custom grid size
    pub fn gridvania(grid_width: u32, grid_height: u32) -> Self {
        Self {
            layout: WorldLayout::GridVania,
            grid_width,
            grid_height,
            connections: Vec::new(),
        }
    }

    /// Add a connection between two levels
    pub fn add_connection(&mut self, connection: LevelConnection) {
        self.connections.push(connection);
    }

    /// Remove a connection by ID
    pub fn remove_connection(&mut self, id: Uuid) -> Option<LevelConnection> {
        self.connections
            .iter()
            .position(|c| c.id == id)
            .map(|idx| self.connections.remove(idx))
    }

    /// Get connections originating from a specific level
    pub fn connections_from(&self, level_id: Uuid) -> Vec<&LevelConnection> {
        self.connections
            .iter()
            .filter(|c| c.from_level == level_id)
            .collect()
    }

    /// Get connections leading to a specific level
    pub fn connections_to(&self, level_id: Uuid) -> Vec<&LevelConnection> {
        self.connections
            .iter()
            .filter(|c| c.to_level == level_id)
            .collect()
    }

    /// Get all connections involving a specific level (as source or destination)
    pub fn connections_for(&self, level_id: Uuid) -> Vec<&LevelConnection> {
        self.connections
            .iter()
            .filter(|c| c.from_level == level_id || c.to_level == level_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_layout_display_names() {
        assert_eq!(WorldLayout::Free.display_name(), "Free");
        assert_eq!(WorldLayout::GridVania.display_name(), "Grid-vania");
    }

    #[test]
    fn test_world_config_default() {
        let config = WorldConfig::default();
        assert_eq!(config.layout, WorldLayout::Free);
        assert_eq!(config.grid_width, 256);
        assert_eq!(config.grid_height, 256);
        assert!(config.connections.is_empty());
    }

    #[test]
    fn test_level_connection() {
        let level_a = Uuid::new_v4();
        let level_b = Uuid::new_v4();

        let connection = LevelConnection::new(
            level_a,
            ConnectionDirection::East,
            level_b,
            ConnectionDirection::West,
        );
        assert_eq!(connection.from_level, level_a);
        assert_eq!(connection.from_direction, ConnectionDirection::East);
        assert_eq!(connection.to_level, level_b);
        assert_eq!(connection.to_direction, ConnectionDirection::West);
    }

    #[test]
    fn test_connection_auto_direction() {
        let level_a = Uuid::new_v4();
        let level_b = Uuid::new_v4();

        let connection =
            LevelConnection::auto_direction(level_a, ConnectionDirection::East, level_b);
        assert_eq!(connection.from_direction, ConnectionDirection::East);
        assert_eq!(connection.to_direction, ConnectionDirection::West);
    }

    #[test]
    fn test_connection_management() {
        let mut config = WorldConfig::default();
        let level_a = Uuid::new_v4();
        let level_b = Uuid::new_v4();

        let connection =
            LevelConnection::auto_direction(level_a, ConnectionDirection::East, level_b);
        let connection_id = connection.id;

        config.add_connection(connection);
        assert_eq!(config.connections.len(), 1);

        let from_a = config.connections_from(level_a);
        assert_eq!(from_a.len(), 1);

        config.remove_connection(connection_id);
        assert!(config.connections.is_empty());
    }
}
