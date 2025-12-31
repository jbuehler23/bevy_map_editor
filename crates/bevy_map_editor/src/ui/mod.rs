//! Editor UI components using bevy_egui
//!
//! This module provides all the UI panels, dialogs, and widgets for the editor.

mod animation_editor;
mod asset_browser;
mod dialogs;
mod dialogue_editor;
mod entity_palette;
mod inspector;
mod menu_bar;
mod new_project_dialog;
mod schema_editor;
mod settings_dialog;
mod spritesheet_editor;
mod terrain;
mod terrain_palette;
mod theme;
mod tileset;
mod tileset_editor;
mod toolbar;
mod tree_view;
mod world_view;

pub use animation_editor::{render_animation_editor, AnimationEditorResult, AnimationEditorState};
pub use asset_browser::{render_asset_browser, AssetBrowserResult, AssetBrowserState};
pub use dialogs::*;
pub use dialogue_editor::{render_dialogue_editor, DialogueEditorResult, DialogueEditorState};
pub use entity_palette::{render_entity_palette, EntityPaintState};
pub use inspector::{get_default_value, render_inspector, InspectorResult, Selection};
pub use menu_bar::*;
pub use schema_editor::{render_schema_editor, SchemaEditorState};
pub use spritesheet_editor::{
    render_spritesheet_editor, SpriteSheetEditorResult, SpriteSheetEditorState,
};
pub use terrain_palette::{render_terrain_palette, TerrainPaintState};
pub use theme::EditorTheme;
pub use tileset::{
    find_base_tile_for_position, render_tileset_palette, render_tileset_palette_with_cache,
};
pub use tileset_editor::{render_tileset_editor, TilesetEditorState};
pub use toolbar::{render_toolbar, EditorTool, ToolMode};
pub use tree_view::{render_tree_view, TreeViewResult};
pub use world_view::{render_new_level_dialog, render_world_view, NewLevelParams, WorldViewResult};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass, EguiTextureHandle};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::commands::{CommandHistory, TileClipboard};
use crate::project::{DataInstance, Project};
use crate::render::RenderState;
use crate::EditorState;

/// Resource to track spritesheet texture loading
#[derive(Resource, Default)]
pub struct SpritesheetTextureCache {
    /// Loaded textures: path -> (handle, texture_id, width, height)
    pub loaded: HashMap<String, (Handle<Image>, egui::TextureId, f32, f32)>,
    /// Pending texture loads: path -> handle
    pub pending: HashMap<String, Handle<Image>>,
}

/// State of an image load operation
#[derive(Debug, Clone, PartialEq)]
pub enum ImageLoadState {
    /// Not yet started loading
    Pending,
    /// Currently loading
    Loading,
    /// Successfully loaded
    Loaded,
    /// Failed to load
    Failed(String),
}

/// Resource to track tileset texture loading
#[derive(Resource, Default)]
pub struct TilesetTextureCache {
    /// Loaded tileset image textures: image_id -> (handle, texture_id, width, height)
    pub loaded: HashMap<Uuid, (Handle<Image>, egui::TextureId, f32, f32)>,
    /// Pending tileset image loads: image_id -> (path, handle)
    pub pending: HashMap<Uuid, (PathBuf, Handle<Image>)>,
    /// Mapping from tileset_id to its first image's id
    pub tileset_primary_image: HashMap<Uuid, Uuid>,
    /// Load state for each image (for UI feedback)
    pub load_states: HashMap<Uuid, ImageLoadState>,
}

impl TilesetTextureCache {
    /// Get the load state for an image
    pub fn get_load_state(&self, image_id: &Uuid) -> ImageLoadState {
        if self.loaded.contains_key(image_id) {
            ImageLoadState::Loaded
        } else if let Some(state) = self.load_states.get(image_id) {
            state.clone()
        } else if self.pending.contains_key(image_id) {
            ImageLoadState::Loading
        } else {
            ImageLoadState::Pending
        }
    }
}

/// Main UI plugin
pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<SpritesheetTextureCache>()
            .init_resource::<TilesetTextureCache>()
            .add_systems(
                Update,
                (
                    load_tileset_textures,
                    load_spritesheet_textures,
                    process_edit_actions,
                ),
            )
            .add_systems(EguiPrimaryContextPass, render_ui);
    }
}

/// UI state for panel visibility and sizes
#[derive(Resource)]
pub struct UiState {
    pub show_tree_view: bool,
    pub show_inspector: bool,
    pub show_asset_browser: bool,
    pub tree_view_width: f32,
    pub inspector_width: f32,
    pub asset_browser_height: f32,
    pub asset_browser_state: AssetBrowserState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_tree_view: true,
            show_inspector: true,
            show_asset_browser: false,
            tree_view_width: 200.0,
            inspector_width: 250.0,
            asset_browser_height: 200.0,
            asset_browser_state: AssetBrowserState::default(),
        }
    }
}

