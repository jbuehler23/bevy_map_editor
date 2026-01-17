//! Property inspector panel

use bevy_egui::egui;
use bevy_map_animation::SpriteData;
use uuid::Uuid;

use crate::project::Project;
use crate::EditorState;

/// What is currently selected in the editor
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Selection {
    #[default]
    None,
    Level(Uuid),
    Layer(Uuid, usize), // level_id, layer_index
    Entity(Uuid, Uuid), // level_id, entity_id
    Tileset(Uuid),
    DataType(String), // type_name
    DataInstance(Uuid),
    SpriteSheet(Uuid), // sprite sheet asset id
    Dialogue(String),  // dialogue asset id
    // Multi-select variants
    MultipleDataInstances(Vec<Uuid>),
    MultipleEntities(Vec<(Uuid, Uuid)>), // Vec of (level_id, entity_id)
}

/// Result from rendering the inspector
#[derive(Default)]
pub struct InspectorResult {
    pub delete_data_instance: Option<Uuid>,
    pub delete_entity: Option<(Uuid, Uuid)>,
    pub open_sprite_editor: Option<(String, Uuid)>,
    pub open_dialogue_editor: Option<(String, Uuid)>,
    /// Edit sprite sheet animations (opens Animation Editor)
    pub edit_sprite_sheet: Option<Uuid>,
    /// Edit sprite sheet grid settings (opens SpriteSheet Editor)
    pub edit_sprite_sheet_settings: Option<Uuid>,
    pub edit_dialogue: Option<String>,
    /// Create a new data instance and add its ID to an array property
    /// (type_name, target_instance_id, property_name)
    pub create_instance_for_array: Option<(String, Uuid, String)>,
}

/// Render the property inspector
pub fn render_inspector(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
) -> InspectorResult {
    let mut result = InspectorResult::default();

    ui.heading("Inspector");
    ui.separator();

    match &editor_state.selection {
        Selection::None => {
            ui.label("Nothing selected");
        }
        Selection::Level(level_id) => {
            render_level_inspector(ui, *level_id, project);
        }
        Selection::Layer(level_id, layer_idx) => {
            render_layer_inspector(ui, *level_id, *layer_idx, project);
        }
        Selection::Entity(level_id, entity_id) => {
            if render_entity_inspector(ui, *level_id, *entity_id, project) {
                result.delete_entity = Some((*level_id, *entity_id));
            }
        }
        Selection::Tileset(tileset_id) => {
            render_tileset_inspector(ui, *tileset_id, project);
        }
        Selection::DataType(type_name) => {
            render_data_type_inspector(ui, type_name, project);
        }
        Selection::DataInstance(instance_id) => {
            if render_data_instance_inspector(ui, *instance_id, project, &mut result) {
                result.delete_data_instance = Some(*instance_id);
            }
        }
        Selection::SpriteSheet(sprite_sheet_id) => {
            render_sprite_sheet_inspector(ui, *sprite_sheet_id, project, &mut result);
        }
        Selection::Dialogue(ref dialogue_id) => {
            render_dialogue_inspector(ui, dialogue_id, project, &mut result);
        }
        Selection::MultipleDataInstances(ids) => {
            ui.label(format!("{} data instances selected", ids.len()));
            ui.label("Use context menu for bulk operations");
        }
        Selection::MultipleEntities(items) => {
            ui.label(format!("{} entities selected", items.len()));
            ui.label("Use context menu for bulk operations");
        }
    }

    result
}

fn render_level_inspector(ui: &mut egui::Ui, level_id: Uuid, project: &mut Project) {
    let Some(level) = project.get_level_mut(level_id) else {
        ui.label("Level not found");
        return;
    };

    ui.label(format!("Level: {}", level.name));
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut level.name);
    });

    ui.horizontal(|ui| {
        ui.label("Size:");
        ui.label(format!("{}x{}", level.width, level.height));
    });

    ui.label(format!("Layers: {}", level.layers.len()));
    ui.label(format!("Entities: {}", level.entities.len()));
}

