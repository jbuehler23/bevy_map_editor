//! Editor tools - painting, selection, pan/zoom
//!
//! Handles viewport input for various editing operations.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_map_autotile;
use bevy_map_core::{EntityInstance, LayerData, OCCUPIED_CELL};
use std::collections::HashMap;

use crate::commands::{
    collect_tiles_in_region, BatchTileCommand, CommandHistory, MoveEntityCommand,
};
use crate::project::Project;
use crate::render::RenderState;
use crate::ui::{EditorTool, Selection, ToolMode};
use crate::EditorState;
use std::collections::HashSet;

/// Plugin for editor tools and viewport input
pub struct EditorToolsPlugin;

impl Plugin for EditorToolsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewportInputState>()
            .init_resource::<PaintStrokeTracker>()
            .add_systems(
                Update,
                (
                    handle_viewport_input,
                    handle_zoom_input,
                    finalize_paint_stroke,
                ),
            );
    }
}

/// State for viewport input handling
#[derive(Resource, Default)]
pub struct ViewportInputState {
    /// Last mouse position in world coordinates
    pub last_world_pos: Option<Vec2>,
    /// Whether we're currently panning
    pub is_panning: bool,
    /// Last cursor position for panning
    pub pan_start_pos: Option<Vec2>,
    /// Start tile position for rectangle tool
    pub rect_start_tile: Option<(i32, i32)>,
    /// Whether we're currently drawing a rectangle
    pub is_drawing_rect: bool,
    /// Paint targets painted during the current stroke (for target-based deduplication)
    /// Tracks corners/edges that have been painted to avoid overlapping operations
    pub painted_targets_this_stroke: HashSet<bevy_map_autotile::PaintTarget>,
    /// Last preview target position (for recalculating preview only on change)
    pub last_preview_target: Option<bevy_map_autotile::PaintTarget>,
    /// Last world position when painting (for line interpolation during drag)
    pub last_paint_world_pos: Option<Vec2>,
    /// Whether full-tile mode was active for the last preview (to detect mode changes)
    pub last_preview_full_tile_mode: bool,
    /// Anchor tile position for line brush (Shift+Click draws line from anchor to click)
    pub line_brush_anchor: Option<(i32, i32)>,
}

/// Tracks tile changes during a painting stroke for undo support
#[derive(Resource, Default)]
pub struct PaintStrokeTracker {
    /// Whether we're currently in a paint stroke
    pub active: bool,
    /// The level being painted
    pub level_id: Option<uuid::Uuid>,
    /// The layer being painted
    pub layer_idx: Option<usize>,
    /// Changes made during this stroke: (x, y) -> (old_tile, new_tile)
    pub changes: HashMap<(u32, u32), (Option<u32>, Option<u32>)>,
    /// Description of the operation
    pub description: String,
}

/// Capture a snapshot of tiles in a region around a center point for undo tracking
fn capture_tile_region(
    tiles: &[Option<u32>],
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    radius: i32,
) -> HashMap<(u32, u32), Option<u32>> {
    let mut snapshot = HashMap::new();

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let x = center_x + dx;
            let y = center_y + dy;

            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                let x = x as u32;
                let y = y as u32;
                let idx = (y * width + x) as usize;
                let tile = tiles.get(idx).copied().flatten();
                snapshot.insert((x, y), tile);
            }
        }
    }

    snapshot
}

/// Capture a snapshot of tiles in a rectangular bounds for undo tracking
fn capture_tile_region_bounds(
    tiles: &[Option<u32>],
    width: u32,
    height: u32,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> HashMap<(u32, u32), Option<u32>> {
    let mut snapshot = HashMap::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                let x = x as u32;
                let y = y as u32;
                let idx = (y * width + x) as usize;
                let tile = tiles.get(idx).copied().flatten();
                snapshot.insert((x, y), tile);
            }
        }
    }

    snapshot
}

/// Calculate the bounding box of all paint targets plus a buffer
fn calculate_targets_bounds(
    targets: &[bevy_map_autotile::PaintTarget],
    buffer: i32,
) -> (i32, i32, i32, i32) {
    if targets.is_empty() {
        return (0, 0, 0, 0);
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for target in targets {
        let (cx, cy) = match target {
            bevy_map_autotile::PaintTarget::Corner { corner_x, corner_y } => {
                (*corner_x as i32, *corner_y as i32)
            }
            bevy_map_autotile::PaintTarget::HorizontalEdge { tile_x, edge_y } => {
                (*tile_x as i32, *edge_y as i32)
            }
            bevy_map_autotile::PaintTarget::VerticalEdge { edge_x, tile_y } => {
                (*edge_x as i32, *tile_y as i32)
            }
        };

        min_x = min_x.min(cx - buffer);
        min_y = min_y.min(cy - buffer);
        max_x = max_x.max(cx + buffer);
        max_y = max_y.max(cy + buffer);
    }

    (min_x, min_y, max_x, max_y)
}

/// Bresenham's line algorithm - generates all tile coordinates along a line
fn bresenham_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        points.push((x, y));

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }

    points
}

