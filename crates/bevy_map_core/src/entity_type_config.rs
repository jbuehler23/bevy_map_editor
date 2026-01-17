//! Entity type component configurations
//!
//! This module provides configuration structures for entity type-level components
//! like physics, input, and sprite settings. These are configured once per entity type
//! in the schema editor, not per instance.
//!
//! # Usage
//!
//! Entity types (e.g., "Player", "NPC", "Crate") can have component configurations
//! that define how all instances of that type behave:
//!
//! - **PhysicsConfig**: Collider shape, body type (dynamic/static/kinematic)
//! - **InputConfig**: Input profile (platformer, top-down, etc.) and movement parameters
//! - **SpriteConfig**: Default sprite sheet and animation for this entity type

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for an entity type's automatic components
///
/// This is stored at the type level (in the schema) and applies to all
/// instances of that entity type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityTypeConfig {
    /// Physics component configuration (collider, body type, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physics: Option<PhysicsConfig>,

    /// Input component configuration (movement profile, speed, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<InputConfig>,

    /// Sprite/animation configuration (sprite sheet, default animation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SpriteConfig>,
}

impl EntityTypeConfig {
    /// Create an empty entity type config
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any component is configured
    pub fn has_any(&self) -> bool {
        self.physics.is_some() || self.input.is_some() || self.sprite.is_some()
    }
}

// ============================================================================
// Physics Configuration
// ============================================================================

/// Physics body type for entity colliders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PhysicsBodyType {
    /// Affected by gravity and forces (e.g., player, crate)
    Dynamic,
    /// Moved by code, not affected by forces (e.g., moving platform)
    Kinematic,
    /// Immovable (e.g., wall, ground)
    #[default]
    Static,
}

impl PhysicsBodyType {
    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            PhysicsBodyType::Dynamic => "Dynamic",
            PhysicsBodyType::Kinematic => "Kinematic",
            PhysicsBodyType::Static => "Static",
        }
    }

    /// All available body types for UI dropdowns
    pub fn all() -> &'static [PhysicsBodyType] {
        &[
            PhysicsBodyType::Dynamic,
            PhysicsBodyType::Kinematic,
            PhysicsBodyType::Static,
        ]
    }
}

/// Collider shape configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ColliderConfig {
    /// Box/rectangle collider
    Box {
        /// Width in pixels
        width: f32,
        /// Height in pixels
        height: f32,
    },
    /// Capsule collider (rounded ends)
    Capsule {
        /// Width of the capsule (diameter)
        width: f32,
        /// Total height including rounded ends
        height: f32,
    },
    /// Circle collider
    Circle {
        /// Radius in pixels
        radius: f32,
    },
}

impl Default for ColliderConfig {
    fn default() -> Self {
        ColliderConfig::Box {
            width: 16.0,
            height: 16.0,
        }
    }
}

impl ColliderConfig {
    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ColliderConfig::Box { .. } => "Box",
            ColliderConfig::Capsule { .. } => "Capsule",
            ColliderConfig::Circle { .. } => "Circle",
        }
    }

    /// Get the variant type name (for UI dropdown)
    pub fn variant_name(&self) -> &'static str {
        match self {
            ColliderConfig::Box { .. } => "box",
            ColliderConfig::Capsule { .. } => "capsule",
            ColliderConfig::Circle { .. } => "circle",
        }
    }

    /// Create a new box collider with given dimensions
    pub fn new_box(width: f32, height: f32) -> Self {
        ColliderConfig::Box { width, height }
    }

    /// Create a new capsule collider with given dimensions
    pub fn new_capsule(width: f32, height: f32) -> Self {
        ColliderConfig::Capsule { width, height }
    }

    /// Create a new circle collider with given radius
    pub fn new_circle(radius: f32) -> Self {
        ColliderConfig::Circle { radius }
    }
}

/// Physics component configuration for an entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Type of physics body (Dynamic, Kinematic, Static)
    #[serde(default)]
    pub body_type: PhysicsBodyType,

    /// Collider shape and dimensions
    #[serde(default)]
    pub collider: ColliderConfig,

    /// Gravity scale multiplier (1.0 = normal, 0.0 = no gravity)
    #[serde(default = "default_gravity_scale")]
    pub gravity_scale: f32,

    /// Lock rotation (prevent entity from rotating)
    #[serde(default)]
    pub lock_rotation: bool,

    /// Linear damping (air resistance)
    #[serde(default)]
    pub linear_damping: f32,

    /// Friction coefficient (0.0-1.0)
    #[serde(default = "default_friction")]
    pub friction: f32,

    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    #[serde(default)]
    pub restitution: f32,
}

fn default_gravity_scale() -> f32 {
    1.0
}

fn default_friction() -> f32 {
    0.5
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            body_type: PhysicsBodyType::Dynamic,
            collider: ColliderConfig::default(),
            gravity_scale: 1.0,
            lock_rotation: true,
            linear_damping: 0.0,
            friction: 0.5,
            restitution: 0.0,
        }
    }
}

// ============================================================================
// Input Configuration
// ============================================================================

/// Input profile presets for common game types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputProfile {
    /// Platformer controls: A/D for horizontal movement, Space to jump
    #[default]
    Platformer,
    /// Top-down 8-directional movement: WASD
    TopDown,
    /// Twin-stick: WASD move, mouse aim
    TwinStick,
    /// Custom profile defined by name (for user-provided systems)
    Custom {
        /// Name of the custom profile
        name: String,
    },
    /// No input handling (for NPCs, static objects, etc.)
    None,
}