fn render_layer_inspector(
    ui: &mut egui::Ui,
    level_id: Uuid,
    layer_idx: usize,
    project: &mut Project,
) {
    let Some(level) = project.get_level_mut(level_id) else {
        ui.label("Level not found");
        return;
    };

    let Some(layer) = level.layers.get_mut(layer_idx) else {
        ui.label("Layer not found");
        return;
    };

    ui.label(format!("Layer: {}", layer.name));
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut layer.name);
    });

    ui.horizontal(|ui| {
        ui.label("Visible:");
        ui.checkbox(&mut layer.visible, "");
    });
}

fn render_entity_inspector(
    ui: &mut egui::Ui,
    level_id: Uuid,
    entity_id: Uuid,
    project: &mut Project,
) -> bool {
    let mut should_delete = false;

    // Phase 1: Extract read-only schema data before mutable borrow
    let (type_name, type_def, enums, sprite_sheets, dialogue_options, ref_options) = {
        let Some(level) = project.get_level(level_id) else {
            ui.label("Level not found");
            return false;
        };
        let Some(entity) = level.get_entity(entity_id) else {
            ui.label("Entity not found");
            return false;
        };

        let type_name = entity.type_name.clone();
        let type_def = project.schema.get_type(&type_name).cloned();
        let enums = project.schema.enums.clone();

        // Collect sprite sheet data (full SpriteData for embedding)
        let sprite_sheets: Vec<SpriteData> = project.sprite_sheets.clone();

        // Collect dialogue options: (id, name)
        let dialogue_options: Vec<(String, String)> = project
            .dialogues
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();

        // Collect ref options per type: HashMap<type_name, Vec<(id, display_name)>>
        let ref_options: std::collections::HashMap<String, Vec<(String, String)>> = project
            .data
            .instances
            .iter()
            .map(|(type_name, instances)| {
                let opts: Vec<(String, String)> = instances
                    .iter()
                    .map(|inst| {
                        let name = inst
                            .properties
                            .get("name")
                            .and_then(|v| v.as_string())
                            .unwrap_or(&inst.id.to_string())
                            .to_string();
                        (inst.id.to_string(), name)
                    })
                    .collect();
                (type_name.clone(), opts)
            })
            .collect();

        (
            type_name,
            type_def,
            enums,
            sprite_sheets,
            dialogue_options,
            ref_options,
        )
    };

    // Phase 2: Mutable access for editing
    let Some(level) = project.get_level_mut(level_id) else {
        ui.label("Level not found");
        return false;
    };
    let Some(entity) = level.get_entity_mut(entity_id) else {
        ui.label("Entity not found");
        return false;
    };

    // Header
    ui.label(format!("Entity: {}", type_name));
    ui.separator();

    // Position editor
    ui.horizontal(|ui| {
        ui.label("Position:");
        ui.add(
            egui::DragValue::new(&mut entity.position[0])
                .speed(1.0)
                .prefix("X: "),
        );
        ui.add(
            egui::DragValue::new(&mut entity.position[1])
                .speed(1.0)
                .prefix("Y: "),
        );
    });

    // Properties section
    if let Some(type_def) = type_def {
        ui.separator();
        ui.label("Properties");

        for prop_def in &type_def.properties {
            // Check show_if condition
            if !should_show_property(prop_def, &entity.properties) {
                continue;
            }

            // Ensure property exists with default
            if !entity.properties.contains_key(&prop_def.name) {
                entity
                    .properties
                    .insert(prop_def.name.clone(), get_default_value(prop_def));
            }

            let value = entity.properties.get_mut(&prop_def.name).unwrap();
            let id_salt = format!("entity_{}_{}", entity_id, prop_def.name);

            // Label with required indicator
            ui.horizontal(|ui| {
                ui.label(&prop_def.name);
                if prop_def.required {
                    ui.colored_label(egui::Color32::RED, "*");
                }
            });

            // Render editor based on prop_type
            render_property_value_editor(
                ui,
                prop_def,
                value,
                &id_salt,
                &enums,
                &sprite_sheets,
                &dialogue_options,
                &ref_options,
            );
        }
    }

    ui.separator();

    if ui.button("Delete Entity").clicked() {
        should_delete = true;
    }

    should_delete
}

fn render_tileset_inspector(ui: &mut egui::Ui, tileset_id: Uuid, project: &mut Project) {
    let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) else {
        ui.label("Tileset not found");
        return;
    };

    ui.label(format!("Tileset: {}", tileset.name));
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut tileset.name);
    });

    ui.horizontal(|ui| {
        ui.label("Tile Size:");
        ui.add(egui::DragValue::new(&mut tileset.tile_size).range(1..=256));
    });

    ui.label(format!("Images: {}", tileset.images.len()));
    ui.label(format!("Total Tiles: {}", tileset.total_tile_count()));
}