/// System to handle viewport input (painting, panning)
fn handle_viewport_input(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut project: ResMut<Project>,
    mut render_state: ResMut<RenderState>,
    mut input_state: ResMut<ViewportInputState>,
    mut stroke_tracker: ResMut<PaintStrokeTracker>,
    mut history: ResMut<CommandHistory>,
    tileset_cache: Res<crate::ui::TilesetTextureCache>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some((camera, camera_transform)) = camera_q.iter().next() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        input_state.is_panning = false;
        editor_state.is_painting = false;
        editor_state.brush_preview.active = false;
        return;
    };

    // Convert cursor position to world coordinates
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    // Always update world position for preview rendering
    input_state.last_world_pos = Some(world_pos);

    // Handle pending cancel move (from Escape key)
    if editor_state.pending_cancel_move {
        cancel_move_operation(&mut editor_state, &mut project);
        editor_state.pending_cancel_move = false;
    }

    // Check if egui is actively using the pointer (dragging, clicking on widgets)
    // Note: wants_pointer_input() is too broad - it returns true for entire CentralPanel
    // even when there are no interactive widgets, blocking viewport input.
    let egui_wants_pointer = ctx.is_using_pointer();

    // If egui wants the pointer and we're not in the middle of a rectangle draw,
    // block input. But always allow rectangle operations to complete.
    if egui_wants_pointer && !input_state.is_drawing_rect {
        input_state.is_panning = false;
        editor_state.is_painting = false;
        editor_state.brush_preview.active = false;
        return;
    }

    // Handle panning (middle mouse or right mouse)
    if mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right) {
        if !input_state.is_panning {
            input_state.is_panning = true;
            input_state.pan_start_pos = Some(cursor_position);
        } else if let Some(start_pos) = input_state.pan_start_pos {
            let delta = cursor_position - start_pos;
            editor_state.camera_offset.x -= delta.x / editor_state.zoom;
            editor_state.camera_offset.y += delta.y / editor_state.zoom;
            input_state.pan_start_pos = Some(cursor_position);
        }
    } else {
        input_state.is_panning = false;
        input_state.pan_start_pos = None;
    }

    // Get tile size for coordinate conversion
    let tile_size = get_tile_size(&editor_state, &project);

    // Check if pointer is over the right-side inspector panel
    // Estimate: if cursor is in the right ~250px of the screen, it's likely over the inspector
    let pointer_over_right_panel = {
        if let Some(cursor_pos) = window.cursor_position() {
            let window_width = window.resolution.width();
            // Right panel is roughly the rightmost 250px (inspector_width default is 250)
            cursor_pos.x > (window_width - 250.0)
        } else {
            false
        }
    };

    // Check if any modal editors are open (block input to map when they are)
    // TODO: In the future, only block when editors are maximized, not minimized
    let modal_editor_open = editor_state.show_tileset_editor
        || editor_state.show_spritesheet_editor
        || editor_state.show_animation_editor
        || editor_state.show_dialogue_editor;

    // Determine if we're in rectangle mode for this tool
    let is_rectangle_mode =
        editor_state.tool_mode == ToolMode::Rectangle && editor_state.current_tool.supports_modes();

    // Handle painting/erasing/entity placement/selection with left mouse
    // Block entity/fill/select if pointer is over right panel or if modal editors are open
    if mouse_buttons.just_pressed(MouseButton::Left)
        && !input_state.is_panning
        && !pointer_over_right_panel
        && !modal_editor_open
    {
        match editor_state.current_tool {
            EditorTool::Entity => {
                place_entity(&mut editor_state, &mut project, world_pos);
            }
            EditorTool::Fill => {
                fill_area(
                    &mut editor_state,
                    &mut project,
                    &mut render_state,
                    world_pos,
                );
            }
            // Select tool - check for move operations first, then entity click, then marquee selection
            EditorTool::Select => {
                // FIRST: Check if clicking on already-selected entity → start entity move
                if is_click_on_selected_entity(world_pos, &editor_state, &project) {
                    if let Selection::Entity(level_id, entity_id) = &editor_state.selection {
                        // Get the entity's current position for undo
                        if let Some(level) = project.levels.iter().find(|l| l.id == *level_id) {
                            if let Some(entity) = level.entities.iter().find(|e| e.id == *entity_id)
                            {
                                editor_state.is_moving = true;
                                editor_state.move_drag_start = Some(world_pos);
                                editor_state.entity_original_position = Some(entity.position);
                            }
                        }
                    }
                    return;
                }

                // SECOND: Check if clicking on tile selection → start tile move
                if is_click_on_tile_selection(world_pos, &editor_state, tile_size) {
                    editor_state.is_moving = true;
                    editor_state.move_drag_start = Some(world_pos);
                    editor_state.tile_move_offset = Some((0, 0));
                    capture_tile_selection_for_move(&mut editor_state, &project);
                    return;
                }

                // THIRD: Check if clicking on an entity (only on selected Object layer) to select it
                if let Some(level_id) = editor_state.selected_level {
                    if let Some(entity_id) = find_entity_at_position(
                        world_pos,
                        &project,
                        level_id,
                        editor_state.selected_layer,
                    ) {
                        // Select the entity
                        editor_state.selection = Selection::Entity(level_id, entity_id);
                        // Clear any tile selection
                        editor_state.tile_selection.clear();
                        // Don't start marquee selection
                        return;
                    }
                }

                // FOURTH: No entity hit - start marquee selection for tiles
                // Clear entity selection when starting tile selection
                editor_state.selection = Selection::None;
                let tile_x = (world_pos.x / tile_size).floor() as i32;
                let tile_y = (world_pos.y / tile_size).floor() as i32;
                input_state.rect_start_tile = Some((tile_x, tile_y));
                input_state.is_drawing_rect = true;
                editor_state.tile_selection.is_selecting = true;
                editor_state.tile_selection.drag_start = Some((tile_x, tile_y));
            }
            // For tools that support modes, start rectangle drawing if in Rectangle mode
            EditorTool::Paint | EditorTool::Erase | EditorTool::Terrain if is_rectangle_mode => {
                let tile_x = (world_pos.x / tile_size).floor() as i32;
                let tile_y = (world_pos.y / tile_size).floor() as i32;
                input_state.rect_start_tile = Some((tile_x, tile_y));
                input_state.is_drawing_rect = true;
            }
            _ => {}
        }
    }

    // Handle rectangle mode release
    if mouse_buttons.just_released(MouseButton::Left) && input_state.is_drawing_rect {
        if let Some((start_x, start_y)) = input_state.rect_start_tile {
            let end_x = (world_pos.x / tile_size).floor() as i32;
            let end_y = (world_pos.y / tile_size).floor() as i32;

            // Fill based on the current tool
            match editor_state.current_tool {
                EditorTool::Terrain => {
                    fill_terrain_rectangle(
                        &mut editor_state,
                        &mut project,
                        &mut render_state,
                        &mut history,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                    );
                }
                EditorTool::Paint | EditorTool::Erase => {
                    fill_rectangle(
                        &mut editor_state,
                        &mut project,
                        &mut render_state,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                    );
                }
                EditorTool::Select => {
                    // Finalize marquee selection
                    if let (Some(level_id), Some(layer_idx)) =
                        (editor_state.selected_level, editor_state.selected_layer)
                    {
                        let additive = false;

                        // Normalize rectangle bounds
                        let min_x = start_x.min(end_x).max(0) as u32;
                        let max_x = start_x.max(end_x).max(0) as u32;
                        let min_y = start_y.min(end_y).max(0) as u32;
                        let max_y = start_y.max(end_y).max(0) as u32;

                        editor_state.tile_selection.select_rectangle(
                            level_id, layer_idx, min_x, min_y, max_x, max_y, additive,
                        );
                    }
                    editor_state.tile_selection.is_selecting = false;
                    editor_state.tile_selection.drag_start = None;
                }
                _ => {}
            }
        }
        input_state.rect_start_tile = None;
        input_state.is_drawing_rect = false;
    }

    // Handle move operation release (finalize move)
    if mouse_buttons.just_released(MouseButton::Left) && editor_state.is_moving {
        // Finalize entity move
        if editor_state.entity_original_position.is_some() {
            finalize_entity_move(&mut editor_state, &mut project, &mut history);
        }
        // Finalize tile move
        else if editor_state.tile_move_original.is_some() {
            finalize_tile_move(
                &mut editor_state,
                &mut project,
                &mut render_state,
                &mut history,
            );
        }

        // Reset all move state
        editor_state.is_moving = false;
        editor_state.move_drag_start = None;
        editor_state.entity_original_position = None;
        editor_state.tile_move_original = None;
        editor_state.tile_move_offset = None;
    }

    // Handle move operation drag (update positions during move)
    if mouse_buttons.pressed(MouseButton::Left) && editor_state.is_moving && !input_state.is_panning
    {
        if let Some(start_pos) = editor_state.move_drag_start {
            let delta = world_pos - start_pos;

            // Entity move - update position live
            if editor_state.entity_original_position.is_some() {
                if let Selection::Entity(level_id, entity_id) = &editor_state.selection {
                    let level_id = *level_id;
                    let entity_id = *entity_id;

                    if let Some(original_pos) = editor_state.entity_original_position {
                        let mut new_pos = [original_pos[0] + delta.x, original_pos[1] + delta.y];

                        // Apply snap-to-grid if enabled
                        if editor_state.snap_to_grid {
                            let snap_unit = tile_size / 2.0;
                            new_pos[0] = (new_pos[0] / snap_unit).round() * snap_unit;
                            new_pos[1] = (new_pos[1] / snap_unit).round() * snap_unit;
                        }

                        // Bounds check
                        if let Some(level) = project.get_level_mut(level_id) {
                            let level_width_px = level.width as f32 * tile_size;
                            let level_height_px = level.height as f32 * tile_size;

                            // Clamp to level bounds
                            new_pos[0] = new_pos[0].clamp(0.0, level_width_px);
                            new_pos[1] = new_pos[1].clamp(0.0, level_height_px);

                            // Update entity position
                            if let Some(entity) =
                                level.entities.iter_mut().find(|e| e.id == entity_id)
                            {
                                entity.position = new_pos;
                            }
                        }
                    }
                }
            }
            // Tile move - update offset (tiles aren't moved until release)
            else if editor_state.tile_move_original.is_some() {
                // Calculate tile offset from delta
                let offset_x = (delta.x / tile_size).round() as i32;
                let offset_y = (delta.y / tile_size).round() as i32;
                editor_state.tile_move_offset = Some((offset_x, offset_y));
            }
        }
    }

    // Calculate terrain preview only when target position changes (like Tiled's tilePositionChanged)
    // This prevents recalculating the expensive preview every frame
    if !input_state.is_drawing_rect
        && editor_state.terrain_paint_state.is_terrain_mode
        && editor_state.current_tool == EditorTool::Terrain
    {
        // Detect full-tile mode (Ctrl key) for preview
        let full_tile_mode =
            keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

        // Only recalculate if we have terrain set info to compute the paint target
        if let (Some(terrain_set_id), Some(_)) = (
            editor_state.selected_terrain_set,
            editor_state.selected_terrain_in_set,
        ) {
            if let Some(terrain_set) = project.autotile_config.get_terrain_set(terrain_set_id) {
                let tile_size = project
                    .tilesets
                    .iter()
                    .find(|t| t.id == terrain_set.tileset_id)
                    .map(|t| t.tile_size as f32)
                    .unwrap_or(32.0);

                let paint_target = bevy_map_autotile::get_paint_target(
                    world_pos.x,
                    world_pos.y,
                    tile_size,
                    terrain_set.set_type,
                );

                // Recalculate preview if target changed OR full_tile_mode changed
                let mode_changed = input_state.last_preview_full_tile_mode != full_tile_mode;
                if input_state.last_preview_target != Some(paint_target) || mode_changed {
                    input_state.last_preview_target = Some(paint_target);
                    input_state.last_preview_full_tile_mode = full_tile_mode;
                    calculate_terrain_preview(
                        &mut editor_state,
                        &project,
                        world_pos,
                        tile_size,
                        full_tile_mode,
                    );
                }
            }
        }
    } else if !input_state.is_drawing_rect {
        // Clear preview when not in terrain mode
        editor_state.terrain_preview.active = false;
        editor_state.terrain_preview.preview_tiles.clear();
        input_state.last_preview_target = None;
    } else {
        // Clear preview while drawing rectangle
        editor_state.terrain_preview.active = false;
        input_state.last_preview_target = None;
    }

    // Update brush preview position for Paint tool (non-terrain mode)
    // Preview appears on the tile the mouse is currently over
    if editor_state.current_tool == EditorTool::Paint
        && !editor_state.terrain_paint_state.is_terrain_mode
        && editor_state.selected_tile.is_some()
        && !input_state.is_drawing_rect
        && !pointer_over_right_panel
        && !modal_editor_open
    {
        // Simple floor division to get tile under cursor
        let tile_x = (world_pos.x / tile_size).floor() as i32;
        let tile_y = (world_pos.y / tile_size).floor() as i32;
        editor_state.brush_preview.position = Some((tile_x, tile_y));
        editor_state.brush_preview.active = true;
    } else {
        editor_state.brush_preview.active = false;
        editor_state.brush_preview.position = None;
    }

    // Check for Shift key (used for line brush)
    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Point mode painting (continuous while dragging)
    if mouse_buttons.pressed(MouseButton::Left) && !input_state.is_panning && !is_rectangle_mode {
        match editor_state.current_tool {
            EditorTool::Paint => {
                // Get current tile position for line brush anchor tracking
                let current_tile_x = (world_pos.x / tile_size).floor() as i32;
                let current_tile_y = (world_pos.y / tile_size).floor() as i32;

                // Line brush: Shift+Click draws line from anchor to current position
                if mouse_buttons.just_pressed(MouseButton::Left)
                    && shift_pressed
                    && input_state.line_brush_anchor.is_some()
                {
                    let (anchor_x, anchor_y) = input_state.line_brush_anchor.unwrap();
                    let line_points =
                        bresenham_line(anchor_x, anchor_y, current_tile_x, current_tile_y);

                    // Paint each tile along the line
                    for (lx, ly) in line_points {
                        // Convert tile coords back to world position (center of tile)
                        let world_x = (lx as f32 + 0.5) * tile_size;
                        let world_y = (ly as f32 + 0.5) * tile_size;
                        paint_tile(
                            &mut commands,
                            &mut editor_state,
                            &mut project,
                            &mut render_state,
                            &mut stroke_tracker,
                            &tileset_cache,
                            Vec2::new(world_x, world_y),
                        );
                        // Clear last_painted_tile to allow painting each point
                        editor_state.last_painted_tile = None;
                    }

                    // Update anchor to new position
                    input_state.line_brush_anchor = Some((current_tile_x, current_tile_y));
                } else {
                    // Normal painting
                    paint_tile(
                        &mut commands,
                        &mut editor_state,
                        &mut project,
                        &mut render_state,
                        &mut stroke_tracker,
                        &tileset_cache,
                        world_pos,
                    );

                    // Update line brush anchor on first click of stroke
                    if mouse_buttons.just_pressed(MouseButton::Left) {
                        input_state.line_brush_anchor = Some((current_tile_x, current_tile_y));
                    }
                }
            }
            EditorTool::Terrain => {
                // Ctrl+click enables full-tile mode (paints all 8 positions of tile)
                let full_tile_mode = keyboard.pressed(KeyCode::ControlLeft)
                    || keyboard.pressed(KeyCode::ControlRight);
                paint_terrain_tile(
                    &mut commands,
                    &mut editor_state,
                    &mut project,
                    &mut render_state,
                    &mut input_state,
                    &mut stroke_tracker,
                    &tileset_cache,
                    world_pos,
                    full_tile_mode,
                );
            }
            EditorTool::Erase => {
                // Get current tile position for line brush anchor tracking
                let current_tile_x = (world_pos.x / tile_size).floor() as i32;
                let current_tile_y = (world_pos.y / tile_size).floor() as i32;

                // Line brush for erase: Shift+Click erases line from anchor to current position
                if mouse_buttons.just_pressed(MouseButton::Left)
                    && shift_pressed
                    && input_state.line_brush_anchor.is_some()
                {
                    let (anchor_x, anchor_y) = input_state.line_brush_anchor.unwrap();
                    let line_points =
                        bresenham_line(anchor_x, anchor_y, current_tile_x, current_tile_y);

                    // Erase each tile along the line
                    for (lx, ly) in line_points {
                        // Convert tile coords back to world position (center of tile)
                        let world_x = (lx as f32 + 0.5) * tile_size;
                        let world_y = (ly as f32 + 0.5) * tile_size;
                        erase_tile(
                            &mut commands,
                            &mut editor_state,
                            &mut project,
                            &mut render_state,
                            &mut stroke_tracker,
                            &tileset_cache,
                            Vec2::new(world_x, world_y),
                        );
                        // Clear last_painted_tile to allow erasing each point
                        editor_state.last_painted_tile = None;
                    }

                    // Update anchor to new position
                    input_state.line_brush_anchor = Some((current_tile_x, current_tile_y));
                } else {
                    // Normal erasing
                    erase_tile(
                        &mut commands,
                        &mut editor_state,
                        &mut project,
                        &mut render_state,
                        &mut stroke_tracker,
                        &tileset_cache,
                        world_pos,
                    );

                    // Update line brush anchor on first click of stroke
                    if mouse_buttons.just_pressed(MouseButton::Left) {
                        input_state.line_brush_anchor = Some((current_tile_x, current_tile_y));
                    }
                }
            }
            _ => {}
        }
    } else if !input_state.is_drawing_rect {
        editor_state.is_painting = false;
        editor_state.last_painted_tile = None;
        // Clear stroke data when not painting (stroke ended)
        input_state.painted_targets_this_stroke.clear();
        input_state.last_paint_world_pos = None;
    }
}