impl InputProfile {
    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            InputProfile::Platformer => "Platformer",
            InputProfile::TopDown => "Top-Down",
            InputProfile::TwinStick => "Twin-Stick",
            InputProfile::Custom { .. } => "Custom",
            InputProfile::None => "None",
        }
    }

    /// All available built-in profiles for UI dropdowns
    pub fn all_builtin() -> &'static [InputProfile] {
        &[
            InputProfile::Platformer,
            InputProfile::TopDown,
            InputProfile::TwinStick,
            InputProfile::None,
        ]
    }

    /// Get the variant type name (for UI dropdown)
    pub fn variant_name(&self) -> &'static str {
        match self {
            InputProfile::Platformer => "platformer",
            InputProfile::TopDown => "topdown",
            InputProfile::TwinStick => "twinstick",
            InputProfile::Custom { .. } => "custom",
            InputProfile::None => "none",
        }
    }
}

/// Input component configuration for an entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// The input profile to use
    #[serde(default)]
    pub profile: InputProfile,

    /// Movement speed in pixels per second
    #[serde(default = "default_speed")]
    pub speed: f32,

    /// Jump force (for Platformer profile)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump_force: Option<f32>,

    /// Maximum fall speed (terminal velocity)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fall_speed: Option<f32>,

    /// Acceleration time (0 = instant, higher = smoother)
    #[serde(default)]
    pub acceleration: f32,

    /// Deceleration time (0 = instant stop, higher = slide)
    #[serde(default)]
    pub deceleration: f32,
}

fn default_speed() -> f32 {
    200.0
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            profile: InputProfile::Platformer,
            speed: 200.0,
            jump_force: Some(400.0),
            max_fall_speed: Some(600.0),
            acceleration: 0.0,
            deceleration: 0.0,
        }
    }
}

impl InputConfig {
    /// Create a platformer input config with default values
    pub fn platformer() -> Self {
        Self::default()
    }

    /// Create a top-down input config
    pub fn top_down() -> Self {
        Self {
            profile: InputProfile::TopDown,
            speed: 200.0,
            jump_force: None,
            max_fall_speed: None,
            acceleration: 0.0,
            deceleration: 0.0,
        }
    }

    /// Create a config with no input (for NPCs, etc.)
    pub fn none() -> Self {
        Self {
            profile: InputProfile::None,
            speed: 0.0,
            jump_force: None,
            max_fall_speed: None,
            acceleration: 0.0,
            deceleration: 0.0,
        }
    }
}

// ============================================================================
// Sprite Configuration
// ============================================================================

/// Sprite/animation configuration for an entity type
///
/// This configures the default sprite sheet and animation for all instances
/// of this entity type. Individual instances can still override properties.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpriteConfig {
    /// UUID of the sprite sheet to use
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_sheet_id: Option<Uuid>,

    /// Name of the default animation to play (e.g., "idle")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_animation: Option<String>,

    /// Optional scale multiplier for the sprite
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,

    /// Offset from entity position (for centering sprites)
    #[serde(default, skip_serializing_if = "is_zero_vec")]
    pub offset: [f32; 2],

    /// Flip sprite horizontally based on movement direction
    #[serde(default)]
    pub flip_with_direction: bool,
}

fn is_zero_vec(v: &[f32; 2]) -> bool {
    v[0] == 0.0 && v[1] == 0.0
}

impl SpriteConfig {
    /// Create a sprite config with a sprite sheet reference
    pub fn with_sprite_sheet(sprite_sheet_id: Uuid) -> Self {
        Self {
            sprite_sheet_id: Some(sprite_sheet_id),
            ..Default::default()
        }
    }

    /// Set the default animation
    pub fn with_animation(mut self, animation: &str) -> Self {
        self.default_animation = Some(animation.to_string());
        self
    }

    /// Set the scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_config_serialization() {
        let config = EntityTypeConfig {
            physics: Some(PhysicsConfig {
                body_type: PhysicsBodyType::Dynamic,
                collider: ColliderConfig::Capsule {
                    width: 14.0,
                    height: 24.0,
                },
                gravity_scale: 1.0,
                lock_rotation: true,
                linear_damping: 0.0,
                friction: 0.5,
                restitution: 0.0,
            }),
            input: Some(InputConfig {
                profile: InputProfile::Platformer,
                speed: 200.0,
                jump_force: Some(400.0),
                max_fall_speed: Some(600.0),
                acceleration: 0.0,
                deceleration: 0.0,
            }),
            sprite: Some(SpriteConfig {
                sprite_sheet_id: Some(Uuid::new_v4()),
                default_animation: Some("idle".to_string()),
                scale: Some(2.0),
                offset: [0.0, 0.0],
                flip_with_direction: true,
            }),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{}", json);

        let deserialized: EntityTypeConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.physics.is_some());
        assert!(deserialized.input.is_some());
        assert!(deserialized.sprite.is_some());
    }

    #[test]
    fn test_physics_body_type() {
        assert_eq!(PhysicsBodyType::Dynamic.display_name(), "Dynamic");
        assert_eq!(PhysicsBodyType::all().len(), 3);
    }

    #[test]
    fn test_input_profile() {
        assert_eq!(InputProfile::Platformer.display_name(), "Platformer");
        assert_eq!(InputProfile::all_builtin().len(), 4);
    }

    #[test]
    fn test_collider_config() {
        let box_collider = ColliderConfig::new_box(16.0, 32.0);
        assert_eq!(box_collider.display_name(), "Box");

        let capsule = ColliderConfig::new_capsule(14.0, 24.0);
        assert_eq!(capsule.display_name(), "Capsule");

        let circle = ColliderConfig::new_circle(8.0);
        assert_eq!(circle.display_name(), "Circle");
    }
}
