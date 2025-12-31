//! Map rendering for the editor viewport using bevy_ecs_tilemap
//!
//! This module uses GPU-accelerated tilemap rendering via bevy_ecs_tilemap,
//! matching the approach used in bevy_map_runtime for consistent rendering
//! between editor and game.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_map_core::{LayerData, OCCUPIED_CELL};
use std::collections::HashMap;
use uuid::Uuid;

use crate::project::Project;
use crate::tools::ViewportInputState;
use crate::ui::{EditorTool, Selection, TilesetTextureCache, ToolMode};
use crate::EditorState;

/// Plugin for map rendering
pub struct MapRenderPlugin;

impl Plugin for MapRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .init_resource::<RenderState>()
            .init_resource::<SelectionRenderState>()
            .init_resource::<TerrainPreviewCache>()
            .init_resource::<BrushPreviewCache>()
            .init_resource::<EntityRenderState>()
            .init_resource::<CollisionOverlayCache>()
            .add_systems(Update, sync_level_rendering)
            .add_systems(Update, sync_layer_visibility)
            .add_systems(Update, sync_grid_rendering)
            .add_systems(Update, sync_collision_rendering)
            .add_systems(Update, sync_selection_preview)
            .add_systems(Update, sync_tile_selection_highlights)
            .add_systems(Update, sync_terrain_preview)
            .add_systems(Update, sync_brush_preview)
            .add_systems(Update, sync_entity_rendering)
            .add_systems(Update, update_camera_from_editor_state);
    }
}

/// Tracks the current render state for bevy_ecs_tilemap-based rendering
#[derive(Resource, Default)]
pub struct RenderState {
    /// Currently rendered level ID
    pub rendered_level: Option<Uuid>,
    /// Tilemap entities: (level_id, layer_index, image_index) -> tilemap entity
    pub tilemap_entities: HashMap<(Uuid, usize, usize), Entity>,
    /// TileStorage for each tilemap (needed for tile updates)
    pub tile_storages: HashMap<(Uuid, usize, usize), TileStorage>,
    /// Grid line entities
    pub grid_entities: Vec<Entity>,
    /// Whether we need to rebuild the map
    pub needs_rebuild: bool,
    /// Last known layer visibility states for change detection
    pub layer_visibility: HashMap<(Uuid, usize), bool>,
    /// Last known grid visibility state
    pub last_grid_visible: bool,
    /// Last rendered level dimensions for grid
    pub last_grid_dimensions: Option<(u32, u32, u32)>, // (width, height, tile_size)
    /// Multi-cell tile sprites: (level_id, layer_index, x, y) -> sprite entity
    /// These are rendered as separate Sprites instead of TileBundle to span multiple cells
    pub multi_cell_sprites: HashMap<(Uuid, usize, u32, u32), Entity>,
}

impl RenderState {
    /// Mark the viewport as needing a rebuild
    pub fn mark_dirty(&mut self) {
        self.needs_rebuild = true;
    }

    /// Set the current level being rendered
    pub fn set_level(&mut self, level_id: Uuid) {
        if self.rendered_level != Some(level_id) {
            self.rendered_level = Some(level_id);
            self.needs_rebuild = true;
        }
    }
}

/// Marker component for editor tilemaps
#[derive(Component)]
pub struct EditorTilemap {
    pub level_id: Uuid,
    pub layer_index: usize,
    pub image_index: usize,
}

/// Marker component for the grid overlay
#[derive(Component)]
pub struct GridLine;

/// Marker component for the selection rectangle preview
#[derive(Component)]
pub struct SelectionPreview;

/// Marker component for collision shape overlays
#[derive(Component)]
pub struct CollisionOverlay;

/// Marker component for multi-cell tile sprites
#[derive(Component)]
pub struct MultiCellTileSprite {
    pub level_id: Uuid,
    pub layer_index: usize,
    pub x: u32,
    pub y: u32,
}

/// Cache for collision overlay entities (for efficient updates)
#[derive(Resource, Default)]
pub struct CollisionOverlayCache {
    /// Collision overlay entities
    pub entities: Vec<Entity>,
    /// Last known show_collisions state
    pub last_visible: bool,
    /// Last known level ID
    pub last_level: Option<Uuid>,
}

/// System to sync level rendering with the project data
fn sync_level_rendering(
    mut commands: Commands,
    mut render_state: ResMut<RenderState>,
    editor_state: Res<EditorState>,
    project: Res<Project>,
    tileset_cache: Res<TilesetTextureCache>,
    tilemap_query: Query<Entity, With<EditorTilemap>>,
    asset_server: Res<AssetServer>,
) {
    let current_level_id = editor_state.selected_level;

    // Check if we need to switch levels
    if render_state.rendered_level != current_level_id {
        // Despawn all tile entities from storages first (safe - entity may not exist)
        for storage in render_state.tile_storages.values() {
            for tile_entity in storage.iter().flatten() {
                let _ = commands.get_entity(*tile_entity).map(|mut e| e.despawn());
            }
        }
        // Despawn all tilemap entities (safe - entity may not exist)
        for entity in tilemap_query.iter() {
            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
        }
        // Despawn multi-cell tile sprites (safe - entity may not exist)
        for entity in render_state.multi_cell_sprites.values() {
            let _ = commands.get_entity(*entity).map(|mut e| e.despawn());
        }
        render_state.tilemap_entities.clear();
        render_state.tile_storages.clear();
        render_state.multi_cell_sprites.clear();
        render_state.layer_visibility.clear();
        render_state.rendered_level = current_level_id;
        render_state.needs_rebuild = true;
    }

    // Get the current level
    let Some(level_id) = current_level_id else {
        return;
    };

    // Use O(1) lookup instead of iter().find()
    let Some(level) = project.get_level(level_id) else {
        return;
    };

    // Rebuild if needed
    if render_state.needs_rebuild {
        // Despawn all tile entities from storages first (safe - entity may not exist)
        for storage in render_state.tile_storages.values() {
            for tile_entity in storage.iter().flatten() {
                let _ = commands.get_entity(*tile_entity).map(|mut e| e.despawn());
            }
        }
        // Despawn all tilemap entities (safe - entity may not exist)
        for entity in tilemap_query.iter() {
            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
        }
        // Despawn multi-cell tile sprites (safe - entity may not exist)
        for entity in render_state.multi_cell_sprites.values() {
            let _ = commands.get_entity(*entity).map(|mut e| e.despawn());
        }
        render_state.tilemap_entities.clear();
        render_state.tile_storages.clear();
        render_state.multi_cell_sprites.clear();

        spawn_level_tilemaps(
            &mut commands,
            &mut render_state,
            level,
            &project,
            &tileset_cache,
            &asset_server,
        );
        render_state.needs_rebuild = false;
    }
}

/// System to sync layer visibility
fn sync_layer_visibility(
    editor_state: Res<EditorState>,
    project: Res<Project>,
    mut render_state: ResMut<RenderState>,
    mut tilemap_query: Query<(&EditorTilemap, &mut Visibility), Without<MultiCellTileSprite>>,
    mut multi_cell_query: Query<(&MultiCellTileSprite, &mut Visibility), Without<EditorTilemap>>,
) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };

    // Use O(1) lookup
    let Some(level) = project.get_level(level_id) else {
        return;
    };

    // Check for layer visibility changes and update tilemaps
    for (layer_index, layer) in level.layers.iter().enumerate() {
        let key = (level_id, layer_index);
        let old_vis = render_state.layer_visibility.get(&key).copied();

        if old_vis != Some(layer.visible) {
            render_state.layer_visibility.insert(key, layer.visible);

            let new_visibility = if layer.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };

            // Update visibility of all tilemaps for this layer
            for (editor_tilemap, mut visibility) in tilemap_query.iter_mut() {
                if editor_tilemap.level_id == level_id && editor_tilemap.layer_index == layer_index
                {
                    *visibility = new_visibility;
                }
            }

            // Update visibility of multi-cell tile sprites for this layer
            for (multi_cell_sprite, mut visibility) in multi_cell_query.iter_mut() {
                if multi_cell_sprite.level_id == level_id
                    && multi_cell_sprite.layer_index == layer_index
                {
                    *visibility = new_visibility;
                }
            }
        }
    }
}