/// Get the tile size for the current level/layer/tileset
fn get_tile_size(editor_state: &EditorState, project: &Project) -> f32 {
    let level_id = editor_state.selected_level;
    let layer_idx = editor_state.selected_layer;

    let level = level_id.and_then(|id| project.levels.iter().find(|l| l.id == id));
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

    layer_tileset_id
        .or(editor_state.selected_tileset)
        .and_then(|id| project.tilesets.iter().find(|t| t.id == id))
        .map(|t| t.tile_size as f32)
        .unwrap_or(32.0)
}

/// System to handle zoom input
#[allow(deprecated)] // EventReader is deprecated but still works in Bevy 0.17
fn handle_zoom_input(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut scroll_events: bevy::ecs::event::EventReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let Ok(window) = windows.single() else { return };

    // Only block zoom if egui is actively using the pointer (dragging, etc.)
    // or if we're over a side panel area (which has scroll areas)
    let egui_using_pointer = ctx.is_using_pointer();

    // Check if cursor is over a side panel (left tree view or right inspector)
    // These have ScrollAreas that should receive scroll events
    let over_side_panel = if let Some(cursor_pos) = window.cursor_position() {
        let window_width = window.resolution.width();
        // Left panel is roughly 250px, right panel is roughly 250px
        cursor_pos.x < 250.0 || cursor_pos.x > (window_width - 250.0)
    } else {
        false
    };

    // Check if any modal editors are open (mirrors viewport input handling)
    let modal_editor_open = editor_state.show_schema_editor
        || editor_state.show_tileset_editor
        || editor_state.show_spritesheet_editor
        || editor_state.show_animation_editor
        || editor_state.show_dialogue_editor
        || editor_state.show_settings_dialog;

    for event in scroll_events.read() {
        // Skip zoom if egui is using pointer, cursor is over side panels, or modal editors are open
        if egui_using_pointer || over_side_panel || modal_editor_open {
            continue;
        }
        let zoom_delta = event.y * 0.1;
        editor_state.zoom = (editor_state.zoom * (1.0 + zoom_delta)).clamp(0.25, 4.0);
    }
}

/// Check if a layer has any non-empty tiles
fn layer_has_tiles(layer: &bevy_map_core::Layer) -> bool {
    if let LayerData::Tiles { tiles, .. } = &layer.data {
        tiles.iter().any(|t| t.is_some())
    } else {
        false
    }
}

/// Get the tileset_id for a layer
fn get_layer_tileset_id(layer: &bevy_map_core::Layer) -> Option<uuid::Uuid> {
    if let LayerData::Tiles { tileset_id, .. } = &layer.data {
        Some(*tileset_id)
    } else {
        None
    }
}

/// Check if the selected layer is a Tile layer (returns false for Object layers)
fn is_tile_layer(project: &Project, level_id: uuid::Uuid, layer_idx: usize) -> bool {
    project
        .get_level(level_id)
        .and_then(|level| level.layers.get(layer_idx))
        .map(|layer| matches!(&layer.data, LayerData::Tiles { .. }))
        .unwrap_or(false)
}

/// Find an entity at the given world position
/// Returns the entity ID if found, None otherwise
/// Only checks entities on the selected Object layer
fn find_entity_at_position(
    world_pos: Vec2,
    project: &Project,
    level_id: uuid::Uuid,
    layer_idx: Option<usize>,
) -> Option<uuid::Uuid> {
    let level = project.levels.iter().find(|l| l.id == level_id)?;

    // Get entity IDs on the selected Object layer
    let layer_entity_ids: std::collections::HashSet<uuid::Uuid> = layer_idx
        .and_then(|idx| level.layers.get(idx))
        .and_then(|layer| match &layer.data {
            LayerData::Objects { entities } => Some(entities.iter().copied().collect()),
            _ => None,
        })
        .unwrap_or_default();

    // If no entities on this layer, return early
    if layer_entity_ids.is_empty() {
        return None;
    }

    // Check each entity on this layer (in reverse order so topmost entities are checked first)
    for entity in level.entities.iter().rev() {
        // Only check entities on the selected layer
        if !layer_entity_ids.contains(&entity.id) {
            continue;
        }

        // Get marker size from schema, default to 16
        let marker_size = project
            .schema
            .get_type(&entity.type_name)
            .and_then(|td| td.marker_size)
            .unwrap_or(16) as f32;

        let half_size = marker_size / 2.0;
        let entity_pos = Vec2::new(entity.position[0], entity.position[1]);

        // Check if click is within entity bounds
        let min = entity_pos - Vec2::splat(half_size);
        let max = entity_pos + Vec2::splat(half_size);

        if world_pos.x >= min.x
            && world_pos.x <= max.x
            && world_pos.y >= min.y
            && world_pos.y <= max.y
        {
            return Some(entity.id);
        }
    }

    None
}

/// Check if click is on the currently selected entity
fn is_click_on_selected_entity(
    world_pos: Vec2,
    editor_state: &EditorState,
    project: &Project,
) -> bool {
    if let Selection::Entity(level_id, entity_id) = &editor_state.selection {
        if let Some(level) = project.levels.iter().find(|l| l.id == *level_id) {
            if let Some(entity) = level.entities.iter().find(|e| e.id == *entity_id) {
                // Get marker size from schema, default to 16
                let marker_size = project
                    .schema
                    .get_type(&entity.type_name)
                    .and_then(|td| td.marker_size)
                    .unwrap_or(16) as f32;

                let half_size = marker_size / 2.0;
                let entity_pos = Vec2::new(entity.position[0], entity.position[1]);

                // Check if click is within entity bounds
                let min = entity_pos - Vec2::splat(half_size);
                let max = entity_pos + Vec2::splat(half_size);

                return world_pos.x >= min.x
                    && world_pos.x <= max.x
                    && world_pos.y >= min.y
                    && world_pos.y <= max.y;
            }
        }
    }
    false
}