fn render_data_type_inspector(ui: &mut egui::Ui, type_name: &str, project: &mut Project) {
    let Some(type_def) = project.schema.get_type(type_name) else {
        ui.label("Type not found");
        return;
    };

    ui.label(format!("Data Type: {}", type_name));
    ui.separator();

    // Show type info
    ui.horizontal(|ui| {
        ui.label("Color:");
        let hex = type_def.color.trim_start_matches('#');
        if hex.len() >= 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                let color = egui::Color32::from_rgb(r, g, b);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(50.0, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
                ui.label(format!("#{}", hex));
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Placeable:");
        ui.label(if type_def.placeable { "Yes" } else { "No" });
    });

    if let Some(icon) = &type_def.icon {
        ui.horizontal(|ui| {
            ui.label("Icon:");
            ui.label(icon);
        });
    }

    // Show properties
    ui.separator();
    ui.label(format!("Properties ({}):", type_def.properties.len()));

    for prop in &type_def.properties {
        ui.horizontal(|ui| {
            ui.label(&prop.name);
            ui.label(format!("({:?})", prop.prop_type));
            if prop.required {
                ui.label("*required");
            }
        });
    }

    // Show instance count
    ui.separator();
    let instance_count = project
        .data
        .instances
        .get(type_name)
        .map(|v| v.len())
        .unwrap_or(0);
    ui.label(format!("Instances: {}", instance_count));
}

fn render_data_instance_inspector(
    ui: &mut egui::Ui,
    instance_id: Uuid,
    project: &mut Project,
    result: &mut InspectorResult,
) -> bool {
    let mut should_delete = false;

    // Phase 1: Extract read-only schema data before mutable borrow
    let (type_name, type_def, enums, sprite_sheets, dialogue_options, ref_options) = {
        let Some(instance) = project.get_data_instance(instance_id) else {
            ui.label("Instance not found");
            return false;
        };

        let type_name = instance.type_name.clone();
        let type_def = project.schema.get_type(&type_name).cloned();
        let enums = project.schema.enums.clone();

        // Collect sprite sheet data (full SpriteData for embedding)
        let sprite_sheets: Vec<SpriteData> = project.sprite_sheets.clone();

        // Collect dialogue options: (id, name)
        let dialogue_options: Vec<(String, String)> = project
            .dialogues
            .iter()
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect();

        // Collect ref options per type: HashMap<type_name, Vec<(id, display_name)>>
        let ref_options: std::collections::HashMap<String, Vec<(String, String)>> = project
            .data
            .instances
            .iter()
            .map(|(type_name, instances)| {
                let opts: Vec<(String, String)> = instances
                    .iter()
                    .map(|inst| {
                        let name = inst
                            .properties
                            .get("name")
                            .and_then(|v| v.as_string())
                            .unwrap_or(&inst.id.to_string())
                            .to_string();
                        (inst.id.to_string(), name)
                    })
                    .collect();
                (type_name.clone(), opts)
            })
            .collect();

        (
            type_name,
            type_def,
            enums,
            sprite_sheets,
            dialogue_options,
            ref_options,
        )
    };

    // Phase 2: Mutable access for editing
    let Some(instance) = project.get_data_instance_mut(instance_id) else {
        ui.label("Instance not found");
        return false;
    };

    // Header
    ui.label(format!("Data Instance: {}", type_name));
    ui.separator();

    // Properties section - schema-aware editing
    if let Some(type_def) = type_def {
        for prop_def in &type_def.properties {
            // Check show_if condition
            if !should_show_property(prop_def, &instance.properties) {
                continue;
            }

            // Ensure property exists with default
            if !instance.properties.contains_key(&prop_def.name) {
                instance
                    .properties
                    .insert(prop_def.name.clone(), get_default_value(prop_def));
            }

            let value = instance.properties.get_mut(&prop_def.name).unwrap();
            let id_salt = format!("data_instance_{}_{}", instance_id, prop_def.name);

            // Label with required indicator
            ui.horizontal(|ui| {
                ui.label(&prop_def.name);
                if prop_def.required {
                    ui.colored_label(egui::Color32::RED, "*");
                }
            });

            // Render editor based on prop_type using the full property editor
            if let Some(create_type) = render_property_value_editor(
                ui,
                prop_def,
                value,
                &id_salt,
                &enums,
                &sprite_sheets,
                &dialogue_options,
                &ref_options,
            ) {
                // Handle inline instance creation for arrays
                result.create_instance_for_array =
                    Some((create_type, instance_id, prop_def.name.clone()));
            }
        }
    } else {
        // Fallback if no schema - display raw properties read-only
        ui.label("(No schema found for this type)");
        for (key, value) in instance.properties.iter() {
            ui.horizontal(|ui| {
                ui.label(format!("{}:", key));
                ui.label(format!("{:?}", value));
            });
        }
    }

    ui.separator();

    if ui.button("Delete Instance").clicked() {
        should_delete = true;
    }

    should_delete
}

fn render_sprite_sheet_inspector(
    ui: &mut egui::Ui,
    sprite_sheet_id: Uuid,
    project: &mut Project,
    result: &mut InspectorResult,
) {
    // First get mutable access to edit the name
    if let Some(sprite_sheet) = project.get_sprite_sheet_mut(sprite_sheet_id) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut sprite_sheet.name);
        });
    }

    // Re-get immutable reference for display
    let Some(sprite_sheet) = project.get_sprite_sheet(sprite_sheet_id) else {
        ui.label("Sprite Sheet not found");
        return;
    };

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Sheet:");
        ui.label(if sprite_sheet.sheet_path.is_empty() {
            "(not set)"
        } else {
            &sprite_sheet.sheet_path
        });
    });

    ui.horizontal(|ui| {
        ui.label("Frame size:");
        ui.label(format!(
            "{}x{}",
            sprite_sheet.frame_width, sprite_sheet.frame_height
        ));
    });

    ui.horizontal(|ui| {
        ui.label("Grid:");
        ui.label(format!(
            "{} columns x {} rows",
            sprite_sheet.columns, sprite_sheet.rows
        ));
    });

    ui.horizontal(|ui| {
        ui.label("Total frames:");
        ui.label(format!("{}", sprite_sheet.total_frames()));
    });

    ui.horizontal(|ui| {
        ui.label("Animations:");
        ui.label(format!("{}", sprite_sheet.animations.len()));
    });

    ui.separator();

    if ui.button("Edit Animations").clicked() {
        result.edit_sprite_sheet = Some(sprite_sheet_id);
    }

    if ui.button("Edit Sheet").clicked() {
        result.edit_sprite_sheet_settings = Some(sprite_sheet_id);
    }
}