/// Spawn tilemaps for a level using bevy_ecs_tilemap
fn spawn_level_tilemaps(
    commands: &mut Commands,
    render_state: &mut RenderState,
    level: &bevy_map_core::Level,
    project: &Project,
    tileset_cache: &TilesetTextureCache,
    asset_server: &AssetServer,
) {
    for (layer_index, layer) in level.layers.iter().enumerate() {
        // Skip non-tile layers
        let LayerData::Tiles {
            tileset_id, tiles, ..
        } = &layer.data
        else {
            continue;
        };

        // Get tileset info (O(1) lookup)
        let Some(tileset) = project.get_tileset(*tileset_id) else {
            continue;
        };

        let tile_size = tileset.tile_size;
        let tile_size_f32 = tile_size as f32;

        // Group tiles by image (for multi-image tilesets)
        // bevy_ecs_tilemap uses a single texture per tilemap, so we need separate tilemaps per image
        // Also track which tiles are multi-cell (they'll be rendered as Sprites instead)
        let mut tiles_by_image: HashMap<usize, Vec<(u32, u32, u32)>> = HashMap::new();
        let mut multi_cell_tiles: Vec<(u32, u32, u32, u32, u32, usize)> = Vec::new(); // (x, y, virtual_idx, grid_w, grid_h, image_index)

        for y in 0..level.height {
            for x in 0..level.width {
                let index = (y * level.width + x) as usize;
                if let Some(Some(virtual_tile_index)) = tiles.get(index) {
                    // Skip OCCUPIED_CELL sentinel values (used for multi-cell tiles)
                    if *virtual_tile_index == OCCUPIED_CELL {
                        continue;
                    }

                    // Check if this is a multi-cell tile
                    let (grid_width, grid_height) = tileset.get_tile_grid_size(*virtual_tile_index);

                    if grid_width > 1 || grid_height > 1 {
                        // Multi-cell tile - will be rendered as Sprite
                        if let Some((image_index, _)) =
                            tileset.virtual_to_local(*virtual_tile_index)
                        {
                            multi_cell_tiles.push((
                                x,
                                y,
                                *virtual_tile_index,
                                grid_width,
                                grid_height,
                                image_index,
                            ));
                        }
                    } else {
                        // Regular 1x1 tile - use TileBundle
                        if let Some((image_index, local_tile_index)) =
                            tileset.virtual_to_local(*virtual_tile_index)
                        {
                            tiles_by_image.entry(image_index).or_default().push((
                                x,
                                y,
                                local_tile_index,
                            ));
                        }
                    }
                }
            }
        }

        // Spawn a tilemap for each image used in this layer (for regular 1x1 tiles)
        for (image_index, image_tiles) in tiles_by_image {
            // Get texture handle for this image
            let texture_handle = if let Some(image) = tileset.images.get(image_index) {
                if let Some((handle, _, _, _)) = tileset_cache.loaded.get(&image.id) {
                    handle.clone()
                } else {
                    asset_server.load(crate::to_asset_path(&image.path))
                }
            } else if let Some(path) = tileset.primary_path() {
                asset_server.load(crate::to_asset_path(path))
            } else {
                continue;
            };

            let map_size = TilemapSize {
                x: level.width,
                y: level.height,
            };

            let tilemap_tile_size = TilemapTileSize {
                x: tile_size_f32,
                y: tile_size_f32,
            };

            let grid_size: TilemapGridSize = tilemap_tile_size.into();
            let mut tile_storage = TileStorage::empty(map_size);
            let tilemap_entity = commands.spawn_empty().id();

            // Spawn tiles for this image
            for (x, y, local_tile_index) in &image_tiles {
                let tile_pos = TilePos { x: *x, y: *y };
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(*local_tile_index),
                        ..default()
                    })
                    .id();
                tile_storage.set(&tile_pos, tile_entity);
            }

            // Z-offset: layer_index * 0.1 + image_index * 0.01
            // This ensures proper ordering: all images in layer 0 render before layer 1
            let layer_z = layer_index as f32 * 0.1 + image_index as f32 * 0.01;

            // Insert TilemapBundle first (which includes Visibility internally)
            // Use BottomLeft anchor so tiles at (0,0) start at world origin
            commands.entity(tilemap_entity).insert((
                TilemapBundle {
                    grid_size,
                    map_type: TilemapType::Square,
                    size: map_size,
                    storage: tile_storage.clone(),
                    texture: TilemapTexture::Single(texture_handle),
                    tile_size: tilemap_tile_size,
                    transform: Transform::from_xyz(0.0, 0.0, layer_z),
                    anchor: TilemapAnchor::BottomLeft,
                    visibility: if layer.visible {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    ..default()
                },
                EditorTilemap {
                    level_id: level.id,
                    layer_index,
                    image_index,
                },
            ));

            // Store references for later updates
            let key = (level.id, layer_index, image_index);
            render_state.tilemap_entities.insert(key, tilemap_entity);
            render_state.tile_storages.insert(key, tile_storage);
        }

        // Spawn multi-cell tiles as Sprites
        for (x, y, virtual_tile_index, grid_width, grid_height, image_index) in multi_cell_tiles {
            // Get texture handle and image info for this tile
            let Some(image) = tileset.images.get(image_index) else {
                continue;
            };

            let texture_handle =
                if let Some((handle, _, _, _)) = tileset_cache.loaded.get(&image.id) {
                    handle.clone()
                } else {
                    // Image not loaded yet, skip for now (will be rendered on rebuild)
                    continue;
                };

            // Calculate local tile position in the tileset image
            let (_, local_tile_index) = tileset.virtual_to_local(virtual_tile_index).unwrap();
            let tile_col = local_tile_index % image.columns;
            let tile_row = local_tile_index / image.columns;

            // Source rect in texture coordinates (pixels)
            // Note: In Bevy textures, Y=0 is at top, but we need to flip for correct sampling
            let src_x = (tile_col * tile_size) as f32;
            let src_y = (tile_row * tile_size) as f32;
            let src_width = (grid_width * tile_size) as f32;
            let src_height = (grid_height * tile_size) as f32;

            // Create a rect for the source region
            let rect = bevy::math::Rect::new(src_x, src_y, src_x + src_width, src_y + src_height);

            // Get origin point (defaults to center if not set)
            let props = tileset
                .get_tile_properties(virtual_tile_index)
                .cloned()
                .unwrap_or_default();
            let (origin_x, origin_y) = props.get_origin(src_width as u32, src_height as u32);

            // World position: place sprite so origin aligns with grid cell corner
            // For center origin (size/2): sprite center at grid + size/2 (standard behavior)
            // For top-left origin (0): sprite center at grid + 0 (tile shifts left/down)
            let world_x = x as f32 * tile_size_f32 + origin_x as f32;
            let world_y = y as f32 * tile_size_f32 + origin_y as f32;

            // Z-offset slightly above regular tiles in same layer
            let layer_z = layer_index as f32 * 0.1 + image_index as f32 * 0.01 + 0.001;

            let sprite_entity = commands
                .spawn((
                    Sprite {
                        image: texture_handle,
                        rect: Some(rect),
                        custom_size: Some(Vec2::new(src_width, src_height)),
                        ..default()
                    },
                    Transform::from_xyz(world_x, world_y, layer_z),
                    Visibility::Inherited,
                    MultiCellTileSprite {
                        level_id: level.id,
                        layer_index,
                        x,
                        y,
                    },
                ))
                .id();

            // Store reference for updates
            render_state
                .multi_cell_sprites
                .insert((level.id, layer_index, x, y), sprite_entity);
        }

        // Store layer visibility
        render_state
            .layer_visibility
            .insert((level.id, layer_index), layer.visible);
    }
}

