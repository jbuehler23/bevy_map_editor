//! Schema type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The schema loaded from schema.json - defines all types and enums
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub version: u32,
    pub project: ProjectConfig,
    #[serde(default)]
    pub enums: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub data_types: HashMap<String, TypeDef>,
    #[serde(default)]
    pub embedded_types: HashMap<String, TypeDef>,
}

impl Schema {
    /// Get a type definition by name (checks data_types and embedded_types)
    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.data_types
            .get(name)
            .or_else(|| self.embedded_types.get(name))
    }

    /// Get enum values by name
    pub fn get_enum(&self, name: &str) -> Option<&Vec<String>> {
        self.enums.get(name)
    }

    /// Get all type names sorted alphabetically
    pub fn all_type_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.data_types.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get all data type names sorted alphabetically
    pub fn data_type_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.data_types.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get all placeable type names (types that can be placed in levels)
    pub fn placeable_type_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .data_types
            .iter()
            .filter(|(_, def)| def.placeable)
            .map(|(name, _)| name.as_str())
            .collect();
        names.sort();
        names
    }
}

/// Project-level configuration from schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_tile_size")]
    pub tile_size: u32,
    #[serde(default)]
    pub default_layer_types: Vec<String>,
}

fn default_tile_size() -> u32 {
    32
}

/// Definition of a type (from schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    #[serde(default = "default_color")]
    pub color: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub placeable: bool,
    /// Marker size in pixels for rendering on canvas (default: 16)
    #[serde(default)]
    pub marker_size: Option<u32>,
    #[serde(default)]
    pub properties: Vec<PropertyDef>,
}

fn default_color() -> String {
    "#808080".to_string()
}

impl Default for TypeDef {
    fn default() -> Self {
        Self {
            color: default_color(),
            icon: None,
            placeable: false,
            marker_size: None,
            properties: Vec::new(),
        }
    }
}

/// Definition of a property (from schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDef {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: PropType,
    #[serde(default)]
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    #[serde(rename = "showIf")]
    pub show_if: Option<String>,
    #[serde(rename = "enumType")]
    pub enum_type: Option<String>,
    #[serde(rename = "refType")]
    pub ref_type: Option<String>,
    #[serde(rename = "itemType")]
    pub item_type: Option<String>,
    #[serde(rename = "embeddedType")]
    pub embedded_type: Option<String>,
}

/// Property types supported by the schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropType {
    String,
    Multiline,
    Int,
    Float,
    Bool,
    Enum,
    Ref,
    Array,
    Embedded,
    Point,
    Color,
    /// DEPRECATED: Use SpriteConfig in EntityTypeConfig instead.
    /// Sprite configuration should be at the entity type level, not per-instance.
    /// This variant is kept for backwards compatibility with existing projects.
    #[deprecated(
        since = "0.3.0",
        note = "Use SpriteConfig in EntityTypeConfig instead. Sprite configuration should be at the entity type level."
    )]
    Sprite,
    Dialogue,
}

impl PropType {
    pub fn display_name(&self) -> &'static str {
        #[allow(deprecated)]
        match self {
            PropType::String => "String",
            PropType::Multiline => "Multiline",
            PropType::Int => "Integer",
            PropType::Float => "Float",
            PropType::Bool => "Boolean",
            PropType::Enum => "Enum",
            PropType::Ref => "Reference",
            PropType::Array => "Array",
            PropType::Embedded => "Embedded",
            PropType::Point => "Point",
            PropType::Color => "Color",
            PropType::Sprite => "Sprite (Deprecated)",
            PropType::Dialogue => "Dialogue Tree",
        }
    }

    /// Check if this property type is deprecated
    #[allow(deprecated)]
    pub fn is_deprecated(&self) -> bool {
        matches!(self, PropType::Sprite)
    }

    /// Get all non-deprecated property types for UI display
    pub fn all_active() -> &'static [PropType] {
        &[
            PropType::String,
            PropType::Multiline,
            PropType::Int,
            PropType::Float,
            PropType::Bool,
            PropType::Enum,
            PropType::Ref,
            PropType::Array,
            PropType::Point,
            PropType::Color,
            PropType::Dialogue,
        ]
    }
}

/// Generic property value (JSON-like but typed)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(untagged)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Convert from serde_json::Value
    pub fn from_json(json: serde_json::Value) -> Self {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from_json).collect())
            }
            serde_json::Value::Object(obj) => Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, Value::from_json(v)))
                    .collect(),
            ),
        }
    }

    /// Convert to serde_json::Value
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::json!(*i),
            Value::Float(f) => serde_json::json!(*f),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
            Value::Object(obj) => serde_json::Value::Object(
                obj.iter().map(|(k, v)| (k.clone(), v.to_json())).collect(),
            ),
        }
    }
}