/// System to load tileset textures and register them with egui
fn load_tileset_textures(
    mut project: ResMut<Project>,
    mut cache: ResMut<TilesetTextureCache>,
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
) {
    use bevy::asset::LoadState;

    // Early exit: if no pending loads and all images loaded/failed, skip entirely
    if cache.pending.is_empty() {
        // Check if we have any unprocessed images
        let has_unprocessed = project.tilesets.iter().any(|tileset| {
            tileset.images.iter().any(|img| {
                !cache.loaded.contains_key(&img.id)
                    && !matches!(
                        cache.load_states.get(&img.id),
                        Some(ImageLoadState::Failed(_))
                    )
            })
        });
        if !has_unprocessed {
            return; // All textures loaded or failed, nothing to do
        }
    }

    // Migrate legacy tilesets to multi-image format (only if needed)
    // Check if any tileset needs migration first
    let needs_migration = project
        .tilesets
        .iter()
        .any(|t| t.images.is_empty() && t.path.as_ref().map_or(false, |p| !p.is_empty()));
    if needs_migration {
        for tileset in project.tilesets.iter_mut() {
            tileset.migrate_to_multi_image();
        }
    }

    // Process images directly without collecting into Vec
    // First gather what we need to process (without cloning paths yet)
    let mut images_to_process: Vec<(uuid::Uuid, usize, uuid::Uuid, u32)> = Vec::new();
    for tileset in project.tilesets.iter() {
        let tileset_id = tileset.id;
        let tile_size = tileset.tile_size;
        for (img_idx, image) in tileset.images.iter().enumerate() {
            let img_id = image.id;
            if !cache.loaded.contains_key(&img_id)
                && !matches!(
                    cache.load_states.get(&img_id),
                    Some(ImageLoadState::Failed(_))
                )
            {
                images_to_process.push((tileset_id, img_idx, img_id, tile_size));
            }
        }
    }

    for (tileset_id, img_idx, image_id, tile_size) in images_to_process {
        // Get the image path only when needed
        let image_path = project
            .tilesets
            .iter()
            .find(|t| t.id == tileset_id)
            .and_then(|t| t.images.get(img_idx))
            .map(|img| img.path.clone());

        let Some(image_path) = image_path else {
            continue;
        };
        // Check if load is pending
        if let Some((path, handle)) = cache.pending.get(&image_id).cloned() {
            // Check load state using AssetServer
            match asset_server.load_state(&handle) {
                LoadState::Loaded => {
                    // Check if the image asset is available
                    if let Some(image) = images.get(&handle) {
                        let width = image.width() as f32;
                        let height = image.height() as f32;

                        // Register with egui
                        let texture_id =
                            contexts.add_image(EguiTextureHandle::Strong(handle.clone()));

                        // Cache the result
                        cache
                            .loaded
                            .insert(image_id, (handle.clone(), texture_id, width, height));
                        cache.pending.remove(&image_id);
                        cache.load_states.insert(image_id, ImageLoadState::Loaded);

                        // Track primary image for tileset (first image)
                        if img_idx == 0 {
                            cache.tileset_primary_image.insert(tileset_id, image_id);
                        }

                        // Update image dimensions based on actual size
                        if let Some(tileset) =
                            project.tilesets.iter_mut().find(|t| t.id == tileset_id)
                        {
                            if let Some(tileset_image) =
                                tileset.images.iter_mut().find(|i| i.id == image_id)
                            {
                                tileset_image.columns = (width as u32) / tile_size.max(1);
                                tileset_image.rows = (height as u32) / tile_size.max(1);
                            }
                            // Also update legacy columns/rows if this is first image
                            if img_idx == 0 {
                                tileset.columns = (width as u32) / tile_size.max(1);
                                tileset.rows = (height as u32) / tile_size.max(1);
                            }
                        }
                    }
                }
                LoadState::Failed(_) => {
                    // Mark as failed
                    let error_msg = format!("Failed to load: {}", path.display());
                    cache
                        .load_states
                        .insert(image_id, ImageLoadState::Failed(error_msg));
                    cache.pending.remove(&image_id);

                    // Track primary image even on failure (for UI display)
                    if img_idx == 0 {
                        cache.tileset_primary_image.insert(tileset_id, image_id);
                    }
                }
                LoadState::Loading => {
                    // Still loading
                    cache.load_states.insert(image_id, ImageLoadState::Loading);
                }
                LoadState::NotLoaded => {
                    // Not started yet, will be started
                }
            }
            continue;
        }

        // Start loading the image (convert path for Bevy's AssetServer)
        let asset_path = crate::to_asset_path(&image_path);
        let handle: Handle<Image> = asset_server.load(&asset_path);
        cache
            .pending
            .insert(image_id, (PathBuf::from(&image_path), handle));
        cache.load_states.insert(image_id, ImageLoadState::Loading);

        // Track primary image for tileset
        if img_idx == 0 {
            cache.tileset_primary_image.insert(tileset_id, image_id);
        }
    }
}

/// System to load spritesheet textures for both SpriteSheet Editor and Animation Editor
fn load_spritesheet_textures(
    mut editor_state: ResMut<EditorState>,
    mut cache: ResMut<SpritesheetTextureCache>,
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
) {
    // Collect paths that need loading from both editors
    let mut paths_to_check: Vec<(String, bool, bool)> = Vec::new();

    // SpriteSheet Editor needs texture?
    if editor_state.show_spritesheet_editor
        && editor_state.spritesheet_editor_state.needs_texture_load()
    {
        let path = editor_state
            .spritesheet_editor_state
            .sprite_data
            .sheet_path
            .clone();
        if !path.is_empty() {
            paths_to_check.push((path, true, false));
        }
    }

    // Animation Editor needs texture?
    if editor_state.show_animation_editor
        && editor_state.animation_editor_state.needs_texture_load()
    {
        let path = editor_state
            .animation_editor_state
            .sprite_data
            .sheet_path
            .clone();
        if !path.is_empty() {
            // Check if we already have this path from spritesheet editor
            if !paths_to_check.iter().any(|(p, _, _)| p == &path) {
                paths_to_check.push((path, false, true));
            } else {
                // Mark that animation editor also needs this path
                if let Some((_, _, anim)) = paths_to_check.iter_mut().find(|(p, _, _)| p == &path) {
                    *anim = true;
                }
            }
        }
    }

    for (sheet_path, for_spritesheet_editor, for_animation_editor) in paths_to_check {
        // Check if already loaded
        if let Some((_, texture_id, width, height)) = cache.loaded.get(&sheet_path) {
            // Already loaded, update the appropriate editor state(s)
            if for_spritesheet_editor {
                editor_state
                    .spritesheet_editor_state
                    .set_texture(*texture_id, *width, *height);
            }
            if for_animation_editor {
                editor_state
                    .animation_editor_state
                    .set_texture(*texture_id, *width, *height);
            }
            continue;
        }

        // Check if load is pending
        if let Some(handle) = cache.pending.get(&sheet_path).cloned() {
            // Check if the image has finished loading
            if let Some(image) = images.get(&handle) {
                let width = image.width() as f32;
                let height = image.height() as f32;

                debug!(
                    "Spritesheet loaded: {} -> {}x{} px",
                    sheet_path, width as u32, height as u32
                );

                // Register with egui
                let texture_id = contexts.add_image(EguiTextureHandle::Strong(handle.clone()));

                // Cache the result
                cache.loaded.insert(
                    sheet_path.clone(),
                    (handle.clone(), texture_id, width, height),
                );
                cache.pending.remove(&sheet_path);

                // Update editor state(s)
                if for_spritesheet_editor {
                    editor_state
                        .spritesheet_editor_state
                        .set_texture(texture_id, width, height);
                }
                if for_animation_editor {
                    editor_state
                        .animation_editor_state
                        .set_texture(texture_id, width, height);
                }
            }
            continue;
        }

        // Start loading the image (convert path for Bevy's AssetServer)
        debug!("Loading spritesheet: {}", sheet_path);
        let asset_path = crate::to_asset_path(&sheet_path);
        let handle: Handle<Image> = asset_server.load(&asset_path);
        cache.pending.insert(sheet_path, handle);
    }
}