/// Check if click is within current tile selection
fn is_click_on_tile_selection(world_pos: Vec2, editor_state: &EditorState, tile_size: f32) -> bool {
    if editor_state.tile_selection.tiles.is_empty() {
        return false;
    }

    // Convert world position to tile coordinates
    let tile_x = (world_pos.x / tile_size).floor() as u32;
    let tile_y = (world_pos.y / tile_size).floor() as u32;

    // Check if this tile position is in the selection
    // Selection tiles are stored as (level_id, layer_idx, x, y)
    let level_id = match editor_state.tile_selection.level_id {
        Some(id) => id,
        None => return false,
    };
    let layer_idx = match editor_state.tile_selection.layer_idx {
        Some(idx) => idx,
        None => return false,
    };

    editor_state
        .tile_selection
        .tiles
        .contains(&(level_id, layer_idx, tile_x, tile_y))
}

/// Capture tile data for move operation
fn capture_tile_selection_for_move(editor_state: &mut EditorState, project: &Project) {
    let Some(level_id) = editor_state.tile_selection.level_id else {
        return;
    };
    let Some(layer_idx) = editor_state.tile_selection.layer_idx else {
        return;
    };

    let Some(level) = project.levels.iter().find(|l| l.id == level_id) else {
        return;
    };
    let Some(layer) = level.layers.get(layer_idx) else {
        return;
    };

    let tiles = if let LayerData::Tiles { tiles, .. } = &layer.data {
        tiles
    } else {
        return;
    };

    let mut original_tiles = HashMap::new();

    // Selection tiles are stored as (level_id, layer_idx, x, y)
    for &(_sel_level_id, _sel_layer_idx, x, y) in &editor_state.tile_selection.tiles {
        let idx = (y * level.width + x) as usize;
        let tile = tiles.get(idx).copied().flatten();
        original_tiles.insert((x, y), (layer_idx, tile));
    }

    editor_state.tile_move_original = Some(original_tiles);
}

/// Finalize entity move operation and create undo command
fn finalize_entity_move(
    editor_state: &mut EditorState,
    project: &mut Project,
    history: &mut CommandHistory,
) {
    let Some(original_pos) = editor_state.entity_original_position else {
        return;
    };

    if let Selection::Entity(level_id, entity_id) = &editor_state.selection {
        let level_id = *level_id;
        let entity_id = *entity_id;

        let Some(level) = project.get_level(level_id) else {
            return;
        };
        let Some(entity) = level.entities.iter().find(|e| e.id == entity_id) else {
            return;
        };
        let new_pos = entity.position;

        // Skip if no change
        if original_pos == new_pos {
            return;
        }

        // Push undo command (stores the move so undo reverses it)
        let command = MoveEntityCommand::new(level_id, entity_id, original_pos, new_pos);
        history.push_undo(Box::new(command));
        project.mark_dirty();
    }
}

/// Finalize tile move operation and create undo command
fn finalize_tile_move(
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    history: &mut CommandHistory,
) {
    let Some(original_tiles) = editor_state.tile_move_original.take() else {
        return;
    };
    let Some((offset_x, offset_y)) = editor_state.tile_move_offset else {
        return;
    };

    // Skip if no movement
    if offset_x == 0 && offset_y == 0 {
        return;
    }

    let Some(level_id) = editor_state.tile_selection.level_id else {
        return;
    };
    let Some(layer_idx) = editor_state.tile_selection.layer_idx else {
        return;
    };

    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };
    let level_width = level.width;
    let level_height = level.height;

    // Build change map for undo
    let mut changes: HashMap<(u32, u32), (Option<u32>, Option<u32>)> = HashMap::new();

    // 1. Clear source tiles and track changes
    for ((x, y), (_, _tile)) in &original_tiles {
        let old_tile = level.get_tile(layer_idx, *x, *y);
        level.set_tile(layer_idx, *x, *y, None);
        changes.insert((*x, *y), (old_tile, None));
    }

    // 2. Set destination tiles (with bounds check) and track changes
    let mut new_selection = HashSet::new();
    for ((x, y), (_, tile)) in &original_tiles {
        let dest_x = *x as i32 + offset_x;
        let dest_y = *y as i32 + offset_y;

        if dest_x >= 0 && dest_y >= 0 && dest_x < level_width as i32 && dest_y < level_height as i32
        {
            let dest_x = dest_x as u32;
            let dest_y = dest_y as u32;

            // Track what was at destination before (if not already tracked from source clear)
            if !changes.contains_key(&(dest_x, dest_y)) {
                let old_tile = level.get_tile(layer_idx, dest_x, dest_y);
                changes.insert((dest_x, dest_y), (old_tile, *tile));
            } else {
                // Update the "after" value for this position
                if let Some(change) = changes.get_mut(&(dest_x, dest_y)) {
                    change.1 = *tile;
                }
            }

            level.set_tile(layer_idx, dest_x, dest_y, *tile);
            // Use full 4-tuple format for tile selection: (level_id, layer_idx, x, y)
            new_selection.insert((level_id, layer_idx, dest_x, dest_y));
        }
    }

    // 3. Update tile selection to new positions
    editor_state.tile_selection.tiles = new_selection;

    // 4. Push undo command
    if !changes.is_empty() {
        // Create inverse command for undo
        let mut inverse_changes = HashMap::new();
        for ((x, y), (old_tile, new_tile)) in &changes {
            inverse_changes.insert((*x, *y), (*new_tile, *old_tile));
        }

        let command = BatchTileCommand::new(level_id, layer_idx, inverse_changes, "Move Tiles");
        history.push_undo(Box::new(command));
    }

    render_state.needs_rebuild = true;
    project.mark_dirty();
}

/// Cancel move operation and restore original state
fn cancel_move_operation(editor_state: &mut EditorState, project: &mut Project) {
    // Restore entity position if entity move was in progress
    if let Some(original_pos) = editor_state.entity_original_position {
        if let Selection::Entity(level_id, entity_id) = &editor_state.selection {
            if let Some(level) = project.get_level_mut(*level_id) {
                if let Some(entity) = level.entities.iter_mut().find(|e| e.id == *entity_id) {
                    entity.position = original_pos;
                }
            }
        }
    }

    // Reset all move state
    editor_state.is_moving = false;
    editor_state.move_drag_start = None;
    editor_state.entity_original_position = None;
    editor_state.tile_move_original = None;
    editor_state.tile_move_offset = None;
}