/// Update a single tile in the rendered tilemap
///
/// This function updates both the visual representation and the TileStorage.
/// Call this when painting tiles to avoid full rebuilds.
pub fn update_tile(
    commands: &mut Commands,
    render_state: &mut RenderState,
    project: &Project,
    tileset_cache: &TilesetTextureCache,
    level_id: Uuid,
    layer_index: usize,
    x: u32,
    y: u32,
    new_tile_index: Option<u32>,
) {
    // Get the level and layer to find the tileset (O(1) lookups)
    let Some(level) = project.get_level(level_id) else {
        return;
    };

    let Some(layer) = level.layers.get(layer_index) else {
        return;
    };

    let LayerData::Tiles { tileset_id, .. } = &layer.data else {
        return;
    };

    let Some(tileset) = project.get_tileset(*tileset_id) else {
        return;
    };

    let tile_size = tileset.tile_size;
    let tile_size_f32 = tile_size as f32;
    let tile_pos = TilePos { x, y };

    // Skip rendering OCCUPIED_CELL sentinel values (used for multi-cell tiles)
    // Treat them as empty cells
    let effective_tile_index = new_tile_index.filter(|&idx| idx != OCCUPIED_CELL);

    // Only remove existing multi-cell sprite if we're placing a real tile here
    // (not for OCCUPIED_CELL or None, which shouldn't affect other tiles)
    if effective_tile_index.is_some() {
        let multi_cell_key = (level_id, layer_index, x, y);
        if let Some(entity) = render_state.multi_cell_sprites.remove(&multi_cell_key) {
            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
        }
    }

    if let Some(tile_idx) = effective_tile_index {
        // Check if this is a multi-cell tile
        let (grid_width, grid_height) = tileset.get_tile_grid_size(tile_idx);

        if grid_width > 1 || grid_height > 1 {
            // Multi-cell tile - render as Sprite
            // Note: Only remove sprite at exact same position (already handled above at lines 507-511)
            // Overlapping tiles are preserved - no cleanup needed here

            if let Some((image_index, local_idx)) = tileset.virtual_to_local(tile_idx) {
                // Remove from regular tilemap storage if it was there
                for ((lid, li, _), storage) in render_state.tile_storages.iter_mut() {
                    if *lid == level_id && *li == layer_index {
                        if let Some(entity) = storage.get(&tile_pos) {
                            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
                            storage.remove(&tile_pos);
                        }
                    }
                }

                // Get texture handle
                if let Some(image) = tileset.images.get(image_index) {
                    if let Some((texture_handle, _, _, _)) = tileset_cache.loaded.get(&image.id) {
                        // Calculate tile position in tileset image
                        let tile_col = local_idx % image.columns;
                        let tile_row = local_idx / image.columns;

                        // Source rect
                        let src_x = (tile_col * tile_size) as f32;
                        let src_y = (tile_row * tile_size) as f32;
                        let src_width = (grid_width * tile_size) as f32;
                        let src_height = (grid_height * tile_size) as f32;

                        let rect = bevy::math::Rect::new(
                            src_x,
                            src_y,
                            src_x + src_width,
                            src_y + src_height,
                        );

                        // Get origin point (defaults to center if not set)
                        let props = tileset
                            .get_tile_properties(tile_idx)
                            .cloned()
                            .unwrap_or_default();
                        let (origin_x, origin_y) =
                            props.get_origin(src_width as u32, src_height as u32);

                        // World position: place sprite so origin aligns with grid cell corner
                        let world_x = x as f32 * tile_size_f32 + origin_x as f32;
                        let world_y = y as f32 * tile_size_f32 + origin_y as f32;
                        let layer_z = layer_index as f32 * 0.1 + image_index as f32 * 0.01 + 0.001;

                        let sprite_entity = commands
                            .spawn((
                                Sprite {
                                    image: texture_handle.clone(),
                                    rect: Some(rect),
                                    custom_size: Some(Vec2::new(src_width, src_height)),
                                    ..default()
                                },
                                Transform::from_xyz(world_x, world_y, layer_z),
                                Visibility::Inherited,
                                MultiCellTileSprite {
                                    level_id,
                                    layer_index,
                                    x,
                                    y,
                                },
                            ))
                            .id();

                        render_state
                            .multi_cell_sprites
                            .insert((level_id, layer_index, x, y), sprite_entity);
                    }
                }
            }
        } else {
            // Regular 1x1 tile - use TileBundle
            if let Some((image_index, local_idx)) = tileset.virtual_to_local(tile_idx) {
                let key = (level_id, layer_index, image_index);

                // First, remove the tile from any other image's storage at this position
                // (in case we're changing which image the tile uses)
                for ((lid, li, img_idx), storage) in render_state.tile_storages.iter_mut() {
                    if *lid == level_id && *li == layer_index && *img_idx != image_index {
                        if let Some(old_entity) = storage.get(&tile_pos) {
                            let _ = commands.get_entity(old_entity).map(|mut e| e.despawn());
                            storage.remove(&tile_pos);
                        }
                    }
                }

                // Create tilemap on-demand if it doesn't exist
                if !render_state.tile_storages.contains_key(&key) {
                    // Get texture handle from cache
                    let texture_handle = tileset.images.get(image_index).and_then(|image| {
                        tileset_cache
                            .loaded
                            .get(&image.id)
                            .map(|(handle, _, _, _)| handle.clone())
                    });

                    if let Some(texture_handle) = texture_handle {
                        let map_size = TilemapSize {
                            x: level.width,
                            y: level.height,
                        };

                        let tilemap_tile_size = TilemapTileSize {
                            x: tile_size_f32,
                            y: tile_size_f32,
                        };

                        let grid_size: TilemapGridSize = tilemap_tile_size.into();
                        let tile_storage = TileStorage::empty(map_size);
                        let tilemap_entity = commands.spawn_empty().id();

                        let layer_z = layer_index as f32 * 0.1 + image_index as f32 * 0.01;
                        let layer_visible = layer.visible;

                        commands.entity(tilemap_entity).insert((
                            TilemapBundle {
                                grid_size,
                                map_type: TilemapType::Square,
                                size: map_size,
                                storage: tile_storage.clone(),
                                texture: TilemapTexture::Single(texture_handle),
                                tile_size: tilemap_tile_size,
                                transform: Transform::from_xyz(0.0, 0.0, layer_z),
                                anchor: TilemapAnchor::BottomLeft,
                                visibility: if layer_visible {
                                    Visibility::Inherited
                                } else {
                                    Visibility::Hidden
                                },
                                ..default()
                            },
                            EditorTilemap {
                                level_id,
                                layer_index,
                                image_index,
                            },
                        ));

                        render_state.tilemap_entities.insert(key, tilemap_entity);
                        render_state.tile_storages.insert(key, tile_storage);
                    }
                }

                // Now update the correct tilemap
                if let Some(storage) = render_state.tile_storages.get_mut(&key) {
                    let tilemap_entity = render_state.tilemap_entities[&key];

                    // Remove old tile if exists
                    if let Some(old_entity) = storage.get(&tile_pos) {
                        let _ = commands.get_entity(old_entity).map(|mut e| e.despawn());
                    }

                    // Spawn new tile
                    let new_tile = commands
                        .spawn(TileBundle {
                            position: tile_pos,
                            tilemap_id: TilemapId(tilemap_entity),
                            texture_index: TileTextureIndex(local_idx),
                            ..default()
                        })
                        .id();
                    storage.set(&tile_pos, new_tile);
                }
            }
        }
    } else {
        // Erase: remove tile from all image tilemaps for this layer
        for ((lid, li, _), storage) in render_state.tile_storages.iter_mut() {
            if *lid == level_id && *li == layer_index {
                if let Some(entity) = storage.get(&tile_pos) {
                    let _ = commands.get_entity(entity).map(|mut e| e.despawn());
                    storage.remove(&tile_pos);
                }
            }
        }
    }
}