fn render_dialogue_inspector(
    ui: &mut egui::Ui,
    dialogue_id: &str,
    project: &mut Project,
    result: &mut InspectorResult,
) {
    // First get mutable access to edit the name
    if let Some(dialogue) = project.get_dialogue_mut(dialogue_id) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut dialogue.name);
        });
    }

    // Re-get immutable reference for display
    let Some(dialogue) = project.get_dialogue(dialogue_id) else {
        ui.label("Dialogue not found");
        return;
    };

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("ID:");
        ui.label(&dialogue.id);
    });

    ui.horizontal(|ui| {
        ui.label("Nodes:");
        ui.label(format!("{}", dialogue.nodes.len()));
    });

    if !dialogue.start_node.is_empty() {
        ui.horizontal(|ui| {
            ui.label("Start node:");
            ui.label(&dialogue.start_node);
        });
    }

    ui.separator();

    if ui.button("Open Editor").clicked() {
        result.edit_dialogue = Some(dialogue_id.to_string());
    }
}

// ============================================================================
// Property Editor Helpers
// ============================================================================

/// Get a default value for a property based on its definition
#[allow(deprecated)] // PropType::Sprite is deprecated but we still handle it for backwards compat
pub fn get_default_value(prop_def: &bevy_map_schema::PropertyDef) -> bevy_map_core::Value {
    use bevy_map_core::Value;
    use bevy_map_schema::PropType;

    if let Some(default) = &prop_def.default {
        return Value::from_json(default.clone());
    }

    match prop_def.prop_type {
        PropType::String | PropType::Multiline => Value::String(String::new()),
        PropType::Int => Value::Int(0),
        PropType::Float => Value::Float(0.0),
        PropType::Bool => Value::Bool(false),
        PropType::Enum => Value::String(String::new()),
        PropType::Ref => Value::Null,
        PropType::Array => Value::Array(Vec::new()),
        PropType::Point => Value::Object(
            [
                ("x".to_string(), Value::Float(0.0)),
                ("y".to_string(), Value::Float(0.0)),
            ]
            .into_iter()
            .collect(),
        ),
        PropType::Color => Value::String("#808080".to_string()),
        PropType::Sprite => Value::Null,
        PropType::Dialogue => Value::Null,
        PropType::Embedded => Value::Null,
    }
}