/// Paint a tile at the given world position
fn paint_tile(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    stroke_tracker: &mut PaintStrokeTracker,
    tileset_cache: &crate::ui::TilesetTextureCache,
    world_pos: Vec2,
) {
    // Need a selected level, layer, tile, and tileset
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };
    let Some(selected_tileset) = editor_state.selected_tileset else {
        return;
    };

    // Determine which tile to paint: random from set or selected tile
    let base_tile_index =
        if editor_state.random_paint && !editor_state.random_paint_tiles.is_empty() {
            // Pick random tile from the random paint set
            let idx = fastrand::usize(..editor_state.random_paint_tiles.len());
            editor_state.random_paint_tiles[idx]
        } else {
            // Use the normally selected tile
            let Some(tile) = editor_state.selected_tile else {
                return;
            };
            tile
        };

    // Apply flip flags to the tile index
    let tile_index = bevy_map_core::tile_with_flips(
        base_tile_index,
        editor_state.paint_flip_x,
        editor_state.paint_flip_y,
    );

    // Can only paint tiles on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    // Get tile size and grid size from the selected tileset and collect valid tileset IDs
    // (collect before mutable borrow of level)
    // Use base_tile_index for grid size lookup (flip flags don't affect grid size)
    let tileset_info = project
        .tilesets
        .iter()
        .find(|t| t.id == selected_tileset)
        .map(|t| (t.tile_size as f32, t.get_tile_grid_size(base_tile_index)));
    let tile_size = tileset_info.map(|(ts, _)| ts).unwrap_or(32.0);
    let (grid_width, grid_height) = tileset_info.map(|(_, gs)| gs).unwrap_or((1, 1));
    let is_multi_cell = grid_width > 1 || grid_height > 1;
    let valid_tileset_ids: HashSet<_> = project.tilesets.iter().map(|t| t.id).collect();

    // Convert world position to tile coordinates
    // Simple floor division to get tile under cursor
    let tile_x = (world_pos.x / tile_size).floor() as i32;
    let tile_y = (world_pos.y / tile_size).floor() as i32;

    // Don't repaint the same tile
    if editor_state.last_painted_tile == Some((tile_x as u32, tile_y as u32)) {
        return;
    }

    // Validate coordinates
    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };
    if tile_x < 0 || tile_y < 0 || tile_x >= level.width as i32 || tile_y >= level.height as i32 {
        return;
    }

    let tile_x = tile_x as u32;
    let tile_y = tile_y as u32;

    // Check tileset compatibility
    let (has_tiles, layer_tileset) = level
        .layers
        .get(layer_idx)
        .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
        .unwrap_or((false, None));

    // Check if the layer's tileset actually exists in the project
    let tileset_exists = layer_tileset
        .map(|id| valid_tileset_ids.contains(&id))
        .unwrap_or(false);

    if has_tiles {
        if !tileset_exists {
            // Self-healing: layer has orphaned tileset - clear tiles and reassign
            warn!(
                "Layer has tiles from a deleted tileset. Clearing orphaned data and assigning new tileset."
            );
            if let Some(layer) = level.layers.get_mut(layer_idx) {
                if let LayerData::Tiles {
                    tileset_id,
                    tiles,
                    occupied_cells,
                } = &mut layer.data
                {
                    // Clear all orphaned tiles
                    tiles.iter_mut().for_each(|t| *t = None);
                    // Clear multi-cell tile tracking
                    occupied_cells.clear();
                    // Assign the selected tileset
                    *tileset_id = selected_tileset;
                }
            }
            // Continue with painting (don't return)
        } else if layer_tileset != Some(selected_tileset) {
            // Different valid tileset - block painting
            return;
        }
    } else {
        if let Some(layer) = level.layers.get_mut(layer_idx) {
            if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                *tileset_id = selected_tileset;
            }
        }
    }

    // For multi-cell tiles, check if all cells are within bounds
    if is_multi_cell {
        for dy in 0..grid_height {
            for dx in 0..grid_width {
                let cx = tile_x + dx;
                let cy = tile_y + dy;
                if cx >= level.width || cy >= level.height {
                    // Out of bounds - can't place this multi-cell tile here
                    return;
                }
            }
        }
    }

    // Track changes for undo
    if !stroke_tracker.active {
        stroke_tracker.active = true;
        stroke_tracker.level_id = Some(level_id);
        stroke_tracker.layer_idx = Some(layer_idx);
        stroke_tracker.changes.clear();
        stroke_tracker.description = "Paint Tiles".to_string();
    }

    // Collect tiles to update for rendering (after level borrow is released)
    let mut tiles_to_update: Vec<(u32, u32, Option<u32>)> = Vec::new();

    if is_multi_cell {
        // Multi-cell tile placement
        let base_idx = (tile_y * level.width + tile_x) as usize;
        let level_width = level.width;

        // First, clean up any existing occupied_cells entries for cells we're overwriting
        // This ensures consistent state with the sprite cleanup in render/mod.rs
        for dy in 0..grid_height {
            for dx in 0..grid_width {
                let cx = tile_x + dx;
                let cy = tile_y + dy;
                let cell_idx = (cy * level_width + cx) as usize;

                if let Some(layer) = level.layers.get_mut(layer_idx) {
                    if let LayerData::Tiles { occupied_cells, .. } = &mut layer.data {
                        occupied_cells.remove(&cell_idx);
                    }
                }
            }
        }

        for dy in 0..grid_height {
            for dx in 0..grid_width {
                let cx = tile_x + dx;
                let cy = tile_y + dy;
                let cell_idx = (cy * level_width + cx) as usize;
                let old_tile = level.get_tile(layer_idx, cx, cy);

                let new_tile = if dx == 0 && dy == 0 {
                    // Base cell - place the actual tile
                    level.set_tile(layer_idx, cx, cy, Some(tile_index));
                    Some(tile_index)
                } else {
                    // Occupied cell - place sentinel value
                    level.set_tile(layer_idx, cx, cy, Some(OCCUPIED_CELL));
                    // Track in occupied_cells map
                    if let Some(layer) = level.layers.get_mut(layer_idx) {
                        if let LayerData::Tiles { occupied_cells, .. } = &mut layer.data {
                            occupied_cells.insert(cell_idx, base_idx);
                        }
                    }
                    Some(OCCUPIED_CELL)
                };

                // Track for undo
                if !stroke_tracker.changes.contains_key(&(cx, cy)) {
                    stroke_tracker
                        .changes
                        .insert((cx, cy), (old_tile, new_tile));
                } else if let Some(change) = stroke_tracker.changes.get_mut(&(cx, cy)) {
                    change.1 = new_tile;
                }

                tiles_to_update.push((cx, cy, new_tile));
            }
        }
    } else {
        // Standard single-cell tile placement
        let old_tile = level.get_tile(layer_idx, tile_x, tile_y);
        level.set_tile(layer_idx, tile_x, tile_y, Some(tile_index));

        if !stroke_tracker.changes.contains_key(&(tile_x, tile_y)) {
            stroke_tracker
                .changes
                .insert((tile_x, tile_y), (old_tile, Some(tile_index)));
        } else if let Some(change) = stroke_tracker.changes.get_mut(&(tile_x, tile_y)) {
            change.1 = Some(tile_index);
        }

        tiles_to_update.push((tile_x, tile_y, Some(tile_index)));
    }

    // Update tile rendering for all changed tiles (level borrow is released by block end)
    for (cx, cy, new_tile) in tiles_to_update {
        crate::render::update_tile(
            commands,
            render_state,
            project,
            tileset_cache,
            level_id,
            layer_idx,
            cx,
            cy,
            new_tile,
        );
    }

    project.mark_dirty();
    editor_state.is_painting = true;
    editor_state.last_painted_tile = Some((tile_x, tile_y));
}

/// Erase a tile at the given world position
fn erase_tile(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    stroke_tracker: &mut PaintStrokeTracker,
    tileset_cache: &crate::ui::TilesetTextureCache,
    world_pos: Vec2,
) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };

    // Can only erase tiles on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    let tile_size = get_tile_size(editor_state, project);

    let tile_x = (world_pos.x / tile_size).floor() as i32;
    let tile_y = (world_pos.y / tile_size).floor() as i32;

    if editor_state.last_painted_tile == Some((tile_x as u32, tile_y as u32)) {
        return;
    }

    // First pass: Get all needed info from level (immutable borrow)
    let erase_info: Option<(u32, u32, u32, u32, u32, u32, bool)> = {
        let Some(level) = project.get_level(level_id) else {
            return;
        };
        if tile_x < 0 || tile_y < 0 || tile_x >= level.width as i32 || tile_y >= level.height as i32
        {
            return;
        }

        let tile_x = tile_x as u32;
        let tile_y = tile_y as u32;
        let cell_idx = (tile_y * level.width + tile_x) as usize;
        let level_width = level.width;

        // Check if this is part of a multi-cell tile
        let base_cell_idx = if let Some(layer) = level.layers.get(layer_idx) {
            if let LayerData::Tiles { occupied_cells, .. } = &layer.data {
                occupied_cells.get(&cell_idx).copied()
            } else {
                None
            }
        } else {
            None
        };

        // Find the base cell (either this cell or the one it points to)
        let actual_base_idx = base_cell_idx.unwrap_or(cell_idx);
        let base_x = (actual_base_idx % level_width as usize) as u32;
        let base_y = (actual_base_idx / level_width as usize) as u32;

        // Get the tile at the base cell and tileset ID to determine grid size
        let base_tile = level.get_tile(layer_idx, base_x, base_y);
        let tileset_id = level.layers.get(layer_idx).and_then(|l| {
            if let LayerData::Tiles { tileset_id, .. } = &l.data {
                Some(*tileset_id)
            } else {
                None
            }
        });

        // Determine grid size
        let (grid_width, grid_height) = if let Some(tile_index) = base_tile {
            if tile_index != OCCUPIED_CELL {
                if let Some(ts_id) = tileset_id {
                    project
                        .tilesets
                        .iter()
                        .find(|t| t.id == ts_id)
                        .map(|t| t.get_tile_grid_size(tile_index))
                        .unwrap_or((1, 1))
                } else {
                    (1, 1)
                }
            } else {
                (1, 1)
            }
        } else {
            (1, 1)
        };

        let is_multi_cell = grid_width > 1 || grid_height > 1;

        Some((
            tile_x,
            tile_y,
            base_x,
            base_y,
            grid_width,
            grid_height,
            is_multi_cell,
        ))
    };

    let Some((tile_x, tile_y, base_x, base_y, grid_width, grid_height, is_multi_cell)) = erase_info
    else {
        return;
    };

    if !stroke_tracker.active {
        stroke_tracker.active = true;
        stroke_tracker.level_id = Some(level_id);
        stroke_tracker.layer_idx = Some(layer_idx);
        stroke_tracker.changes.clear();
        stroke_tracker.description = "Erase Tiles".to_string();
    }

    // Second pass: Apply changes (mutable borrow)
    let mut tiles_to_update: Vec<(u32, u32)> = Vec::new();

    {
        let Some(level) = project.get_level_mut(level_id) else {
            return;
        };
        let level_width = level.width;

        if is_multi_cell {
            // Erase all cells of the multi-cell tile
            for dy in 0..grid_height {
                for dx in 0..grid_width {
                    let cx = base_x + dx;
                    let cy = base_y + dy;
                    let cidx = (cy * level_width + cx) as usize;
                    let old_tile = level.get_tile(layer_idx, cx, cy);

                    level.set_tile(layer_idx, cx, cy, None);

                    // Remove from occupied_cells map if not base cell
                    if dx != 0 || dy != 0 {
                        if let Some(layer) = level.layers.get_mut(layer_idx) {
                            if let LayerData::Tiles { occupied_cells, .. } = &mut layer.data {
                                occupied_cells.remove(&cidx);
                            }
                        }
                    }

                    // Track for undo
                    if !stroke_tracker.changes.contains_key(&(cx, cy)) {
                        stroke_tracker.changes.insert((cx, cy), (old_tile, None));
                    } else if let Some(change) = stroke_tracker.changes.get_mut(&(cx, cy)) {
                        change.1 = None;
                    }

                    tiles_to_update.push((cx, cy));
                }
            }
        } else {
            // Standard single-cell erase
            let old_tile = level.get_tile(layer_idx, tile_x, tile_y);
            level.set_tile(layer_idx, tile_x, tile_y, None);

            if !stroke_tracker.changes.contains_key(&(tile_x, tile_y)) {
                stroke_tracker
                    .changes
                    .insert((tile_x, tile_y), (old_tile, None));
            } else if let Some(change) = stroke_tracker.changes.get_mut(&(tile_x, tile_y)) {
                change.1 = None;
            }

            tiles_to_update.push((tile_x, tile_y));
        }
    }

    // Third pass: Update rendering (level borrow released)
    for (cx, cy) in tiles_to_update {
        crate::render::update_tile(
            commands,
            render_state,
            project,
            tileset_cache,
            level_id,
            layer_idx,
            cx,
            cy,
            None,
        );
    }

    project.mark_dirty();
    editor_state.is_painting = true;
    editor_state.last_painted_tile = Some((tile_x, tile_y));
}