/// System to render grid overlay (sprite-based, on top of tilemaps)
fn sync_grid_rendering(
    mut commands: Commands,
    mut render_state: ResMut<RenderState>,
    editor_state: Res<EditorState>,
    project: Res<Project>,
) {
    let show_grid = editor_state.show_grid;

    // Get current level info
    let level_info = editor_state.selected_level.and_then(|level_id| {
        project
            .levels
            .iter()
            .find(|l| l.id == level_id)
            .map(|level| {
                // Get tile size from first tile layer's tileset, or default
                let tile_size = level
                    .layers
                    .iter()
                    .find_map(|layer| {
                        if let LayerData::Tiles { tileset_id, .. } = &layer.data {
                            project
                                .tilesets
                                .iter()
                                .find(|t| t.id == *tileset_id)
                                .map(|t| t.tile_size)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(32);
                (level.width, level.height, tile_size)
            })
    });

    // Check if we need to update grid
    let needs_update = show_grid != render_state.last_grid_visible
        || (show_grid && level_info != render_state.last_grid_dimensions);

    if !needs_update {
        return;
    }

    // Despawn existing grid
    for entity in render_state.grid_entities.drain(..) {
        let _ = commands.get_entity(entity).map(|mut e| e.despawn());
    }

    render_state.last_grid_visible = show_grid;
    render_state.last_grid_dimensions = level_info;

    if !show_grid {
        return;
    }

    let Some((width, height, tile_size)) = level_info else {
        return;
    };

    let tile_size_f32 = tile_size as f32;
    let grid_color = Color::srgba(0.5, 0.5, 0.5, 0.5);
    let line_thickness = 1.0;
    let grid_width = width as f32 * tile_size_f32;
    let grid_height = height as f32 * tile_size_f32;

    // Spawn vertical lines
    for x in 0..=width {
        let world_x = x as f32 * tile_size_f32;
        let center_y = grid_height / 2.0;
        let entity = commands
            .spawn((
                Sprite {
                    color: grid_color,
                    custom_size: Some(Vec2::new(line_thickness, grid_height)),
                    ..default()
                },
                Transform::from_xyz(world_x, center_y, 100.0),
                GridLine,
            ))
            .id();
        render_state.grid_entities.push(entity);
    }

    // Spawn horizontal lines
    for y in 0..=height {
        let world_y = y as f32 * tile_size_f32;
        let center_x = grid_width / 2.0;
        let entity = commands
            .spawn((
                Sprite {
                    color: grid_color,
                    custom_size: Some(Vec2::new(grid_width, line_thickness)),
                    ..default()
                },
                Transform::from_xyz(center_x, world_y, 100.0),
                GridLine,
            ))
            .id();
        render_state.grid_entities.push(entity);
    }
}

/// System to render collision shape overlays on tiles
fn sync_collision_rendering(
    mut commands: Commands,
    mut cache: ResMut<CollisionOverlayCache>,
    editor_state: Res<EditorState>,
    project: Res<Project>,
) {
    let show_collisions = editor_state.show_collisions;
    let current_level = editor_state.selected_level;

    // Check if we need to update
    let needs_update = show_collisions != cache.last_visible || current_level != cache.last_level;

    if !needs_update && !show_collisions {
        return;
    }

    // Despawn existing collision overlays
    for entity in cache.entities.drain(..) {
        let _ = commands.get_entity(entity).map(|mut e| e.despawn());
    }

    cache.last_visible = show_collisions;
    cache.last_level = current_level;

    if !show_collisions {
        return;
    }

    let Some(level_id) = current_level else {
        return;
    };

    // Use O(1) lookup instead of iter().find()
    let Some(level) = project.get_level(level_id) else {
        return;
    };

    let collision_color = Color::srgba(0.0, 0.6, 1.0, 0.3);

    // Iterate through tile layers
    for (layer_idx, layer) in level.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        if let bevy_map_core::LayerData::Tiles {
            tileset_id, tiles, ..
        } = &layer.data
        {
            // Get the tileset (O(1) lookup)
            let Some(tileset) = project.get_tileset(*tileset_id) else {
                continue;
            };

            let tile_size = tileset.tile_size as f32;

            // Iterate through tiles
            for y in 0..level.height {
                for x in 0..level.width {
                    let idx = (y * level.width + x) as usize;
                    if let Some(&Some(tile_index)) = tiles.get(idx) {
                        // Check if this tile has collision
                        if let Some(props) = tileset.get_tile_properties(tile_index) {
                            if props.collision.has_collision() {
                                // Spawn collision overlay sprite(s)
                                spawn_collision_overlay(
                                    &mut commands,
                                    &mut cache,
                                    &props.collision.shape,
                                    x,
                                    y,
                                    tile_size,
                                    layer_idx,
                                    collision_color,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Spawn collision overlay sprite(s) for a single tile
fn spawn_collision_overlay(
    commands: &mut Commands,
    cache: &mut CollisionOverlayCache,
    shape: &bevy_map_core::CollisionShape,
    tile_x: u32,
    tile_y: u32,
    tile_size: f32,
    layer_idx: usize,
    color: Color,
) {
    // Calculate world position (tiles are positioned from bottom-left)
    let base_x = tile_x as f32 * tile_size;
    let base_y = tile_y as f32 * tile_size;
    let z = 101.0 + layer_idx as f32 * 0.01; // Just above grid (100.0)

    match shape {
        bevy_map_core::CollisionShape::None => {}
        bevy_map_core::CollisionShape::Full => {
            // Full tile collision - single sprite covering the tile
            let entity = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(tile_size, tile_size)),
                        ..default()
                    },
                    Transform::from_xyz(base_x + tile_size / 2.0, base_y + tile_size / 2.0, z),
                    CollisionOverlay,
                ))
                .id();
            cache.entities.push(entity);
        }
        bevy_map_core::CollisionShape::Rectangle { offset, size } => {
            // Rectangle at offset with size (both normalized 0-1)
            // Flip Y: editor uses Y-down (top=0), Bevy uses Y-up (bottom=0)
            let width = size[0] * tile_size;
            let height = size[1] * tile_size;
            let center_x = base_x + (offset[0] + size[0] / 2.0) * tile_size;
            let center_y = base_y + (1.0 - offset[1] - size[1] / 2.0) * tile_size;

            let entity = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(width, height)),
                        ..default()
                    },
                    Transform::from_xyz(center_x, center_y, z),
                    CollisionOverlay,
                ))
                .id();
            cache.entities.push(entity);
        }
        bevy_map_core::CollisionShape::Circle { offset, radius } => {
            // Circle - approximate with a square sprite for now
            // Could use a circle texture or shader in the future
            // Flip Y: editor uses Y-down (top=0), Bevy uses Y-up (bottom=0)
            let diameter = radius * 2.0 * tile_size;
            let center_x = base_x + offset[0] * tile_size;
            let center_y = base_y + (1.0 - offset[1]) * tile_size;

            let entity = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(diameter, diameter)),
                        ..default()
                    },
                    Transform::from_xyz(center_x, center_y, z),
                    CollisionOverlay,
                ))
                .id();
            cache.entities.push(entity);
        }
        bevy_map_core::CollisionShape::Polygon { points } => {
            // Polygon - draw lines connecting vertices
            if points.len() < 2 {
                return;
            }

            let line_thickness = 2.0;
            let line_color = Color::srgba(0.0, 0.6, 1.0, 0.6); // Slightly more opaque for lines

            for i in 0..points.len() {
                let p1 = &points[i];
                let p2 = &points[(i + 1) % points.len()];

                // Convert normalized coords to world coords
                // Flip Y: editor uses Y-down (top=0), Bevy uses Y-up (bottom=0)
                let x1 = base_x + p1[0] * tile_size;
                let y1 = base_y + (1.0 - p1[1]) * tile_size;
                let x2 = base_x + p2[0] * tile_size;
                let y2 = base_y + (1.0 - p2[1]) * tile_size;

                // Calculate line center, length, and angle
                let center_x = (x1 + x2) / 2.0;
                let center_y = (y1 + y2) / 2.0;
                let dx = x2 - x1;
                let dy = y2 - y1;
                let length = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                let entity = commands
                    .spawn((
                        Sprite {
                            color: line_color,
                            custom_size: Some(Vec2::new(length, line_thickness)),
                            ..default()
                        },
                        Transform::from_xyz(center_x, center_y, z)
                            .with_rotation(Quat::from_rotation_z(angle)),
                        CollisionOverlay,
                    ))
                    .id();
                cache.entities.push(entity);
            }
        }
    }
}