/// Check if a property should be shown based on its show_if condition
fn should_show_property(
    prop_def: &bevy_map_schema::PropertyDef,
    properties: &std::collections::HashMap<String, bevy_map_core::Value>,
) -> bool {
    let Some(show_if) = &prop_def.show_if else {
        return true;
    };

    if let Some((prop_name, expected)) = show_if.split_once('=') {
        if let Some(actual) = properties.get(prop_name.trim()) {
            let actual_str = match actual {
                bevy_map_core::Value::String(s) => s.clone(),
                bevy_map_core::Value::Bool(b) => b.to_string(),
                bevy_map_core::Value::Int(i) => i.to_string(),
                _ => return false,
            };
            return actual_str == expected.trim();
        }
        return false;
    }
    true
}

/// Parse a hex color string to RGB floats
fn parse_hex_color_to_rgb(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        }
    }
    [0.5, 0.5, 0.5]
}

/// Render a property value editor based on its type
/// Returns Some(type_name) if user clicks "Create New" for an Array property
#[allow(clippy::too_many_arguments)]
#[allow(deprecated)] // PropType::Sprite is deprecated but we still handle it for backwards compat
fn render_property_value_editor(
    ui: &mut egui::Ui,
    prop_def: &bevy_map_schema::PropertyDef,
    value: &mut bevy_map_core::Value,
    id_salt: &str,
    enums: &std::collections::HashMap<String, Vec<String>>,
    sprite_sheets: &[SpriteData],
    dialogue_options: &[(String, String)],
    ref_options: &std::collections::HashMap<String, Vec<(String, String)>>,
) -> Option<String> {
    use bevy_map_core::Value;
    use bevy_map_schema::PropType;

    match prop_def.prop_type {
        PropType::String => {
            let mut s = value.as_string().unwrap_or(&String::new()).to_string();
            if ui.text_edit_singleline(&mut s).changed() {
                *value = Value::String(s);
            }
        }

        PropType::Multiline => {
            let mut s = value.as_string().unwrap_or(&String::new()).to_string();
            if ui.text_edit_multiline(&mut s).changed() {
                *value = Value::String(s);
            }
        }

        PropType::Int => {
            let mut i = value.as_int().unwrap_or(0);
            let mut drag = egui::DragValue::new(&mut i);
            let min_val = prop_def.min.map(|m| m as i64).unwrap_or(i64::MIN);
            let max_val = prop_def.max.map(|m| m as i64).unwrap_or(i64::MAX);
            drag = drag.range(min_val..=max_val);
            if ui.add(drag).changed() {
                *value = Value::Int(i);
            }
        }

        PropType::Float => {
            let mut f = value.as_float().unwrap_or(0.0);
            let mut drag = egui::DragValue::new(&mut f).speed(0.1);
            let min_val = prop_def.min.unwrap_or(f64::MIN);
            let max_val = prop_def.max.unwrap_or(f64::MAX);
            drag = drag.range(min_val..=max_val);
            if ui.add(drag).changed() {
                *value = Value::Float(f);
            }
        }

        PropType::Bool => {
            let mut b = value.as_bool().unwrap_or(false);
            if ui.checkbox(&mut b, "").changed() {
                *value = Value::Bool(b);
            }
        }

        PropType::Enum => {
            if let Some(enum_type) = &prop_def.enum_type {
                if let Some(enum_values) = enums.get(enum_type) {
                    let current = value.as_string().unwrap_or(&String::new()).to_string();
                    egui::ComboBox::from_id_salt(id_salt)
                        .selected_text(if current.is_empty() {
                            "(None)"
                        } else {
                            &current
                        })
                        .show_ui(ui, |ui| {
                            for enum_val in enum_values {
                                if ui
                                    .selectable_label(current == *enum_val, enum_val)
                                    .clicked()
                                {
                                    *value = Value::String(enum_val.clone());
                                }
                            }
                        });
                }
            }
        }

        PropType::Ref => {
            if let Some(ref_type) = &prop_def.ref_type {
                let current_id = value.as_string().unwrap_or(&String::new()).to_string();
                let instances = ref_options.get(ref_type);
                let current_name = instances
                    .and_then(|opts| opts.iter().find(|(id, _)| *id == current_id))
                    .map(|(_, name)| name.as_str())
                    .unwrap_or("(None)");

                egui::ComboBox::from_id_salt(id_salt)
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(current_id.is_empty(), "(None)")
                            .clicked()
                        {
                            *value = Value::Null;
                        }
                        if let Some(opts) = instances {
                            for (id, name) in opts {
                                if ui.selectable_label(*id == current_id, name).clicked() {
                                    *value = Value::String(id.clone());
                                }
                            }
                        }
                    });
            }
        }

        PropType::Point => {
            let (mut x, mut y) = match value {
                Value::Object(obj) => (
                    obj.get("x").and_then(|v| v.as_float()).unwrap_or(0.0),
                    obj.get("y").and_then(|v| v.as_float()).unwrap_or(0.0),
                ),
                _ => (0.0, 0.0),
            };

            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui
                    .add(egui::DragValue::new(&mut x).speed(1.0).prefix("X: "))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut y).speed(1.0).prefix("Y: "))
                    .changed();
            });

            if changed {
                *value = Value::Object(
                    [
                        ("x".to_string(), Value::Float(x)),
                        ("y".to_string(), Value::Float(y)),
                    ]
                    .into_iter()
                    .collect(),
                );
            }
        }

        PropType::Color => {
            let current = value
                .as_string()
                .unwrap_or(&"#808080".to_string())
                .to_string();
            let mut rgb = parse_hex_color_to_rgb(&current);

            if ui.color_edit_button_rgb(&mut rgb).changed() {
                *value = Value::String(format!(
                    "#{:02x}{:02x}{:02x}",
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8
                ));
            }
        }

        PropType::Sprite => {
            // Get current sprite ID from embedded SpriteData object only
            let current_id = match value {
                Value::Object(obj) => obj
                    .get("id")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string()),
                _ => None,
            }
            .unwrap_or_default();

            let current_name = sprite_sheets
                .iter()
                .find(|s| s.id.to_string() == current_id)
                .map(|s| s.name.as_str())
                .unwrap_or("(None)");

            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(current_name)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current_id.is_empty(), "(None)")
                        .clicked()
                    {
                        *value = Value::Null;
                    }
                    for sprite_data in sprite_sheets {
                        let id_str = sprite_data.id.to_string();
                        if ui
                            .selectable_label(id_str == current_id, &sprite_data.name)
                            .clicked()
                        {
                            // Embed full SpriteData as Value::Object
                            if let Ok(json) = serde_json::to_value(sprite_data) {
                                *value = Value::from_json(json);
                            }
                        }
                    }
                });
        }

        PropType::Dialogue => {
            let current_id = value.as_string().unwrap_or(&String::new()).to_string();
            let current_name = dialogue_options
                .iter()
                .find(|(id, _)| *id == current_id)
                .map(|(_, name)| name.as_str())
                .unwrap_or("(None)");

            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(current_name)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current_id.is_empty(), "(None)")
                        .clicked()
                    {
                        *value = Value::Null;
                    }
                    for (id, name) in dialogue_options {
                        if ui.selectable_label(*id == current_id, name).clicked() {
                            *value = Value::String(id.clone());
                        }
                    }
                });
        }

        PropType::Array => {
            return render_array_editor(ui, prop_def, value, id_salt, ref_options);
        }

        PropType::Embedded => {
            ui.label("(embedded type)");
        }
    }

    None
}