/// Place an entity at the given world position
fn place_entity(editor_state: &mut EditorState, project: &mut Project, world_pos: Vec2) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };

    // REQUIRE an entity type to be selected - don't place if none selected
    let Some(type_name) = editor_state.selected_entity_type.clone() else {
        return;
    };

    // REQUIRE selected layer to be an Object layer
    {
        let Some(level) = project.get_level(level_id) else {
            return;
        };
        let Some(layer) = level.layers.get(layer_idx) else {
            return;
        };
        if !matches!(&layer.data, LayerData::Objects { .. }) {
            return; // Can't place entities on tile layers
        }
    }

    // Get tile size for bounds check and snapping
    let tile_size = get_tile_size(editor_state, project);

    // Apply snap-to-grid if enabled (snap to nearest tile center)
    let final_pos = if editor_state.snap_to_grid {
        // Snap to nearest tile center: round to nearest half-tile unit
        // This snaps to positions 0.5*size, 1.5*size, 2.5*size, etc.
        let snap_unit = tile_size / 2.0;
        let snapped_x = (world_pos.x / snap_unit).round() * snap_unit;
        let snapped_y = (world_pos.y / snap_unit).round() * snap_unit;
        Vec2::new(snapped_x, snapped_y)
    } else {
        world_pos
    };

    // Bounds check - entity must be within level grid (check AFTER snapping)
    {
        let Some(level) = project.get_level(level_id) else {
            return;
        };
        let level_width_px = level.width as f32 * tile_size;
        let level_height_px = level.height as f32 * tile_size;

        if final_pos.x < 0.0
            || final_pos.y < 0.0
            || final_pos.x >= level_width_px
            || final_pos.y >= level_height_px
        {
            return; // Can't place outside level bounds
        }
    }

    let position = [final_pos.x, final_pos.y];

    let mut entity = EntityInstance::new(type_name.clone(), position);

    // Initialize properties from schema defaults if the type exists
    if let Some(type_def) = project.schema.get_type(&type_name) {
        for prop in &type_def.properties {
            if let Some(default_val) = &prop.default {
                entity.properties.insert(
                    prop.name.clone(),
                    bevy_map_core::Value::from_json(default_val.clone()),
                );
            }
        }
    }

    let entity_id = entity.id;

    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };
    level.add_entity(entity);

    // Add entity UUID to the Object layer's entities list
    if let Some(layer) = level.layers.get_mut(layer_idx) {
        if let LayerData::Objects { entities } = &mut layer.data {
            entities.push(entity_id);
        }
    }

    project.mark_dirty();

    editor_state.selection = Selection::Entity(level_id, entity_id);
}

/// Fill a rectangular area with the selected tile (or erase if no tile selected)
fn fill_rectangle(
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };

    // Can only fill tiles on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    let tile_index = editor_state.selected_tile;
    let selected_tileset = editor_state.selected_tileset;

    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };
    let level_width = level.width as i32;
    let level_height = level.height as i32;

    let min_x = start_x.min(end_x).max(0);
    let max_x = start_x.max(end_x).min(level_width - 1);
    let min_y = start_y.min(end_y).max(0);
    let max_y = start_y.max(end_y).min(level_height - 1);

    if let (Some(tile_idx), Some(sel_tileset)) = (tile_index, selected_tileset) {
        let (has_tiles, layer_tileset) = level
            .layers
            .get(layer_idx)
            .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
            .unwrap_or((false, None));

        if has_tiles {
            if layer_tileset != Some(sel_tileset) {
                return;
            }
        } else {
            if let Some(layer) = level.layers.get_mut(layer_idx) {
                if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                    *tileset_id = sel_tileset;
                }
            }
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                level.set_tile(layer_idx, x as u32, y as u32, Some(tile_idx));
            }
        }
    } else {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                level.set_tile(layer_idx, x as u32, y as u32, None);
            }
        }
    }

    project.mark_dirty();
    render_state.needs_rebuild = true;
}

/// Flood fill an area with the selected tile (bucket fill)
fn fill_area(
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    world_pos: Vec2,
) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };
    let Some(tile_index) = editor_state.selected_tile else {
        return;
    };
    let Some(selected_tileset) = editor_state.selected_tileset else {
        return;
    };

    // Can only fill tiles on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    let tile_size = get_tile_size(editor_state, project);

    let start_x = (world_pos.x / tile_size).floor() as i32;
    let start_y = (world_pos.y / tile_size).floor() as i32;

    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };

    if start_x < 0 || start_y < 0 || start_x >= level.width as i32 || start_y >= level.height as i32
    {
        return;
    }

    let target_tile = level.get_tile(layer_idx, start_x as u32, start_y as u32);

    if target_tile == Some(tile_index) {
        return;
    }

    let (has_tiles, layer_tileset) = level
        .layers
        .get(layer_idx)
        .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
        .unwrap_or((false, None));

    if has_tiles {
        if layer_tileset != Some(selected_tileset) {
            return;
        }
    } else {
        if let Some(layer) = level.layers.get_mut(layer_idx) {
            if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                *tileset_id = selected_tileset;
            }
        }
    }

    let level_width = level.width;
    let level_height = level.height;

    let mut stack = vec![(start_x as u32, start_y as u32)];
    let mut visited = std::collections::HashSet::new();

    while let Some((x, y)) = stack.pop() {
        if visited.contains(&(x, y)) {
            continue;
        }
        visited.insert((x, y));

        if level.get_tile(layer_idx, x, y) != target_tile {
            continue;
        }

        level.set_tile(layer_idx, x, y, Some(tile_index));

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x < level_width - 1 {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y < level_height - 1 {
            stack.push((x, y + 1));
        }
    }

    project.mark_dirty();
    render_state.needs_rebuild = true;
}

/// Paint a terrain tile with autotiling at the given world position
/// If full_tile_mode is true (Ctrl held), paints all 8 positions of the tile
fn paint_terrain_tile(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    input_state: &mut ViewportInputState,
    stroke_tracker: &mut PaintStrokeTracker,
    tileset_cache: &crate::ui::TilesetTextureCache,
    world_pos: Vec2,
    full_tile_mode: bool,
) {
    // Note: Preview is calculated separately in handle_viewport_input
    // to continue showing during drag operations

    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };

    // Can only paint terrain on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    // Check for Tiled-style terrain sets
    if let Some(terrain_set_id) = editor_state.selected_terrain_set {
        paint_terrain_set_tile(
            commands,
            editor_state,
            project,
            render_state,
            input_state,
            stroke_tracker,
            tileset_cache,
            world_pos,
            level_id,
            layer_idx,
            terrain_set_id,
            full_tile_mode,
        );
    }
    // Finally check for legacy 47-tile terrains (no full-tile mode for legacy)
    else if let Some(terrain_id) = editor_state.selected_terrain {
        paint_legacy_terrain_tile(
            commands,
            editor_state,
            project,
            render_state,
            stroke_tracker,
            tileset_cache,
            world_pos,
            level_id,
            layer_idx,
            terrain_id,
        );
    }
}

/// Get paint targets along a line between two world positions (for continuous drag painting)
/// Like Tiled's pointsOnLine(), this ensures no gaps when dragging quickly
fn get_paint_targets_along_line(
    start: Vec2,
    end: Vec2,
    tile_size: f32,
    set_type: bevy_map_autotile::TerrainSetType,
) -> Vec<bevy_map_autotile::PaintTarget> {
    let mut targets = Vec::new();
    let dist = start.distance(end);

    // Use sub-tile precision to catch all targets
    let steps = (dist / (tile_size * 0.4)).ceil() as i32;
    let steps = steps.max(1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let pos = start.lerp(end, t);
        let target = bevy_map_autotile::get_paint_target(pos.x, pos.y, tile_size, set_type);

        // Only add if different from last target
        if targets.last() != Some(&target) {
            targets.push(target);
        }
    }

    targets
}