/// System to update camera based on editor state
fn update_camera_from_editor_state(
    editor_state: Res<EditorState>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut projection_query: Query<&mut Projection, With<Camera2d>>,
) {
    // Update camera position
    for mut transform in camera_query.iter_mut() {
        transform.translation.x = editor_state.camera_offset.x;
        transform.translation.y = editor_state.camera_offset.y;
    }

    // Update zoom via projection
    for mut projection in projection_query.iter_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = 1.0 / editor_state.zoom;
        }
    }
}

/// System to render the selection rectangle preview
fn sync_selection_preview(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    input_state: Option<Res<ViewportInputState>>,
    project: Res<Project>,
    existing_preview: Query<Entity, With<SelectionPreview>>,
) {
    // Always despawn existing preview first
    for entity in existing_preview.iter() {
        let _ = commands.get_entity(entity).map(|mut e| e.despawn());
    }

    // If ViewportInputState doesn't exist, skip
    let Some(input_state) = input_state else {
        return;
    };

    // Only show preview when actively drawing in Rectangle mode
    let is_rectangle_mode =
        editor_state.tool_mode == ToolMode::Rectangle && editor_state.current_tool.supports_modes();
    if !is_rectangle_mode || !input_state.is_drawing_rect {
        return;
    }

    let Some((start_x, start_y)) = input_state.rect_start_tile else {
        return;
    };

    let Some(current_pos) = input_state.last_world_pos else {
        return;
    };

    // Get tile size
    let tile_size = get_tile_size(&editor_state, &project);

    // Calculate end tile position
    let end_x = (current_pos.x / tile_size).floor() as i32;
    let end_y = (current_pos.y / tile_size).floor() as i32;

    // Normalize bounds
    let min_x = start_x.min(end_x);
    let max_x = start_x.max(end_x);
    let min_y = start_y.min(end_y);
    let max_y = start_y.max(end_y);

    // Calculate world coordinates for the rectangle
    let world_min_x = min_x as f32 * tile_size;
    let world_max_x = (max_x + 1) as f32 * tile_size;
    let world_min_y = min_y as f32 * tile_size;
    let world_max_y = (max_y + 1) as f32 * tile_size;

    let width = world_max_x - world_min_x;
    let height = world_max_y - world_min_y;
    let center_x = world_min_x + width / 2.0;
    let center_y = world_min_y + height / 2.0;

    // Choose color based on whether we're filling or erasing
    let color = if editor_state.selected_tile.is_some() {
        Color::srgba(0.2, 0.4, 0.8, 0.4) // Blue for fill
    } else {
        Color::srgba(0.8, 0.2, 0.2, 0.4) // Red for erase
    };

    // Spawn the preview rectangle
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_xyz(center_x, center_y, 200.0), // High Z to render on top
        SelectionPreview,
    ));
}

/// Get the tile size for the current level/layer/tileset (for preview rendering)
fn get_tile_size(editor_state: &EditorState, project: &Project) -> f32 {
    let level_id = editor_state.selected_level;
    let layer_idx = editor_state.selected_layer;

    // Use O(1) lookup for level
    let level = level_id.and_then(|id| project.get_level(id));
    let layer_tileset_id = level.and_then(|l| {
        layer_idx
            .and_then(|idx| l.layers.get(idx))
            .and_then(|layer| {
                if let LayerData::Tiles { tileset_id, .. } = &layer.data {
                    Some(*tileset_id)
                } else {
                    None
                }
            })
    });

    // Use O(1) lookup for tileset
    layer_tileset_id
        .or(editor_state.selected_tileset)
        .and_then(|id| project.get_tileset(id))
        .map(|t| t.tile_size as f32)
        .unwrap_or(32.0)
}

/// Resource tracking the current selection highlight state for change detection
#[derive(Resource, Default)]
pub struct SelectionRenderState {
    /// Set of currently highlighted tiles (level_id, layer_idx, x, y)
    pub highlighted_tiles: std::collections::HashSet<(Uuid, usize, u32, u32)>,
    /// 4 border entities for the bounding rectangle (top, right, bottom, left)
    pub border_entities: Option<[Entity; 4]>,
    /// Current move offset being applied to highlights
    pub current_move_offset: Option<(i32, i32)>,
    /// Cached bounding box (min_x, max_x, min_y, max_y) in tile coordinates
    pub cached_bounds: Option<(i32, i32, i32, i32)>,
}

/// Resource for caching terrain preview entities to avoid respawning every frame
#[derive(Resource, Default)]
pub struct TerrainPreviewCache {
    /// Current preview tiles being displayed: (x, y) -> tile_id
    pub current_tiles: HashMap<(i32, i32), u32>,
    /// Entities for each preview tile (position -> list of entities for tile+highlight+borders)
    pub tile_entities: HashMap<(i32, i32), Vec<Entity>>,
    /// Whether preview was active last frame
    pub was_active: bool,
}

/// Resource for caching brush preview entities
#[derive(Resource, Default)]
pub struct BrushPreviewCache {
    /// Sprite entity for the preview tile
    pub sprite_entity: Option<Entity>,
    /// Border entities for the preview
    pub border_entities: Vec<Entity>,
    /// Last rendered position
    pub last_position: Option<(i32, i32)>,
    /// Last rendered tile
    pub last_tile: Option<u32>,
    /// Last tileset
    pub last_tileset: Option<Uuid>,
}

/// Marker component for brush preview sprites
#[derive(Component)]
pub struct BrushPreviewSprite;

/// Marker component for tile selection highlight sprites
#[derive(Component)]
pub struct TileSelectionHighlight;