/// Render an array editor with add/remove support
/// Returns Some(type_name) if user clicks "Create New" for a custom type
fn render_array_editor(
    ui: &mut egui::Ui,
    prop_def: &bevy_map_schema::PropertyDef,
    value: &mut bevy_map_core::Value,
    id_salt: &str,
    ref_options: &std::collections::HashMap<String, Vec<(String, String)>>,
) -> Option<String> {
    use bevy_map_core::Value;

    // Ensure value is an array
    if !matches!(value, Value::Array(_)) {
        *value = Value::Array(Vec::new());
    }

    let item_type = prop_def.item_type.as_deref().unwrap_or("String");
    let is_custom_type = ref_options.contains_key(item_type);

    // Get instances for custom type
    let instances = if is_custom_type {
        ref_options.get(item_type)
    } else {
        None
    };

    let Value::Array(items) = value else {
        return None;
    };

    let item_count = items.len();
    let mut create_new_type: Option<String> = None;

    // Show item type in header for clarity
    let header_text = if let Some(ref item_type_name) = prop_def.item_type {
        format!(
            "{}: Array<{}> ({} items)",
            prop_def.name, item_type_name, item_count
        )
    } else {
        format!("{} ({} items)", prop_def.name, item_count)
    };

    egui::CollapsingHeader::new(header_text)
        .id_salt(id_salt)
        .default_open(item_count < 5)
        .show(ui, |ui| {
            let mut to_remove = None;

            for (idx, item) in items.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("[{}]", idx));

                    // Render based on item_type
                    if is_custom_type {
                        // Custom data type - show dropdown of instances
                        let current_id = item.as_string().unwrap_or(&String::new()).to_string();
                        let current_name = instances
                            .and_then(|opts| opts.iter().find(|(id, _)| *id == current_id))
                            .map(|(_, name)| name.as_str())
                            .unwrap_or("(None)");

                        egui::ComboBox::from_id_salt(format!("{}_{}", id_salt, idx))
                            .selected_text(current_name)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(current_id.is_empty(), "(None)")
                                    .clicked()
                                {
                                    *item = Value::Null;
                                }
                                if let Some(opts) = instances {
                                    for (id, name) in opts {
                                        if ui.selectable_label(*id == current_id, name).clicked() {
                                            *item = Value::String(id.clone());
                                        }
                                    }
                                }
                            });
                    } else {
                        // Built-in types
                        match item_type {
                            "String" => {
                                let mut s = item.as_string().unwrap_or(&String::new()).to_string();
                                if ui.text_edit_singleline(&mut s).changed() {
                                    *item = Value::String(s);
                                }
                            }
                            "Int" => {
                                let mut i = item.as_int().unwrap_or(0);
                                if ui.add(egui::DragValue::new(&mut i)).changed() {
                                    *item = Value::Int(i);
                                }
                            }
                            "Float" => {
                                let mut f = item.as_float().unwrap_or(0.0);
                                if ui.add(egui::DragValue::new(&mut f).speed(0.1)).changed() {
                                    *item = Value::Float(f);
                                }
                            }
                            "Bool" => {
                                let mut b = item.as_bool().unwrap_or(false);
                                if ui.checkbox(&mut b, "").changed() {
                                    *item = Value::Bool(b);
                                }
                            }
                            _ => {
                                // Fallback for unknown types - treat as string
                                let mut s = item.as_string().unwrap_or(&String::new()).to_string();
                                if ui.text_edit_singleline(&mut s).changed() {
                                    *item = Value::String(s);
                                }
                            }
                        }
                    }

                    // Remove button
                    if ui.small_button("X").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }

            // Handle removal
            if let Some(idx) = to_remove {
                items.remove(idx);
            }

            // Add buttons - different for custom types vs primitive types
            if is_custom_type {
                ui.horizontal(|ui| {
                    // Add existing instance
                    if ui.button("+ Add Existing").clicked() {
                        items.push(Value::Null);
                    }
                    // Create new instance button
                    if ui.button(format!("+ Create New {}", item_type)).clicked() {
                        create_new_type = Some(item_type.to_string());
                    }
                });
            } else {
                // Primitive type - just add button
                if ui.button("+ Add").clicked() {
                    let new_item = match item_type {
                        "String" => Value::String(String::new()),
                        "Int" => Value::Int(0),
                        "Float" => Value::Float(0.0),
                        "Bool" => Value::Bool(false),
                        _ => Value::String(String::new()),
                    };
                    items.push(new_item);
                }
            }
        });

    create_new_type
}