/// Paint using the new Tiled-style terrain set system
/// Uses line interpolation for continuous drag painting and target-based deduplication
/// If full_tile_mode is true (Ctrl held), paints all 8 positions of the center tile
fn paint_terrain_set_tile(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    input_state: &mut ViewportInputState,
    stroke_tracker: &mut PaintStrokeTracker,
    tileset_cache: &crate::ui::TilesetTextureCache,
    world_pos: Vec2,
    level_id: uuid::Uuid,
    layer_idx: usize,
    terrain_set_id: uuid::Uuid,
    full_tile_mode: bool,
) {
    let Some(terrain_idx) = editor_state.selected_terrain_in_set else {
        return;
    };

    // Extract terrain set data we need before mutable access to levels
    // (set_type for paint target calculation, tileset_id for validation)
    let (set_type, selected_tileset) = {
        let Some(ts) = project.autotile_config.get_terrain_set(terrain_set_id) else {
            return;
        };
        (ts.set_type, ts.tileset_id)
    };

    let tile_size = project
        .tilesets
        .iter()
        .find(|t| t.id == selected_tileset)
        .map(|t| t.tile_size as f32)
        .unwrap_or(32.0);

    // Get paint targets based on mode
    let paint_targets = if full_tile_mode {
        // Full-tile mode: paint all 8 positions (4 corners + 4 edges) of the center tile
        // This fills the tile completely and updates all 8 surrounding neighbors
        let tile_x = (world_pos.x / tile_size).floor() as u32;
        let tile_y = (world_pos.y / tile_size).floor() as u32;

        vec![
            // 4 corners of the tile
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x,
                corner_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x + 1,
                corner_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x,
                corner_y: tile_y + 1,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x + 1,
                corner_y: tile_y + 1,
            },
            // 4 edges of the tile
            bevy_map_autotile::PaintTarget::HorizontalEdge {
                tile_x,
                edge_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::HorizontalEdge {
                tile_x,
                edge_y: tile_y + 1,
            },
            bevy_map_autotile::PaintTarget::VerticalEdge {
                edge_x: tile_x,
                tile_y,
            },
            bevy_map_autotile::PaintTarget::VerticalEdge {
                edge_x: tile_x + 1,
                tile_y,
            },
        ]
    } else if let Some(last_pos) = input_state.last_paint_world_pos {
        // Normal mode with drag: interpolate along line
        get_paint_targets_along_line(last_pos, world_pos, tile_size, set_type)
    } else {
        // Normal mode, first paint: just paint at current position
        vec![bevy_map_autotile::get_paint_target(
            world_pos.x,
            world_pos.y,
            tile_size,
            set_type,
        )]
    };

    // Filter out targets already painted this stroke (target-based deduplication)
    let new_targets: Vec<_> = paint_targets
        .into_iter()
        .filter(|target| !input_state.painted_targets_this_stroke.contains(target))
        .collect();

    if new_targets.is_empty() {
        // Update last position even if nothing to paint
        input_state.last_paint_world_pos = Some(world_pos);
        return;
    }

    // Use split borrowing: access levels and autotile_config as separate fields
    // This avoids cloning the entire TerrainSet
    let Some(level) = project.levels.iter_mut().find(|l| l.id == level_id) else {
        return;
    };
    let level_width = level.width;
    let level_height = level.height;

    let (has_tiles, layer_tileset) = level
        .layers
        .get(layer_idx)
        .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
        .unwrap_or((false, None));

    if has_tiles {
        if layer_tileset != Some(selected_tileset) {
            input_state.last_paint_world_pos = Some(world_pos);
            return;
        }
    } else {
        if let Some(layer) = level.layers.get_mut(layer_idx) {
            if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                *tileset_id = selected_tileset;
            }
        }
    }

    let tiles = if let Some(layer) = level.layers.get_mut(layer_idx) {
        if let LayerData::Tiles { tiles, .. } = &mut layer.data {
            tiles
        } else {
            input_state.last_paint_world_pos = Some(world_pos);
            return;
        }
    } else {
        input_state.last_paint_world_pos = Some(world_pos);
        return;
    };

    // Initialize stroke tracker
    if !stroke_tracker.active {
        stroke_tracker.active = true;
        stroke_tracker.level_id = Some(level_id);
        stroke_tracker.layer_idx = Some(layer_idx);
        stroke_tracker.changes.clear();
        stroke_tracker.description = "Paint Terrain".to_string();
    }

    // Get terrain set reference for painting (split borrow allows this)
    let Some(terrain_set) = project.autotile_config.get_terrain_set(terrain_set_id) else {
        return;
    };

    // Calculate unified bounding box for all targets (with buffer for corrections)
    let (min_x, min_y, max_x, max_y) = calculate_targets_bounds(&new_targets, 2);

    // Take a single unified snapshot covering all targets (like Tiled)
    let unified_snapshot =
        capture_tile_region_bounds(tiles, level_width, level_height, min_x, min_y, max_x, max_y);

    // Paint all targets in ONE batched operation (like Tiled's approach)
    // This uses a single WangFiller instead of creating 8 separate ones
    bevy_map_autotile::paint_terrain_at_targets(
        tiles,
        level_width,
        level_height,
        &new_targets,
        terrain_set,
        terrain_idx,
    );

    // Track all changes at once from unified snapshot
    // Collect changed tiles for incremental rendering update
    let mut changed_tiles = Vec::new();
    for ((x, y), old_tile) in unified_snapshot {
        let idx = (y * level_width + x) as usize;
        let new_tile = tiles.get(idx).copied().flatten();
        if old_tile != new_tile {
            changed_tiles.push((x, y, new_tile));
            if !stroke_tracker.changes.contains_key(&(x, y)) {
                stroke_tracker.changes.insert((x, y), (old_tile, new_tile));
            } else if let Some(change) = stroke_tracker.changes.get_mut(&(x, y)) {
                change.1 = new_tile;
            }
        }
    }

    // Mark all targets as painted for deduplication
    for paint_target in &new_targets {
        input_state
            .painted_targets_this_stroke
            .insert(*paint_target);
    }

    // Update tile rendering incrementally (like Tiled's approach)
    // This updates only changed tiles instead of triggering a full rebuild
    for (x, y, new_tile) in changed_tiles {
        crate::render::update_tile(
            commands,
            render_state,
            project,
            tileset_cache,
            level_id,
            layer_idx,
            x,
            y,
            new_tile,
        );
    }

    project.mark_dirty();
    editor_state.is_painting = true;

    // Update last paint position for next frame's line interpolation
    input_state.last_paint_world_pos = Some(world_pos);
}

/// Paint using the legacy 47-tile blob terrain system
fn paint_legacy_terrain_tile(
    commands: &mut Commands,
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    stroke_tracker: &mut PaintStrokeTracker,
    tileset_cache: &crate::ui::TilesetTextureCache,
    world_pos: Vec2,
    level_id: uuid::Uuid,
    layer_idx: usize,
    terrain_id: uuid::Uuid,
) {
    let terrain = match project.autotile_config.get_terrain(terrain_id) {
        Some(t) => t.clone(),
        None => return,
    };

    let selected_tileset = terrain.tileset_id;

    let tile_size = project
        .tilesets
        .iter()
        .find(|t| t.id == selected_tileset)
        .map(|t| t.tile_size as f32)
        .unwrap_or(32.0);

    let tile_x = (world_pos.x / tile_size).floor() as i32;
    let tile_y = (world_pos.y / tile_size).floor() as i32;

    if editor_state.last_painted_tile == Some((tile_x as u32, tile_y as u32)) {
        return;
    }

    let Some(level) = project.get_level_mut(level_id) else {
        return;
    };
    if tile_x < 0 || tile_y < 0 || tile_x >= level.width as i32 || tile_y >= level.height as i32 {
        return;
    }

    let tile_x_u32 = tile_x as u32;
    let tile_y_u32 = tile_y as u32;

    let (has_tiles, layer_tileset) = level
        .layers
        .get(layer_idx)
        .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
        .unwrap_or((false, None));

    if has_tiles {
        if layer_tileset != Some(selected_tileset) {
            return;
        }
    } else {
        if let Some(layer) = level.layers.get_mut(layer_idx) {
            if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                *tileset_id = selected_tileset;
            }
        }
    }

    let level_width = level.width;
    let level_height = level.height;

    if let Some(layer) = level.layers.get_mut(layer_idx) {
        if let LayerData::Tiles { tiles, .. } = &mut layer.data {
            let snapshot_region =
                capture_tile_region(tiles, level_width, level_height, tile_x, tile_y, 1);

            let first_tile = terrain.base_tile.saturating_sub(46);
            let last_tile = terrain.base_tile;
            let is_terrain_tile = |tile: Option<u32>| -> bool {
                match tile {
                    Some(t) => t >= first_tile && t <= last_tile,
                    None => false,
                }
            };

            bevy_map_autotile::paint_autotile(
                tiles,
                level_width,
                level_height,
                tile_x_u32,
                tile_y_u32,
                &terrain,
                is_terrain_tile,
            );

            if !stroke_tracker.active {
                stroke_tracker.active = true;
                stroke_tracker.level_id = Some(level_id);
                stroke_tracker.layer_idx = Some(layer_idx);
                stroke_tracker.changes.clear();
                stroke_tracker.description = "Paint Terrain".to_string();
            }

            // Collect changed tiles for incremental update
            let mut changed_tiles = Vec::new();
            for ((x, y), old_tile) in snapshot_region {
                let idx = (y * level_width + x) as usize;
                let new_tile = tiles.get(idx).copied().flatten();
                if old_tile != new_tile {
                    changed_tiles.push((x, y, new_tile));
                    if !stroke_tracker.changes.contains_key(&(x, y)) {
                        stroke_tracker.changes.insert((x, y), (old_tile, new_tile));
                    } else {
                        if let Some(change) = stroke_tracker.changes.get_mut(&(x, y)) {
                            change.1 = new_tile;
                        }
                    }
                }
            }

            // Update tile rendering incrementally for each changed tile
            for (x, y, new_tile) in changed_tiles {
                crate::render::update_tile(
                    commands,
                    render_state,
                    project,
                    tileset_cache,
                    level_id,
                    layer_idx,
                    x,
                    y,
                    new_tile,
                );
            }
        }
    }

    project.mark_dirty();
    editor_state.is_painting = true;
    editor_state.last_painted_tile = Some((tile_x_u32, tile_y_u32));
}