/// System to render tile selection highlights with a calm bounding box outline
fn sync_tile_selection_highlights(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    project: Res<Project>,
    mut selection_state: ResMut<SelectionRenderState>,
    time: Res<Time>,
    mut sprite_query: Query<(&mut Sprite, &mut Transform), With<TileSelectionHighlight>>,
) {
    let current_selection = &editor_state.tile_selection.tiles;

    // Helper to clear all highlight entities
    let clear_highlights = |commands: &mut Commands, state: &mut SelectionRenderState| {
        if let Some(entities) = state.border_entities.take() {
            for entity in entities {
                let _ = commands.get_entity(entity).map(|mut e| e.despawn());
            }
        }
        state.highlighted_tiles.clear();
        state.current_move_offset = None;
        state.cached_bounds = None;
    };

    // Hide tile selection when Entity tool is active (tile selection is irrelevant for entities)
    if editor_state.current_tool == EditorTool::Entity {
        clear_highlights(&mut commands, &mut selection_state);
        return;
    }

    // If no tile selection, clear any existing highlights and return early
    if current_selection.is_empty() {
        clear_highlights(&mut commands, &mut selection_state);
        return;
    }

    let tile_size = get_tile_size(&editor_state, &project);
    let current_offset = editor_state.tile_move_offset;
    let (offset_x, offset_y) = current_offset.unwrap_or((0, 0));
    let elapsed_time = time.elapsed_secs();

    // Calm cyan animation - slow alpha pulse (2 second cycle)
    let alpha = 0.85 + 0.15 * (elapsed_time * std::f32::consts::PI).sin();
    let color = Color::srgba(0.0, 0.8, 1.0, alpha);

    // Calculate bounding box of all selected tiles
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for (level_id, _, x, y) in current_selection.iter() {
        // Only consider tiles from the currently selected level
        if Some(*level_id) != editor_state.selected_level {
            continue;
        }
        min_x = min_x.min(*x as i32);
        max_x = max_x.max(*x as i32);
        min_y = min_y.min(*y as i32);
        max_y = max_y.max(*y as i32);
    }

    // If no valid tiles found for current level, clear highlights
    if min_x == i32::MAX {
        clear_highlights(&mut commands, &mut selection_state);
        return;
    }

    let new_bounds = (min_x, max_x, min_y, max_y);
    let bounds_changed = selection_state.cached_bounds != Some(new_bounds);
    let offset_changed = selection_state.current_move_offset != current_offset;

    // Update colors on all existing sprites for animation
    for (mut sprite, _) in sprite_query.iter_mut() {
        sprite.color = color;
    }

    // If bounds or offset changed, we need to update or recreate the border sprites
    if bounds_changed || offset_changed {
        // Calculate world coordinates for bounding box (apply move offset)
        let world_min_x = (min_x + offset_x) as f32 * tile_size;
        let world_max_x = ((max_x + 1) + offset_x) as f32 * tile_size;
        let world_min_y = (min_y + offset_y) as f32 * tile_size;
        let world_max_y = ((max_y + 1) + offset_y) as f32 * tile_size;

        let width = world_max_x - world_min_x;
        let height = world_max_y - world_min_y;
        let center_x = world_min_x + width / 2.0;
        let center_y = world_min_y + height / 2.0;
        let border_thickness = 2.0;

        if let Some(entities) = &selection_state.border_entities {
            // Update existing border positions
            // Top border
            if let Ok((_, mut transform)) = sprite_query.get_mut(entities[0]) {
                transform.translation.x = center_x;
                transform.translation.y = world_max_y - border_thickness / 2.0;
            }
            // Right border
            if let Ok((_, mut transform)) = sprite_query.get_mut(entities[1]) {
                transform.translation.x = world_max_x - border_thickness / 2.0;
                transform.translation.y = center_y;
            }
            // Bottom border
            if let Ok((_, mut transform)) = sprite_query.get_mut(entities[2]) {
                transform.translation.x = center_x;
                transform.translation.y = world_min_y + border_thickness / 2.0;
            }
            // Left border
            if let Ok((_, mut transform)) = sprite_query.get_mut(entities[3]) {
                transform.translation.x = world_min_x + border_thickness / 2.0;
                transform.translation.y = center_y;
            }

            // Also update sizes if bounds changed
            if bounds_changed {
                if let Ok((mut sprite, _)) = sprite_query.get_mut(entities[0]) {
                    sprite.custom_size = Some(Vec2::new(width, border_thickness));
                }
                if let Ok((mut sprite, _)) = sprite_query.get_mut(entities[1]) {
                    sprite.custom_size = Some(Vec2::new(border_thickness, height));
                }
                if let Ok((mut sprite, _)) = sprite_query.get_mut(entities[2]) {
                    sprite.custom_size = Some(Vec2::new(width, border_thickness));
                }
                if let Ok((mut sprite, _)) = sprite_query.get_mut(entities[3]) {
                    sprite.custom_size = Some(Vec2::new(border_thickness, height));
                }
            }
        } else {
            // Create new border sprites (4 total for the entire bounding box)
            let top = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(width, border_thickness)),
                        ..default()
                    },
                    Transform::from_xyz(center_x, world_max_y - border_thickness / 2.0, 150.0),
                    TileSelectionHighlight,
                ))
                .id();

            let right = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(border_thickness, height)),
                        ..default()
                    },
                    Transform::from_xyz(world_max_x - border_thickness / 2.0, center_y, 150.0),
                    TileSelectionHighlight,
                ))
                .id();

            let bottom = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(width, border_thickness)),
                        ..default()
                    },
                    Transform::from_xyz(center_x, world_min_y + border_thickness / 2.0, 150.0),
                    TileSelectionHighlight,
                ))
                .id();

            let left = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(border_thickness, height)),
                        ..default()
                    },
                    Transform::from_xyz(world_min_x + border_thickness / 2.0, center_y, 150.0),
                    TileSelectionHighlight,
                ))
                .id();

            selection_state.border_entities = Some([top, right, bottom, left]);
        }

        selection_state.cached_bounds = Some(new_bounds);
        selection_state.current_move_offset = current_offset;
    }

    // Update tracked tile set
    selection_state.highlighted_tiles = current_selection.clone();
}

/// Mark the viewport as needing a rebuild when tiles change
pub fn mark_viewport_dirty(render_state: &mut RenderState) {
    render_state.mark_dirty();
}

/// Marker component for terrain preview highlight sprites
#[derive(Component)]
pub struct TerrainPreviewHighlight;

/// System to render terrain preview highlights (blue overlay showing affected tiles)
/// Uses incremental updates - only spawns/despawns when preview data changes
fn sync_terrain_preview(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    project: Res<Project>,
    tileset_cache: Res<TilesetTextureCache>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut preview_cache: ResMut<TerrainPreviewCache>,
) {
    // Build the new tiles map from editor state
    let new_tiles: HashMap<(i32, i32), u32> = if editor_state.terrain_preview.active {
        editor_state
            .terrain_preview
            .preview_tiles
            .iter()
            .map(|&((x, y), tile_id)| ((x, y), tile_id))
            .collect()
    } else {
        HashMap::new()
    };

    // Check if anything changed
    if new_tiles == preview_cache.current_tiles
        && editor_state.terrain_preview.active == preview_cache.was_active
    {
        return; // No change, skip all work
    }

    // Find tiles to remove (in cache but not in new)
    let to_remove: Vec<(i32, i32)> = preview_cache
        .current_tiles
        .keys()
        .filter(|pos| !new_tiles.contains_key(pos))
        .copied()
        .collect();

    // Find tiles to add (in new but not in cache, or tile_id changed)
    let to_add: Vec<((i32, i32), u32)> = new_tiles
        .iter()
        .filter(|(pos, tile_id)| preview_cache.current_tiles.get(pos) != Some(tile_id))
        .map(|(&pos, &tile_id)| (pos, tile_id))
        .collect();

    // Remove old entities
    for pos in to_remove {
        if let Some(entities) = preview_cache.tile_entities.remove(&pos) {
            for entity in entities {
                let _ = commands.get_entity(entity).map(|mut e| e.despawn());
            }
        }
        preview_cache.current_tiles.remove(&pos);
    }

    // Get tileset info for spawning new entities (O(1) lookup)
    let tileset_info = editor_state
        .terrain_preview
        .tileset_id
        .and_then(|tileset_id| project.get_tileset(tileset_id));

    let Some(tileset) = tileset_info else {
        // No tileset, clear everything remaining
        for (_, entities) in preview_cache.tile_entities.drain() {
            for entity in entities {
                let _ = commands.get_entity(entity).map(|mut e| e.despawn());
            }
        }
        preview_cache.current_tiles.clear();
        preview_cache.was_active = editor_state.terrain_preview.active;
        return;
    };

    let tile_size = tileset.tile_size as f32;
    let preview_tile_color = Color::srgba(1.0, 1.0, 1.0, 0.6);
    let highlight_color = Color::srgba(0.2, 0.5, 1.0, 0.2);
    let border_color = Color::srgba(0.2, 0.5, 1.0, 0.8);
    let border_thickness = 2.0;

    // Add new entities
    for ((x, y), tile_id) in to_add {
        // Remove old entities if updating
        if let Some(entities) = preview_cache.tile_entities.remove(&(x, y)) {
            for entity in entities {
                let _ = commands.get_entity(entity).map(|mut e| e.despawn());
            }
        }

        let world_x = x as f32 * tile_size + tile_size / 2.0;
        let world_y = y as f32 * tile_size + tile_size / 2.0;
        let mut entities = Vec::new();

        // Spawn tile sprite
        if let Some((image_index, local_tile_index)) = tileset.virtual_to_local(tile_id) {
            if let Some(image) = tileset.images.get(image_index) {
                if let Some((texture_handle, _, img_width, img_height)) =
                    tileset_cache.loaded.get(&image.id)
                {
                    let columns = (*img_width as u32) / tileset.tile_size;
                    let rows = (*img_height as u32) / tileset.tile_size;

                    if columns > 0 && rows > 0 {
                        let layout = TextureAtlasLayout::from_grid(
                            UVec2::new(tileset.tile_size, tileset.tile_size),
                            columns,
                            rows,
                            None,
                            None,
                        );
                        let atlas_layout_handle = texture_atlas_layouts.add(layout);

                        let entity = commands
                            .spawn((
                                Sprite {
                                    color: preview_tile_color,
                                    image: texture_handle.clone(),
                                    texture_atlas: Some(TextureAtlas {
                                        layout: atlas_layout_handle,
                                        index: local_tile_index as usize,
                                    }),
                                    ..default()
                                },
                                Transform::from_xyz(world_x, world_y, 179.0),
                                TerrainPreviewHighlight,
                            ))
                            .id();
                        entities.push(entity);
                    }
                }
            }
        }

        // Blue highlight overlay
        let entity = commands
            .spawn((
                Sprite {
                    color: highlight_color,
                    custom_size: Some(Vec2::new(tile_size, tile_size)),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, 180.0),
                TerrainPreviewHighlight,
            ))
            .id();
        entities.push(entity);

        // Top border
        let entity = commands
            .spawn((
                Sprite {
                    color: border_color,
                    custom_size: Some(Vec2::new(tile_size, border_thickness)),
                    ..default()
                },
                Transform::from_xyz(
                    world_x,
                    world_y + tile_size / 2.0 - border_thickness / 2.0,
                    181.0,
                ),
                TerrainPreviewHighlight,
            ))
            .id();
        entities.push(entity);

        // Bottom border
        let entity = commands
            .spawn((
                Sprite {
                    color: border_color,
                    custom_size: Some(Vec2::new(tile_size, border_thickness)),
                    ..default()
                },
                Transform::from_xyz(
                    world_x,
                    world_y - tile_size / 2.0 + border_thickness / 2.0,
                    181.0,
                ),
                TerrainPreviewHighlight,
            ))
            .id();
        entities.push(entity);

        // Left border
        let entity = commands
            .spawn((
                Sprite {
                    color: border_color,
                    custom_size: Some(Vec2::new(border_thickness, tile_size)),
                    ..default()
                },
                Transform::from_xyz(
                    world_x - tile_size / 2.0 + border_thickness / 2.0,
                    world_y,
                    181.0,
                ),
                TerrainPreviewHighlight,
            ))
            .id();
        entities.push(entity);

        // Right border
        let entity = commands
            .spawn((
                Sprite {
                    color: border_color,
                    custom_size: Some(Vec2::new(border_thickness, tile_size)),
                    ..default()
                },
                Transform::from_xyz(
                    world_x + tile_size / 2.0 - border_thickness / 2.0,
                    world_y,
                    181.0,
                ),
                TerrainPreviewHighlight,
            ))
            .id();
        entities.push(entity);

        // Store in cache
        preview_cache.current_tiles.insert((x, y), tile_id);
        preview_cache.tile_entities.insert((x, y), entities);
    }

    preview_cache.was_active = editor_state.terrain_preview.active;
}