/// Main UI rendering system
fn render_ui(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut editor_state: ResMut<EditorState>,
    mut project: ResMut<Project>,
    mut preferences: ResMut<crate::preferences::EditorPreferences>,
    tileset_cache: Res<TilesetTextureCache>,
    assets_base_path: Res<crate::AssetsBasePath>,
    history: Res<CommandHistory>,
    clipboard: Res<TileClipboard>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // Apply editor theme
    EditorTheme::apply(ctx);

    // Menu bar
    render_menu_bar(
        ctx,
        &mut ui_state,
        &mut editor_state,
        &mut project,
        Some(&history),
        Some(&clipboard),
        &preferences,
    );

    // New Project dialog
    new_project_dialog::render_new_project_dialog(ctx, &mut editor_state, &mut project);

    // Settings dialog
    settings_dialog::render_settings_dialog(
        ctx,
        &mut editor_state.show_settings_dialog,
        &mut preferences,
    );

    // Toolbar
    render_toolbar(ctx, &mut editor_state);

    // Left panel - Tree View
    let mut tree_view_result = TreeViewResult::default();
    if ui_state.show_tree_view {
        egui::SidePanel::left("tree_view")
            .resizable(true)
            .default_width(ui_state.tree_view_width)
            .show(ctx, |ui| {
                ui_state.tree_view_width = ui.available_width();
                tree_view_result = render_tree_view(ui, &mut editor_state, &mut project);
            });
    }

    // Right panel - Inspector + Terrain Palette
    let mut inspector_result = InspectorResult::default();
    if ui_state.show_inspector {
        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(ui_state.inspector_width)
            .show(ctx, |ui| {
                ui_state.inspector_width = ui.available_width();

                // Split the panel: Inspector at top, Terrain Palette at bottom
                let available_height = ui.available_height();
                let inspector_height = available_height * 0.5;

                // Top: Inspector
                egui::TopBottomPanel::top("inspector_top")
                    .resizable(true)
                    .default_height(inspector_height)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("inspector_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                inspector_result =
                                    render_inspector(ui, &mut editor_state, &mut project);
                            });
                    });

                // Bottom: Palette (contextual based on tool/layer)
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    // Determine if we're on an Object layer
                    let is_object_layer = editor_state
                        .selected_level
                        .and_then(|lid| project.levels.iter().find(|l| l.id == lid))
                        .and_then(|level| {
                            editor_state
                                .selected_layer
                                .and_then(|idx| level.layers.get(idx))
                        })
                        .map(|layer| matches!(layer.data, bevy_map_core::LayerData::Objects { .. }))
                        .unwrap_or(false);

                    // Show Entity palette when Entity tool is selected or on Object layer
                    if matches!(editor_state.current_tool, EditorTool::Entity) || is_object_layer {
                        ui.heading("Entity Types");
                        ui.separator();
                        render_entity_palette(ui, &mut editor_state, &project);
                    } else {
                        ui.heading("Terrain & Tiles");
                        ui.separator();
                        render_terrain_palette(
                            ui,
                            &mut editor_state,
                            &project,
                            Some(&tileset_cache),
                        );
                    }
                });
            });
    }

    // Handle inspector actions (deletions)
    if let Some(id) = inspector_result.delete_data_instance {
        project.remove_data_instance(id);
        editor_state.selection = Selection::None;
    }
    if let Some((level_id, entity_id)) = inspector_result.delete_entity {
        if let Some(level) = project.get_level_mut(level_id) {
            level.remove_entity(entity_id);
            // Also remove from any Object layer that references it
            for layer in &mut level.layers {
                if let bevy_map_core::LayerData::Objects { entities } = &mut layer.data {
                    entities.retain(|&id| id != entity_id);
                }
            }
        }
        editor_state.selection = Selection::None;
    }

    // Handle tree view actions
    if let Some(id) = tree_view_result.duplicate_data {
        if let Some(new_id) = project.duplicate_data_instance(id) {
            editor_state.selection = Selection::DataInstance(new_id);
        }
    }
    if let Some(id) = tree_view_result.delete_data {
        project.remove_data_instance(id);
        editor_state.selection = Selection::None;
    }
    if let Some(id) = tree_view_result.duplicate_level {
        if let Some(new_id) = project.duplicate_level(id) {
            editor_state.selection = Selection::Level(new_id);
        }
    }
    if let Some(id) = tree_view_result.delete_level {
        project.remove_level(id);
        editor_state.selection = Selection::None;
    }
    if let Some((level_id, entity_id)) = tree_view_result.delete_entity {
        if let Some(level) = project.get_level_mut(level_id) {
            level.remove_entity(entity_id);
            // Also remove from any Object layer that references it
            for layer in &mut level.layers {
                if let bevy_map_core::LayerData::Objects { entities } = &mut layer.data {
                    entities.retain(|&id| id != entity_id);
                }
            }
        }
        editor_state.selection = Selection::None;
    }

    // Handle data instance selection from tree view
    if let Some(instance_id) = tree_view_result.selected_data_instance {
        editor_state.selection = Selection::DataInstance(instance_id);
    }

    // Handle entity selection from tree view (level entities shown in Data section)
    if let Some((level_id, entity_id)) = tree_view_result.selected_entity {
        editor_state.selection = Selection::Entity(level_id, entity_id);
    }

    // Handle create new data instance from tree view
    if let Some(type_name) = tree_view_result.create_data_instance {
        if let Some(type_def) = project.schema.get_type(&type_name).cloned() {
            let mut instance = DataInstance::new(type_name.clone());
            // Populate with default values from type definition
            for prop_def in &type_def.properties {
                instance.properties.insert(
                    prop_def.name.clone(),
                    inspector::get_default_value(prop_def),
                );
            }
            let id = instance.id;
            project.add_data_instance(instance);
            editor_state.selection = Selection::DataInstance(id);
        }
    }

    // Handle delete data instance from tree view
    if let Some(instance_id) = tree_view_result.delete_data_instance {
        project.remove_data_instance(instance_id);
        if matches!(editor_state.selection, Selection::DataInstance(id) if id == instance_id) {
            editor_state.selection = Selection::None;
        }
    }

    // Handle duplicate data instance
    if let Some(inst_id) = tree_view_result.duplicate_data_instance {
        if let Some(new_id) = project.duplicate_data_instance(inst_id) {
            editor_state.selection = Selection::DataInstance(new_id);
        }
    }

    // Handle duplicate level entity
    if let Some((level_id, entity_id)) = tree_view_result.duplicate_entity {
        if let Some(level) = project.get_level_mut(level_id) {
            if let Some(original) = level.entities.iter().find(|e| e.id == entity_id).cloned() {
                let mut duplicate = original.clone();
                duplicate.id = Uuid::new_v4();
                // Offset position slightly
                duplicate.position[0] += 32.0;
                duplicate.position[1] += 32.0;
                let new_id = duplicate.id;
                level.entities.push(duplicate);
                // Also add to object layer if original was on one
                for layer in &mut level.layers {
                    if let bevy_map_core::LayerData::Objects { entities } = &mut layer.data {
                        if entities.contains(&entity_id) {
                            entities.push(new_id);
                            break;
                        }
                    }
                }
                editor_state.selection = Selection::Entity(level_id, new_id);
            }
        }
    }

    // Handle Ctrl+Click multi-select toggle for data instances
    if let Some(inst_id) = tree_view_result.toggle_data_instance {
        match &editor_state.selection {
            Selection::MultipleDataInstances(ids) => {
                let mut ids = ids.clone();
                if let Some(pos) = ids.iter().position(|&id| id == inst_id) {
                    ids.remove(pos);
                    if ids.len() == 1 {
                        editor_state.selection = Selection::DataInstance(ids[0]);
                    } else if ids.is_empty() {
                        editor_state.selection = Selection::None;
                    } else {
                        editor_state.selection = Selection::MultipleDataInstances(ids);
                    }
                } else {
                    ids.push(inst_id);
                    editor_state.selection = Selection::MultipleDataInstances(ids);
                }
            }
            Selection::DataInstance(existing_id) => {
                if *existing_id == inst_id {
                    // Deselect
                    editor_state.selection = Selection::None;
                } else {
                    // Start multi-select
                    editor_state.selection =
                        Selection::MultipleDataInstances(vec![*existing_id, inst_id]);
                }
            }
            _ => {
                editor_state.selection = Selection::DataInstance(inst_id);
            }
        }
    }

    // Handle Ctrl+Click multi-select toggle for level entities
    if let Some((level_id, entity_id)) = tree_view_result.toggle_entity {
        match &editor_state.selection {
            Selection::MultipleEntities(items) => {
                let mut items = items.clone();
                if let Some(pos) = items
                    .iter()
                    .position(|&(l, e)| l == level_id && e == entity_id)
                {
                    items.remove(pos);
                    if items.len() == 1 {
                        editor_state.selection = Selection::Entity(items[0].0, items[0].1);
                    } else if items.is_empty() {
                        editor_state.selection = Selection::None;
                    } else {
                        editor_state.selection = Selection::MultipleEntities(items);
                    }
                } else {
                    items.push((level_id, entity_id));
                    editor_state.selection = Selection::MultipleEntities(items);
                }
            }
            Selection::Entity(existing_level, existing_entity) => {
                if *existing_level == level_id && *existing_entity == entity_id {
                    // Deselect
                    editor_state.selection = Selection::None;
                } else {
                    // Start multi-select
                    editor_state.selection = Selection::MultipleEntities(vec![
                        (*existing_level, *existing_entity),
                        (level_id, entity_id),
                    ]);
                }
            }
            _ => {
                editor_state.selection = Selection::Entity(level_id, entity_id);
            }
        }
    }

    // Handle bulk delete data instances
    if tree_view_result.delete_selected_data_instances {
        if let Selection::MultipleDataInstances(ids) = &editor_state.selection {
            for id in ids.clone() {
                project.remove_data_instance(id);
            }
            editor_state.selection = Selection::None;
        }
    }

    // Handle bulk delete level entities
    if tree_view_result.delete_selected_entities {
        if let Selection::MultipleEntities(items) = &editor_state.selection {
            for (level_id, entity_id) in items.clone() {
                if let Some(level) = project.get_level_mut(level_id) {
                    level.remove_entity(entity_id);
                    // Also remove from any Object layer that references it
                    for layer in &mut level.layers {
                        if let bevy_map_core::LayerData::Objects { entities } = &mut layer.data {
                            entities.retain(|&id| id != entity_id);
                        }
                    }
                }
            }
            editor_state.selection = Selection::None;
        }
    }

    // Handle rename data instance (initiate rename mode)
    if let Some(inst_id) = tree_view_result.rename_data_instance {
        if let Some(instance) = project.get_data_instance(inst_id) {
            let current_name = instance
                .properties
                .get("name")
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            editor_state.rename_buffer = current_name.to_string();
            editor_state.renaming_item = Some(crate::RenamingItem::DataInstance(inst_id));
        }
    }

    // Handle rename entity (initiate rename mode)
    if let Some((level_id, entity_id)) = tree_view_result.rename_entity {
        if let Some(level) = project.get_level(level_id) {
            if let Some(entity) = level.entities.iter().find(|e| e.id == entity_id) {
                let current_name = entity
                    .properties
                    .get("name")
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                editor_state.rename_buffer = current_name.to_string();
                editor_state.renaming_item = Some(crate::RenamingItem::Entity(level_id, entity_id));
            }
        }
    }

    // Handle rename level (initiate rename mode)
    if let Some(level_id) = tree_view_result.rename_level {
        if let Some(level) = project.get_level(level_id) {
            editor_state.rename_buffer = level.name.clone();
            editor_state.renaming_item = Some(crate::RenamingItem::Level(level_id));
        }
    }

    // Handle rename layer (initiate rename mode)
    if let Some((level_id, layer_idx)) = tree_view_result.rename_layer {
        if let Some(level) = project.get_level(level_id) {
            if let Some(layer) = level.layers.get(layer_idx) {
                editor_state.rename_buffer = layer.name.clone();
                editor_state.renaming_item = Some(crate::RenamingItem::Layer(level_id, layer_idx));
            }
        }
    }

    // Handle rename tileset (initiate rename mode)
    if let Some(tileset_id) = tree_view_result.rename_tileset {
        if let Some(tileset) = project.tilesets.iter().find(|t| t.id == tileset_id) {
            editor_state.rename_buffer = tileset.name.clone();
            editor_state.renaming_item = Some(crate::RenamingItem::Tileset(tileset_id));
        }
    }

    // Handle rename sprite sheet (initiate rename mode)
    if let Some(sprite_sheet_id) = tree_view_result.rename_sprite_sheet {
        if let Some(sprite_sheet) = project
            .sprite_sheets
            .iter()
            .find(|s| s.id == sprite_sheet_id)
        {
            editor_state.rename_buffer = sprite_sheet.name.clone();
            editor_state.renaming_item = Some(crate::RenamingItem::SpriteSheet(sprite_sheet_id));
        }
    }

    // Handle rename dialogue (initiate rename mode)
    if let Some(ref dialogue_id) = tree_view_result.rename_dialogue {
        if let Some(dialogue) = project.dialogues.iter().find(|d| d.id == *dialogue_id) {
            editor_state.rename_buffer = dialogue.name.clone();
            editor_state.renaming_item = Some(crate::RenamingItem::Dialogue(dialogue_id.clone()));
        }
    }

    // Handle commit rename
    if let Some(new_name) = tree_view_result.commit_rename {
        match &editor_state.renaming_item {
            Some(crate::RenamingItem::DataInstance(inst_id)) => {
                if let Some(instance) = project.get_data_instance_mut(*inst_id) {
                    instance
                        .properties
                        .insert("name".to_string(), bevy_map_core::Value::String(new_name));
                }
            }
            Some(crate::RenamingItem::Entity(level_id, entity_id)) => {
                if let Some(level) = project.get_level_mut(*level_id) {
                    if let Some(entity) = level.entities.iter_mut().find(|e| e.id == *entity_id) {
                        entity
                            .properties
                            .insert("name".to_string(), bevy_map_core::Value::String(new_name));
                    }
                }
            }
            Some(crate::RenamingItem::Level(level_id)) => {
                if let Some(level) = project.get_level_mut(*level_id) {
                    level.name = new_name;
                }
            }
            Some(crate::RenamingItem::Layer(level_id, layer_idx)) => {
                if let Some(level) = project.get_level_mut(*level_id) {
                    if let Some(layer) = level.layers.get_mut(*layer_idx) {
                        layer.name = new_name;
                    }
                }
            }
            Some(crate::RenamingItem::Tileset(tileset_id)) => {
                if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == *tileset_id) {
                    tileset.name = new_name;
                }
            }
            Some(crate::RenamingItem::SpriteSheet(sprite_sheet_id)) => {
                if let Some(sprite_sheet) = project
                    .sprite_sheets
                    .iter_mut()
                    .find(|s| s.id == *sprite_sheet_id)
                {
                    sprite_sheet.name = new_name;
                }
            }
            Some(crate::RenamingItem::Dialogue(dialogue_id)) => {
                if let Some(dialogue) = project.dialogues.iter_mut().find(|d| d.id == *dialogue_id)
                {
                    dialogue.name = new_name;
                }
            }
            None => {}
        }
        editor_state.renaming_item = None;
        editor_state.rename_buffer.clear();
    }

    // Handle cancel rename
    if tree_view_result.cancel_rename {
        editor_state.renaming_item = None;
        editor_state.rename_buffer.clear();
    }

    // Handle layer creation
    if let Some(level_id) = tree_view_result.add_tile_layer {
        // Get tileset id first to avoid borrow conflicts
        let tileset_id = project.tilesets.first().map(|t| t.id);
        if let Some(tileset_id) = tileset_id {
            if let Some(level) = project.get_level_mut(level_id) {
                let layer = bevy_map_core::Layer::new_tile_layer(
                    format!("Tile Layer {}", level.layers.len() + 1),
                    tileset_id,
                    level.width,
                    level.height,
                );
                level.layers.push(layer);
                editor_state.selected_layer = Some(level.layers.len() - 1);
            }
        } else {
            editor_state.error_message = Some(
                "Cannot create tile layer: No tilesets available. Create a tileset first."
                    .to_string(),
            );
        }
    }

    if let Some(level_id) = tree_view_result.add_object_layer {
        if let Some(level) = project.get_level_mut(level_id) {
            let layer = bevy_map_core::Layer::new_object_layer(format!(
                "Object Layer {}",
                level.layers.len() + 1
            ));
            level.layers.push(layer);
            editor_state.selected_layer = Some(level.layers.len() - 1);
        }
    }

    // Handle layer deletion
    if let Some((level_id, layer_idx)) = tree_view_result.delete_layer {
        if let Some(level) = project.get_level_mut(level_id) {
            if layer_idx < level.layers.len() {
                level.layers.remove(layer_idx);
                // Adjust selected layer if needed
                if let Some(selected) = editor_state.selected_layer {
                    if selected >= level.layers.len() {
                        editor_state.selected_layer = level.layers.len().checked_sub(1);
                    }
                }
            }
        }
    }

    // Handle layer reordering
    if let Some((level_id, layer_idx)) = tree_view_result.move_layer_up {
        if let Some(level) = project.get_level_mut(level_id) {
            if layer_idx > 0 && layer_idx < level.layers.len() {
                level.layers.swap(layer_idx, layer_idx - 1);
                if editor_state.selected_layer == Some(layer_idx) {
                    editor_state.selected_layer = Some(layer_idx - 1);
                } else if editor_state.selected_layer == Some(layer_idx - 1) {
                    editor_state.selected_layer = Some(layer_idx);
                }
            }
        }
    }

    if let Some((level_id, layer_idx)) = tree_view_result.move_layer_down {
        if let Some(level) = project.get_level_mut(level_id) {
            if layer_idx + 1 < level.layers.len() {
                level.layers.swap(layer_idx, layer_idx + 1);
                if editor_state.selected_layer == Some(layer_idx) {
                    editor_state.selected_layer = Some(layer_idx + 1);
                } else if editor_state.selected_layer == Some(layer_idx + 1) {
                    editor_state.selected_layer = Some(layer_idx);
                }
            }
        }
    }

    // Handle layer visibility toggle
    if let Some((level_id, layer_idx)) = tree_view_result.toggle_layer_visibility {
        if let Some(level) = project.get_level_mut(level_id) {
            if let Some(layer) = level.layers.get_mut(layer_idx) {
                layer.visible = !layer.visible;
            }
        }
    }

    // Handle layer duplication
    if let Some((level_id, layer_idx)) = tree_view_result.duplicate_layer {
        if let Some(level) = project.get_level_mut(level_id) {
            if let Some(layer) = level.layers.get(layer_idx).cloned() {
                let mut duplicate = layer;
                duplicate.name = format!("{} (Copy)", duplicate.name);
                level.layers.insert(layer_idx + 1, duplicate);
                editor_state.selected_layer = Some(layer_idx + 1);
            }
        }
    }

    // Handle entity type selection from tree view
    if let Some(type_name) = tree_view_result.select_entity_type_for_placement {
        editor_state.selected_entity_type = Some(type_name);
        // Don't automatically switch tools - let users manually select Entity tool
    }

    // Handle tileset duplication
    if let Some(tileset_id) = tree_view_result.duplicate_tileset {
        if let Some(original) = project
            .tilesets
            .iter()
            .find(|t| t.id == tileset_id)
            .cloned()
        {
            let mut duplicate = original;
            duplicate.id = Uuid::new_v4();
            duplicate.name = format!("{} (Copy)", duplicate.name);
            let new_id = duplicate.id;
            project.tilesets.push(duplicate);
            editor_state.selection = Selection::Tileset(new_id);
            editor_state.selected_tileset = Some(new_id);
        }
    }

    // Handle tileset deletion
    if let Some(tileset_id) = tree_view_result.delete_tileset {
        project.tilesets.retain(|t| t.id != tileset_id);

        // Cascade delete: remove terrain sets that used this tileset
        let removed_count = project
            .autotile_config
            .terrain_sets
            .iter()
            .filter(|ts| ts.tileset_id == tileset_id)
            .count();
        project
            .autotile_config
            .terrain_sets
            .retain(|ts| ts.tileset_id != tileset_id);
        if removed_count > 0 {
            bevy::log::info!(
                "Cascade deleted {} terrain set(s) associated with deleted tileset",
                removed_count
            );
        }

        if matches!(editor_state.selection, Selection::Tileset(id) if id == tileset_id) {
            editor_state.selection = Selection::None;
        }
        if editor_state.selected_tileset == Some(tileset_id) {
            editor_state.selected_tileset = None;
        }
    }

    // Handle sprite sheet actions from tree view
    if tree_view_result.create_sprite_sheet {
        let mut sprite_sheet = bevy_map_animation::SpriteData::default();
        sprite_sheet.id = Uuid::new_v4();
        sprite_sheet.name = format!("Sprite Sheet {}", project.sprite_sheets.len() + 1);
        let id = sprite_sheet.id;
        project.add_sprite_sheet(sprite_sheet);
        editor_state.selection = Selection::SpriteSheet(id);
    }

    // Handle opening Animation Editor for sprite sheets
    if let Some(id) = tree_view_result
        .edit_sprite_sheet
        .or(inspector_result.edit_sprite_sheet)
    {
        if let Some(sprite_sheet) = project.get_sprite_sheet(id) {
            editor_state.animation_editor_state =
                AnimationEditorState::from_sprite_data(sprite_sheet.clone(), id);
            editor_state.show_animation_editor = true;
        }
    }

    // Handle opening SpriteSheet Editor (for grid setup)
    if let Some(id) = tree_view_result
        .edit_sprite_sheet_settings
        .or(inspector_result.edit_sprite_sheet_settings)
    {
        if let Some(sprite_sheet) = project.get_sprite_sheet(id) {
            editor_state.spritesheet_editor_state =
                SpriteSheetEditorState::from_sprite_data(sprite_sheet.clone(), id);
            editor_state.show_spritesheet_editor = true;
        }
    }

    if let Some(id) = tree_view_result.delete_sprite_sheet {
        project.remove_sprite_sheet(id);
        if matches!(editor_state.selection, Selection::SpriteSheet(sel_id) if sel_id == id) {
            editor_state.selection = Selection::None;
        }
    }

    if let Some(id) = tree_view_result.duplicate_sprite_sheet {
        if let Some(original) = project.get_sprite_sheet(id) {
            let mut duplicate = original.clone();
            duplicate.id = Uuid::new_v4();
            duplicate.name = format!("{} (Copy)", duplicate.name);
            let new_id = duplicate.id;
            project.add_sprite_sheet(duplicate);
            editor_state.selection = Selection::SpriteSheet(new_id);
        }
    }

    // Handle dialogue actions from tree view
    if tree_view_result.create_dialogue {
        let dialogue = bevy_map_dialogue::DialogueTree::new(format!(
            "New Dialogue {}",
            project.dialogues.len() + 1
        ));
        let id = dialogue.id.clone();
        project.add_dialogue(dialogue);
        editor_state.selection = Selection::Dialogue(id);
    }

    if let Some(id) = tree_view_result
        .edit_dialogue
        .or(inspector_result.edit_dialogue)
    {
        if let Some(dialogue) = project.get_dialogue(&id) {
            editor_state.dialogue_editor_state =
                DialogueEditorState::from_dialogue(dialogue.clone());
            editor_state.dialogue_editor_asset_id = Some(id.clone());
            editor_state.show_dialogue_editor = true;
        }
    }

    if let Some(ref id) = tree_view_result.delete_dialogue {
        project.remove_dialogue(id);
        if matches!(&editor_state.selection, Selection::Dialogue(sel_id) if sel_id == id) {
            editor_state.selection = Selection::None;
        }
    }

    if let Some(ref id) = tree_view_result.duplicate_dialogue {
        if let Some(original) = project.get_dialogue(id) {
            let mut duplicate = original.clone();
            duplicate.id = Uuid::new_v4().to_string();
            duplicate.name = format!("{} (Copy)", duplicate.name);
            let new_id = duplicate.id.clone();
            project.add_dialogue(duplicate);
            editor_state.selection = Selection::Dialogue(new_id);
        }
    }

    // Bottom panel - Asset Browser
    if ui_state.show_asset_browser {
        egui::TopBottomPanel::bottom("asset_browser")
            .resizable(true)
            .default_height(ui_state.asset_browser_height)
            .min_height(100.0)
            .show(ctx, |ui| {
                ui_state.asset_browser_height = ui.available_height();
                let _result = render_asset_browser(ui, &mut ui_state.asset_browser_state);
                // TODO: Handle result.file_activated for import actions
            });
    }

    // Central area - world view or level view
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            match editor_state.view_mode {
                crate::EditorViewMode::World => {
                    // Render world view with a dark background
                    let rect = ui.available_rect_before_wrap();
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 35));

                    let world_result = render_world_view(ui, &mut editor_state, &mut project);

                    // Handle world view results
                    if let Some(level_id) = world_result.open_level {
                        editor_state.selected_level = Some(level_id);
                        editor_state.view_mode = crate::EditorViewMode::Level;
                    }
                    if let Some(level_id) = world_result.delete_level {
                        project.remove_level(level_id);
                        if editor_state.selected_level == Some(level_id) {
                            editor_state.selected_level = project.levels.first().map(|l| l.id);
                        }
                    }
                    if let Some(level_id) = world_result.duplicate_level {
                        if let Some(new_id) = project.duplicate_level(level_id) {
                            editor_state.selected_level = Some(new_id);
                            // Offset the duplicated level
                            if let Some(level) = project.get_level_mut(new_id) {
                                level.world_x += 64;
                                level.world_y += 64;
                            }
                        }
                    }
                    if let Some(params) = world_result.create_level {
                        let new_level = bevy_map_core::Level::new_at(
                            params.name,
                            params.width,
                            params.height,
                            params.world_x,
                            params.world_y,
                        );
                        let new_id = new_level.id;
                        project.add_level(new_level);
                        editor_state.selected_level = Some(new_id);
                    }
                    if let Some(level_id) = world_result.rename_level {
                        if let Some(level) = project.get_level(level_id) {
                            editor_state.rename_buffer = level.name.clone();
                            editor_state.renaming_item = Some(crate::RenamingItem::Level(level_id));
                        }
                    }
                }
                crate::EditorViewMode::Level => {
                    // Normal level editing - transparent to show Bevy rendering
                    render_viewport_overlay(ui, &editor_state);
                }
            }
        });

    // Dialogs
    render_dialogs(ctx, &mut editor_state, &mut project, &assets_base_path);

    // Terrain dialogs
    terrain::render_new_terrain_dialog(ctx, &mut editor_state, &mut project);
    terrain::render_new_terrain_set_dialog(ctx, &mut editor_state, &mut project);
    terrain::render_add_terrain_to_set_dialog(ctx, &mut editor_state, &mut project);

    // New level dialog (for World View)
    if let Some(params) = render_new_level_dialog(ctx, &mut editor_state) {
        let new_level = bevy_map_core::Level::new_at(
            params.name,
            params.width,
            params.height,
            params.world_x,
            params.world_y,
        );
        let new_id = new_level.id;
        project.add_level(new_level);
        editor_state.selected_level = Some(new_id);
    }

    // Tileset & Terrain Editor (modal window)
    render_tileset_editor(ctx, &mut editor_state, &mut project, Some(&tileset_cache));

    // SpriteSheet Editor (modal window) - for spritesheet setup
    if editor_state.show_spritesheet_editor {
        let result = render_spritesheet_editor(ctx, &mut editor_state.spritesheet_editor_state);
        if result.close {
            editor_state.show_spritesheet_editor = false;
        }
        // Save sprite data back when changed (only grid config)
        if result.changed {
            let sprite_data = editor_state.spritesheet_editor_state.get_sprite_data();

            if let Some(asset_id) = editor_state.spritesheet_editor_state.asset_id {
                // Save grid config back to project.sprite_sheets
                if let Some(sprite_sheet) = project.get_sprite_sheet_mut(asset_id) {
                    sprite_sheet.sheet_path = sprite_data.sheet_path.clone();
                    sprite_sheet.frame_width = sprite_data.frame_width;
                    sprite_sheet.frame_height = sprite_data.frame_height;
                    sprite_sheet.columns = sprite_data.columns;
                    sprite_sheet.rows = sprite_data.rows;
                    sprite_sheet.pivot_x = sprite_data.pivot_x;
                    sprite_sheet.pivot_y = sprite_data.pivot_y;
                    sprite_sheet.name = sprite_data.name.clone();
                }

                // Synchronize with Animation Editor if open with same asset
                if editor_state.show_animation_editor
                    && editor_state.animation_editor_state.asset_id == Some(asset_id)
                {
                    editor_state
                        .animation_editor_state
                        .refresh_grid_config(&sprite_data);
                }
            }
        }
        // Handle browse button click
        if result.browse_spritesheet {
            if let Some(path) = crate::ui::spritesheet_editor::open_spritesheet_dialog() {
                let relative_path = assets_base_path.to_relative(std::path::Path::new(&path));
                let relative_path_str = relative_path.to_string_lossy().to_string();
                editor_state.spritesheet_editor_state.sheet_path_input = relative_path_str.clone();
                editor_state.spritesheet_editor_state.sprite_data.sheet_path = relative_path_str;
                editor_state.spritesheet_editor_state.clear_texture();
            }
        }
    }

    // Animation Editor (modal window) - for animation definition
    if editor_state.show_animation_editor {
        let result = render_animation_editor(ctx, &mut editor_state.animation_editor_state);
        if result.close {
            editor_state.show_animation_editor = false;
        }
        // Save animations back when changed
        if result.changed {
            let sprite_data = editor_state.animation_editor_state.get_sprite_data();

            if let Some(asset_id) = editor_state.animation_editor_state.asset_id {
                // Save animations back to project.sprite_sheets
                if let Some(sprite_sheet) = project.get_sprite_sheet_mut(asset_id) {
                    sprite_sheet.animations = sprite_data.animations;
                }
            } else if let Some(instance_id) = editor_state.animation_editor_state.instance_id {
                // Save back to data instance property (inline editing)
                let property_name = editor_state.animation_editor_state.property_name.clone();
                if let Ok(json_value) = serde_json::to_value(&sprite_data) {
                    let value = bevy_map_core::Value::from_json(json_value);
                    if let Some(instance) = project.get_data_instance_mut(instance_id) {
                        instance.properties.insert(property_name, value);
                    }
                }
            }
        }
        // Handle request to open SpriteSheet Editor
        if result.open_spritesheet_editor {
            if let Some(asset_id) = editor_state.animation_editor_state.asset_id {
                if let Some(sprite_sheet) = project.get_sprite_sheet(asset_id) {
                    editor_state.spritesheet_editor_state =
                        SpriteSheetEditorState::from_sprite_data(sprite_sheet.clone(), asset_id);
                    editor_state.show_spritesheet_editor = true;
                }
            }
        }
    }

    // Dialogue Editor (modal window)
    if editor_state.show_dialogue_editor {
        let result = render_dialogue_editor(ctx, &mut editor_state.dialogue_editor_state);
        if result.close {
            editor_state.show_dialogue_editor = false;
            editor_state.dialogue_editor_asset_id = None;
        }
        // Save dialogue data back when changed
        if result.changed {
            let dialogue_tree = editor_state.dialogue_editor_state.get_dialogue_tree();

            // Check if editing a project-level asset
            if let Some(ref asset_id) = editor_state.dialogue_editor_asset_id {
                // Save back to project.dialogues
                if let Some(dialogue) = project.get_dialogue_mut(asset_id) {
                    *dialogue = dialogue_tree;
                }
            } else if let Some(instance_id) = editor_state.dialogue_editor_state.instance_id {
                // Save back to data instance property (inline editing)
                let property_name = editor_state.dialogue_editor_state.property_name.clone();
                if let Ok(json_value) = serde_json::to_value(&dialogue_tree) {
                    let value = bevy_map_core::Value::from_json(json_value);
                    if let Some(instance) = project.get_data_instance_mut(instance_id) {
                        instance.properties.insert(property_name, value);
                    }
                }
            }
        }
    }

    // Schema Editor (modal window)
    render_schema_editor(ctx, &mut editor_state, &mut project);
}