/// Fill a rectangular area with terrain tiles using the autotile system
fn fill_terrain_rectangle(
    editor_state: &mut EditorState,
    project: &mut Project,
    render_state: &mut RenderState,
    history: &mut CommandHistory,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
) {
    let Some(level_id) = editor_state.selected_level else {
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        return;
    };
    let Some(terrain_set_id) = editor_state.selected_terrain_set else {
        return;
    };
    let Some(terrain_idx) = editor_state.selected_terrain_in_set else {
        return;
    };

    // Can only fill terrain on Tile layers
    if !is_tile_layer(project, level_id, layer_idx) {
        return;
    }

    // Extract tileset_id before mutable access to levels
    let selected_tileset = {
        let Some(ts) = project.autotile_config.get_terrain_set(terrain_set_id) else {
            return;
        };
        ts.tileset_id
    };

    // Use split borrowing - access levels directly
    let Some(level) = project.levels.iter().find(|l| l.id == level_id) else {
        return;
    };
    let level_width = level.width as i32;
    let level_height = level.height as i32;

    let min_x = start_x.min(end_x).max(0);
    let max_x = start_x.max(end_x).min(level_width - 1);
    let min_y = start_y.min(end_y).max(0);
    let max_y = start_y.max(end_y).min(level_height - 1);

    let update_min_x = (min_x - 1).max(0);
    let update_max_x = (max_x + 1).min(level_width - 1);
    let update_min_y = (min_y - 1).max(0);
    let update_max_y = (max_y + 1).min(level_height - 1);

    let before_tiles = collect_tiles_in_region(
        project,
        level_id,
        layer_idx,
        update_min_x,
        update_max_x,
        update_min_y,
        update_max_y,
    );

    let Some(level) = project.levels.iter_mut().find(|l| l.id == level_id) else {
        return;
    };

    let (has_tiles, layer_tileset) = level
        .layers
        .get(layer_idx)
        .map(|layer| (layer_has_tiles(layer), get_layer_tileset_id(layer)))
        .unwrap_or((false, None));

    if has_tiles {
        if layer_tileset != Some(selected_tileset) {
            return;
        }
    } else {
        if let Some(layer) = level.layers.get_mut(layer_idx) {
            if let LayerData::Tiles { tileset_id, .. } = &mut layer.data {
                *tileset_id = selected_tileset;
            }
        }
    }

    let level_width = level.width;
    let level_height = level.height;

    let tiles = if let Some(layer) = level.layers.get_mut(layer_idx) {
        if let LayerData::Tiles { tiles, .. } = &mut layer.data {
            tiles
        } else {
            return;
        }
    } else {
        return;
    };

    // Get terrain set reference (split borrow allows this after getting tiles from levels)
    let Some(terrain_set) = project.autotile_config.get_terrain_set(terrain_set_id) else {
        return;
    };

    // Fill with uniform terrain tiles
    let uniform_tiles = terrain_set.find_uniform_tiles(terrain_idx);
    let uniform_tile = uniform_tiles.first().copied();

    if let Some(tile_index) = uniform_tile {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let idx = (y as u32 * level_width + x as u32) as usize;
                if idx < tiles.len() {
                    tiles[idx] = Some(tile_index);
                }
            }
        }
    } else {
        return;
    }

    // Update edge tiles
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let is_at_edge = x == min_x || x == max_x || y == min_y || y == max_y;
            if is_at_edge {
                bevy_map_autotile::update_tile_with_neighbors(
                    tiles,
                    level_width,
                    level_height,
                    x,
                    y,
                    terrain_set,
                    terrain_idx,
                );
            }
        }
    }

    // Update outside neighbor tiles
    let update_min_x = (min_x - 1).max(0);
    let update_max_x = (max_x + 1).min(level_width as i32 - 1);
    let update_min_y = (min_y - 1).max(0);
    let update_max_y = (max_y + 1).min(level_height as i32 - 1);

    for y in update_min_y..=update_max_y {
        for x in update_min_x..=update_max_x {
            let is_inside = x >= min_x && x <= max_x && y >= min_y && y <= max_y;
            if is_inside {
                continue;
            }

            let idx = (y as u32 * level_width + x as u32) as usize;
            let current_tile = tiles.get(idx).copied().flatten();

            if let Some(tile) = current_tile {
                if let Some(tile_data) = terrain_set.get_tile_terrain(tile) {
                    if let Some(primary_terrain) = tile_data.terrains.iter().find_map(|t| *t) {
                        bevy_map_autotile::update_tile_with_neighbors(
                            tiles,
                            level_width,
                            level_height,
                            x,
                            y,
                            terrain_set,
                            primary_terrain,
                        );
                    }
                }
            }
        }
    }

    let after_tiles = collect_tiles_in_region(
        project,
        level_id,
        layer_idx,
        update_min_x,
        update_max_x,
        update_min_y,
        update_max_y,
    );

    let command = BatchTileCommand::from_diff(
        level_id,
        layer_idx,
        before_tiles,
        after_tiles,
        "Fill Terrain Rectangle",
    );

    if !command.changes.is_empty() {
        let mut inverse_changes = std::collections::HashMap::new();
        for ((x, y), (old_tile, new_tile)) in &command.changes {
            inverse_changes.insert((*x, *y), (*new_tile, *old_tile));
        }

        let inverse_command = BatchTileCommand::new(
            level_id,
            layer_idx,
            inverse_changes,
            "Undo Fill Terrain Rectangle",
        );
        history.push_undo(Box::new(inverse_command));
    }

    project.mark_dirty();
    render_state.needs_rebuild = true;
}

/// System to finalize paint strokes and create undo commands
fn finalize_paint_stroke(
    mut stroke_tracker: ResMut<PaintStrokeTracker>,
    mut history: ResMut<CommandHistory>,
    editor_state: Res<EditorState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    if !stroke_tracker.active {
        return;
    }

    if !editor_state.is_painting && !mouse_buttons.pressed(MouseButton::Left) {
        if !stroke_tracker.changes.is_empty() {
            if let (Some(level_id), Some(layer_idx)) =
                (stroke_tracker.level_id, stroke_tracker.layer_idx)
            {
                let mut inverse_changes = HashMap::new();
                for ((x, y), (old_tile, new_tile)) in &stroke_tracker.changes {
                    inverse_changes.insert((*x, *y), (*new_tile, *old_tile));
                }

                let inverse_command = BatchTileCommand::new(
                    level_id,
                    layer_idx,
                    inverse_changes,
                    stroke_tracker.description.clone(),
                );

                history.push_undo(Box::new(inverse_command));
            }
        }

        stroke_tracker.active = false;
        stroke_tracker.level_id = None;
        stroke_tracker.layer_idx = None;
        stroke_tracker.changes.clear();
        stroke_tracker.description.clear();
    }
}

/// Calculate terrain preview tiles for the current mouse position
fn calculate_terrain_preview(
    editor_state: &mut EditorState,
    project: &Project,
    world_pos: Vec2,
    tile_size: f32,
    full_tile_mode: bool,
) {
    let Some(level_id) = editor_state.selected_level else {
        editor_state.terrain_preview.active = false;
        return;
    };
    let Some(layer_idx) = editor_state.selected_layer else {
        editor_state.terrain_preview.active = false;
        return;
    };

    // Check for Tiled-style terrain sets
    let Some(terrain_set_id) = editor_state.selected_terrain_set else {
        editor_state.terrain_preview.active = false;
        return;
    };
    let Some(terrain_idx) = editor_state.selected_terrain_in_set else {
        editor_state.terrain_preview.active = false;
        return;
    };

    let Some(terrain_set) = project.autotile_config.get_terrain_set(terrain_set_id) else {
        editor_state.terrain_preview.active = false;
        return;
    };

    let tileset_id = terrain_set.tileset_id;

    let Some(level) = project.levels.iter().find(|l| l.id == level_id) else {
        editor_state.terrain_preview.active = false;
        return;
    };

    let Some(layer) = level.layers.get(layer_idx) else {
        editor_state.terrain_preview.active = false;
        return;
    };

    let tiles = if let LayerData::Tiles { tiles, .. } = &layer.data {
        tiles
    } else {
        editor_state.terrain_preview.active = false;
        return;
    };

    // Generate paint targets based on mode
    let paint_targets = if full_tile_mode {
        // Full-tile mode: generate all 8 paint targets (4 corners + 4 edges) for the tile
        let tile_x = (world_pos.x / tile_size).floor() as u32;
        let tile_y = (world_pos.y / tile_size).floor() as u32;

        vec![
            // 4 corners of the tile
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x,
                corner_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x + 1,
                corner_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x,
                corner_y: tile_y + 1,
            },
            bevy_map_autotile::PaintTarget::Corner {
                corner_x: tile_x + 1,
                corner_y: tile_y + 1,
            },
            // 4 edges of the tile
            bevy_map_autotile::PaintTarget::HorizontalEdge {
                tile_x,
                edge_y: tile_y,
            },
            bevy_map_autotile::PaintTarget::HorizontalEdge {
                tile_x,
                edge_y: tile_y + 1,
            },
            bevy_map_autotile::PaintTarget::VerticalEdge {
                edge_x: tile_x,
                tile_y,
            },
            bevy_map_autotile::PaintTarget::VerticalEdge {
                edge_x: tile_x + 1,
                tile_y,
            },
        ]
    } else {
        // Normal mode: single paint target based on cursor position
        vec![bevy_map_autotile::get_paint_target(
            world_pos.x,
            world_pos.y,
            tile_size,
            terrain_set.set_type,
        )]
    };

    // Calculate preview using the autotile algorithm
    // For full-tile mode, we need to preview all targets together
    let preview_tiles = bevy_map_autotile::preview_terrain_at_targets(
        tiles,
        level.width,
        level.height,
        &paint_targets,
        terrain_set,
        terrain_idx,
    );

    editor_state.terrain_preview.preview_tiles = preview_tiles;
    editor_state.terrain_preview.tileset_id = Some(tileset_id);
    editor_state.terrain_preview.active = true;
}