/// System to render brush preview when Paint tool is active
fn sync_brush_preview(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    project: Res<Project>,
    tileset_cache: Res<TilesetTextureCache>,
    mut preview_cache: ResMut<BrushPreviewCache>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Helper to clear preview
    fn clear_preview(commands: &mut Commands, cache: &mut BrushPreviewCache) {
        if let Some(entity) = cache.sprite_entity.take() {
            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
        }
        for entity in cache.border_entities.drain(..) {
            let _ = commands.get_entity(entity).map(|mut e| e.despawn());
        }
        cache.last_position = None;
        cache.last_tile = None;
        cache.last_tileset = None;
    }

    // Only show preview when Paint tool is active and not in terrain mode
    let show_preview = editor_state.current_tool == EditorTool::Paint
        && !editor_state.terrain_paint_state.is_terrain_mode
        && editor_state.brush_preview.active
        && editor_state.brush_preview.position.is_some()
        && editor_state.selected_tile.is_some()
        && editor_state.selected_tileset.is_some();

    if !show_preview {
        clear_preview(&mut commands, &mut preview_cache);
        return;
    }

    let position = editor_state.brush_preview.position.unwrap();
    let tile_id = editor_state.selected_tile.unwrap();
    let tileset_id = editor_state.selected_tileset.unwrap();

    // Check if we need to update
    let needs_update = preview_cache.last_position != Some(position)
        || preview_cache.last_tile != Some(tile_id)
        || preview_cache.last_tileset != Some(tileset_id);

    if !needs_update {
        return;
    }

    // Clear old preview
    clear_preview(&mut commands, &mut preview_cache);

    // Get tileset (O(1) lookup)
    let Some(tileset) = project.get_tileset(tileset_id) else {
        return;
    };

    let tile_size = tileset.tile_size as f32;
    let (grid_width, grid_height) = tileset.get_tile_grid_size(tile_id);
    let preview_color = Color::srgba(1.0, 1.0, 1.0, 0.6);
    let border_color = Color::srgba(0.2, 0.8, 0.2, 0.8); // Green for brush

    // Calculate world position using origin (consistent with tile placement)
    let total_width = grid_width as f32 * tile_size;
    let total_height = grid_height as f32 * tile_size;
    let props = tileset
        .get_tile_properties(tile_id)
        .cloned()
        .unwrap_or_default();
    let (origin_x, origin_y) = props.get_origin(total_width as u32, total_height as u32);
    let world_x = position.0 as f32 * tile_size + origin_x as f32;
    let world_y = position.1 as f32 * tile_size + origin_y as f32;

    // Spawn tile sprite (try to use texture, fall back to colored rectangle)
    let mut sprite_created = false;
    if let Some((image_index, local_tile_index)) = tileset.virtual_to_local(tile_id) {
        if let Some(image) = tileset.images.get(image_index) {
            if let Some((texture_handle, _, img_width, img_height)) =
                tileset_cache.loaded.get(&image.id)
            {
                let columns = (*img_width as u32) / tileset.tile_size;
                let rows = (*img_height as u32) / tileset.tile_size;

                if columns > 0 && rows > 0 {
                    if grid_width > 1 || grid_height > 1 {
                        // Multi-cell tile: use Sprite with rect for the full region
                        let tile_col = local_tile_index % image.columns;
                        let tile_row = local_tile_index / image.columns;
                        let src_x = (tile_col * tileset.tile_size) as f32;
                        let src_y = (tile_row * tileset.tile_size) as f32;
                        let src_width = total_width;
                        let src_height = total_height;

                        let rect = bevy::math::Rect::new(
                            src_x,
                            src_y,
                            src_x + src_width,
                            src_y + src_height,
                        );

                        let entity = commands
                            .spawn((
                                Sprite {
                                    color: preview_color,
                                    image: texture_handle.clone(),
                                    rect: Some(rect),
                                    custom_size: Some(Vec2::new(src_width, src_height)),
                                    ..default()
                                },
                                Transform::from_xyz(world_x, world_y, 179.0),
                                BrushPreviewSprite,
                            ))
                            .id();
                        preview_cache.sprite_entity = Some(entity);
                        sprite_created = true;
                    } else {
                        // Single tile: use TextureAtlas
                        let layout = TextureAtlasLayout::from_grid(
                            UVec2::new(tileset.tile_size, tileset.tile_size),
                            columns,
                            rows,
                            None,
                            None,
                        );
                        let atlas_layout_handle = texture_atlas_layouts.add(layout);

                        let entity = commands
                            .spawn((
                                Sprite {
                                    color: preview_color,
                                    image: texture_handle.clone(),
                                    texture_atlas: Some(TextureAtlas {
                                        layout: atlas_layout_handle,
                                        index: local_tile_index as usize,
                                    }),
                                    ..default()
                                },
                                Transform::from_xyz(world_x, world_y, 179.0),
                                BrushPreviewSprite,
                            ))
                            .id();
                        preview_cache.sprite_entity = Some(entity);
                        sprite_created = true;
                    }
                }
            }
        }
    }

    // Fallback: show a semi-transparent highlight if texture not available
    if !sprite_created {
        let highlight_color = Color::srgba(0.2, 0.8, 0.2, 0.3);
        let entity = commands
            .spawn((
                Sprite {
                    color: highlight_color,
                    custom_size: Some(Vec2::new(total_width, total_height)),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, 179.0),
                BrushPreviewSprite,
            ))
            .id();
        preview_cache.sprite_entity = Some(entity);
    }

    // Draw border around preview area
    let border_thickness = 2.0;
    let half_width = total_width / 2.0;
    let half_height = total_height / 2.0;

    // Top border
    let entity = commands
        .spawn((
            Sprite {
                color: border_color,
                custom_size: Some(Vec2::new(total_width, border_thickness)),
                ..default()
            },
            Transform::from_xyz(
                world_x,
                world_y + half_height - border_thickness / 2.0,
                181.0,
            ),
            BrushPreviewSprite,
        ))
        .id();
    preview_cache.border_entities.push(entity);

    // Bottom border
    let entity = commands
        .spawn((
            Sprite {
                color: border_color,
                custom_size: Some(Vec2::new(total_width, border_thickness)),
                ..default()
            },
            Transform::from_xyz(
                world_x,
                world_y - half_height + border_thickness / 2.0,
                181.0,
            ),
            BrushPreviewSprite,
        ))
        .id();
    preview_cache.border_entities.push(entity);

    // Left border
    let entity = commands
        .spawn((
            Sprite {
                color: border_color,
                custom_size: Some(Vec2::new(border_thickness, total_height)),
                ..default()
            },
            Transform::from_xyz(
                world_x - half_width + border_thickness / 2.0,
                world_y,
                181.0,
            ),
            BrushPreviewSprite,
        ))
        .id();
    preview_cache.border_entities.push(entity);

    // Right border
    let entity = commands
        .spawn((
            Sprite {
                color: border_color,
                custom_size: Some(Vec2::new(border_thickness, total_height)),
                ..default()
            },
            Transform::from_xyz(
                world_x + half_width - border_thickness / 2.0,
                world_y,
                181.0,
            ),
            BrushPreviewSprite,
        ))
        .id();
    preview_cache.border_entities.push(entity);

    // Update cache
    preview_cache.last_position = Some(position);
    preview_cache.last_tile = Some(tile_id);
    preview_cache.last_tileset = Some(tileset_id);
}