/// Render viewport overlay with selection info
fn render_viewport_overlay(ui: &mut egui::Ui, editor_state: &EditorState) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Tool: {:?}", editor_state.current_tool));
            if let Some(layer) = editor_state.selected_layer {
                ui.separator();
                ui.label(format!("Layer: {}", layer));
            }
            ui.separator();
            ui.label(format!("Zoom: {}%", (editor_state.zoom * 100.0) as i32));
        });
    });
}

/// System to process edit-related pending actions
fn process_edit_actions(
    mut editor_state: ResMut<EditorState>,
    mut project: ResMut<Project>,
    mut history: ResMut<CommandHistory>,
    mut clipboard: ResMut<TileClipboard>,
    mut render_state: ResMut<RenderState>,
) {
    // Check for pending edit actions
    let action = editor_state.pending_action.take();

    if let Some(action) = action {
        match action {
            PendingAction::Undo => {
                history.undo(&mut project, &mut render_state);
            }
            PendingAction::Redo => {
                history.redo(&mut project, &mut render_state);
            }
            PendingAction::Copy => {
                clipboard.copy_selection(&editor_state.tile_selection, &project, &editor_state);
            }
            PendingAction::Cut => {
                // Copy first
                clipboard.copy_selection(&editor_state.tile_selection, &project, &editor_state);
                // Then flag for deletion
                editor_state.pending_delete_selection = true;
            }
            PendingAction::Paste => {
                if clipboard.has_content() {
                    editor_state.is_pasting = true;
                }
            }
            PendingAction::SelectAll => {
                select_all_visible_tiles(&mut editor_state, &project);
            }
            // File operations are handled in dialogs.rs
            _ => {
                // Put the action back so dialogs.rs can handle it
                editor_state.pending_action = Some(action);
            }
        }
    }
}

/// Select all tiles in the current layer
fn select_all_visible_tiles(editor_state: &mut EditorState, project: &Project) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(_layer_idx) = editor_state.selected_layer else {
        return;
    };

    let Some(level) = project.levels.iter().find(|l| l.id == level_id) else {
        return;
    };

    editor_state.tile_selection.clear();
    editor_state.tile_selection.select_rectangle(
        level_id,
        editor_state.selected_layer.unwrap_or(0),
        0,
        0,
        level.width.saturating_sub(1),
        level.height.saturating_sub(1),
        false,
    );
}