/// Marker component for entity sprites in the editor
#[derive(Component)]
pub struct EditorEntitySprite {
    pub level_id: Uuid,
    pub entity_id: Uuid,
}

/// Tracks entity render state
#[derive(Resource, Default)]
pub struct EntityRenderState {
    /// Spawned entity sprites: (level_id, entity_id) -> sprite entity
    pub entity_sprites: HashMap<(Uuid, Uuid), Entity>,
    /// Selection highlight entity (if any)
    pub selection_highlight: Option<Entity>,
    /// Last rendered level for change detection (despawn on level change only)
    pub last_level: Option<Uuid>,
    /// Last rendered layer for visibility toggle (no despawn needed)
    pub last_layer: Option<usize>,
}

/// System to render entities on the canvas as colored rectangles
fn sync_entity_rendering(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    project: Res<Project>,
    mut entity_render_state: ResMut<EntityRenderState>,
) {
    let current_level_id = editor_state.selected_level;
    let current_layer_idx = editor_state.selected_layer;

    // Only despawn all entities when LEVEL changes (not layer)
    // Layer changes use visibility toggling for performance
    if entity_render_state.last_level != current_level_id {
        for (_, sprite_entity) in entity_render_state.entity_sprites.drain() {
            let _ = commands.get_entity(sprite_entity).map(|mut e| e.despawn());
        }
        if let Some(highlight_entity) = entity_render_state.selection_highlight.take() {
            let _ = commands
                .get_entity(highlight_entity)
                .map(|mut e| e.despawn());
        }
        entity_render_state.last_level = current_level_id;
    }
    entity_render_state.last_layer = current_layer_idx;

    let Some(level_id) = current_level_id else {
        return;
    };

    // Use O(1) lookup
    let Some(level) = project.get_level(level_id) else {
        return;
    };

    // Get entity IDs for the selected Object layer (only show entities on the current layer)
    let layer_entity_ids: std::collections::HashSet<Uuid> = current_layer_idx
        .and_then(|idx| level.layers.get(idx))
        .and_then(|layer| match &layer.data {
            LayerData::Objects { entities } => Some(entities.iter().copied().collect()),
            _ => None,
        })
        .unwrap_or_default();

    // Build set of ALL entity IDs in the level (for existence check)
    let all_entity_ids: std::collections::HashSet<_> =
        level.entities.iter().map(|e| (level_id, e.id)).collect();

    // Remove sprites for entities that no longer exist in the level
    let to_remove: Vec<_> = entity_render_state
        .entity_sprites
        .keys()
        .filter(|key| !all_entity_ids.contains(key))
        .copied()
        .collect();

    for key in to_remove {
        if let Some(sprite_entity) = entity_render_state.entity_sprites.remove(&key) {
            let _ = commands.get_entity(sprite_entity).map(|mut e| e.despawn());
        }
    }

    // Spawn or update sprites for ALL entities in the level
    // Toggle visibility based on whether entity is in current layer
    for entity in &level.entities {
        let key = (level_id, entity.id);
        let x = entity.position[0];
        let y = entity.position[1];

        // Determine visibility: only visible if entity is in current layer
        let is_visible = layer_entity_ids.contains(&entity.id);
        let visibility = if is_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        // Get color and marker size from schema type definition
        let type_def = project.schema.get_type(&entity.type_name);
        let color = type_def
            .map(|td| parse_hex_color(&td.color))
            .unwrap_or(Color::srgba(0.4, 0.8, 0.4, 0.8)); // Default green
        let entity_size = type_def.and_then(|td| td.marker_size).unwrap_or(16) as f32;

        if let Some(&sprite_entity) = entity_render_state.entity_sprites.get(&key) {
            // Update position, color, and visibility of existing sprite
            if let Ok(mut entity_commands) = commands.get_entity(sprite_entity) {
                entity_commands.insert((
                    Transform::from_xyz(x, y, 50.0),
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(entity_size, entity_size)),
                        ..default()
                    },
                    visibility,
                ));
            }
        } else {
            // Spawn new sprite with visibility
            let sprite_entity = commands
                .spawn((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(entity_size, entity_size)),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 50.0),
                    visibility,
                    EditorEntitySprite {
                        level_id,
                        entity_id: entity.id,
                    },
                ))
                .id();
            entity_render_state
                .entity_sprites
                .insert(key, sprite_entity);
        }
    }

    // Handle selection highlight
    if let Some(highlight_entity) = entity_render_state.selection_highlight.take() {
        let _ = commands
            .get_entity(highlight_entity)
            .map(|mut e| e.despawn());
    }

    // Draw selection highlight around selected entity
    if let Selection::Entity(sel_level_id, sel_entity_id) = &editor_state.selection {
        if *sel_level_id == level_id {
            if let Some(entity) = level.entities.iter().find(|e| e.id == *sel_entity_id) {
                // Get marker size from schema for the selected entity
                let sel_entity_size = project
                    .schema
                    .get_type(&entity.type_name)
                    .and_then(|td| td.marker_size)
                    .unwrap_or(16) as f32;

                let highlight_entity = commands
                    .spawn((
                        Sprite {
                            color: Color::srgba(1.0, 1.0, 0.0, 0.5), // Yellow highlight
                            custom_size: Some(Vec2::new(
                                sel_entity_size + 8.0,
                                sel_entity_size + 8.0,
                            )),
                            ..default()
                        },
                        Transform::from_xyz(entity.position[0], entity.position[1], 49.0),
                    ))
                    .id();
                entity_render_state.selection_highlight = Some(highlight_entity);
            }
        }
    }
}

/// Parse a hex color string like "#FF0000" or "FF0000" into Color
fn parse_hex_color(color_str: &str) -> Color {
    let hex = color_str.trim_start_matches('#');

    if hex.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::srgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.8);
        }
    }

    // Default fallback color (green)
    Color::srgba(0.4, 0.8, 0.4, 0.8)
}
