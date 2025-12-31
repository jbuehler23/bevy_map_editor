//! World view panel - displays all levels in an overview
//!
//! This module provides a world view that shows all levels as thumbnails
//! with drag-drop repositioning, layout modes, and connection visualization.

use bevy_egui::egui;
use bevy_map_core::WorldLayout;
use uuid::Uuid;

use crate::project::Project;
use crate::EditorState;

/// Parameters for creating a new level
pub struct NewLevelParams {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub world_x: i32,
    pub world_y: i32,
}

/// Result from world view interaction
#[derive(Default)]
pub struct WorldViewResult {
    /// Level to open (switch to level view)
    pub open_level: Option<Uuid>,
    /// Level to delete
    pub delete_level: Option<Uuid>,
    /// Level to duplicate
    pub duplicate_level: Option<Uuid>,
    /// Create new level with specified parameters
    pub create_level: Option<NewLevelParams>,
    /// Rename level
    pub rename_level: Option<Uuid>,
}

/// Parse a hex color string to egui Color32
fn parse_hex_color(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(r, g, b))
}

/// Generate a default background color for a level based on its index
fn default_level_color(index: usize) -> egui::Color32 {
    let colors = [
        egui::Color32::from_rgb(60, 60, 100),
        egui::Color32::from_rgb(60, 100, 60),
        egui::Color32::from_rgb(100, 60, 60),
        egui::Color32::from_rgb(100, 100, 60),
        egui::Color32::from_rgb(60, 100, 100),
        egui::Color32::from_rgb(100, 60, 100),
        egui::Color32::from_rgb(80, 80, 80),
        egui::Color32::from_rgb(70, 90, 110),
    ];
    colors[index % colors.len()]
}

/// Render the world view panel
pub fn render_world_view(
    ui: &mut egui::Ui,
    editor_state: &mut EditorState,
    project: &mut Project,
) -> WorldViewResult {
    let mut result = WorldViewResult::default();

    // Toolbar at top
    ui.horizontal(|ui| {
        ui.label("Layout:");
        egui::ComboBox::from_id_salt("world_layout_mode")
            .selected_text(project.world_config.layout.display_name())
            .show_ui(ui, |ui| {
                for layout in WorldLayout::all() {
                    if ui
                        .selectable_label(
                            project.world_config.layout == *layout,
                            layout.display_name(),
                        )
                        .clicked()
                    {
                        project.world_config.layout = *layout;
                        project.mark_dirty();
                        // Apply layout if switching to linear
                        match layout {
                            WorldLayout::LinearHorizontal => {
                                apply_linear_layout(project, true);
                            }
                            WorldLayout::LinearVertical => {
                                apply_linear_layout(project, false);
                            }
                            WorldLayout::GridVania => {
                                apply_gridvania_snap(project);
                            }
                            _ => {}
                        }
                    }
                }
            });

        // Grid size for GridVania
        if project.world_config.layout == WorldLayout::GridVania {
            ui.separator();
            ui.label("Grid:");
            ui.add(
                egui::DragValue::new(&mut project.world_config.grid_width)
                    .range(64..=1024)
                    .suffix("px")
                    .speed(8),
            );
            ui.label("x");
            ui.add(
                egui::DragValue::new(&mut project.world_config.grid_height)
                    .range(64..=1024)
                    .suffix("px")
                    .speed(8),
            );
        }

        ui.separator();
        ui.checkbox(&mut editor_state.show_connections, "Connections");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add Level").clicked() {
                // Open dialog to get level details
                let (world_x, world_y) = find_empty_position(project);
                editor_state.world_new_level_dialog_open = true;
                editor_state.world_new_level_pos = (world_x, world_y);
                editor_state.world_new_level_name = format!("Level {}", project.levels.len() + 1);
                editor_state.world_new_level_width = 50;
                editor_state.world_new_level_height = 50;
            }

            // Zoom controls
            ui.separator();
            ui.label(format!(
                "{}%",
                (editor_state.world_view_zoom * 100.0) as i32
            ));
            if ui.button("-").clicked() {
                editor_state.world_view_zoom = (editor_state.world_view_zoom * 0.8).max(0.05);
            }
            if ui.button("+").clicked() {
                editor_state.world_view_zoom = (editor_state.world_view_zoom * 1.25).min(2.0);
            }
            if ui.button("Fit").clicked() {
                fit_world_to_view(editor_state, project, ui.available_size());
            }
        });
    });

    ui.separator();

    // World canvas area
    let available_rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available_rect, egui::Sense::click_and_drag());

    // Handle panning with middle mouse or right mouse drag
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        let delta = response.drag_delta();
        editor_state.world_view_offset.x += delta.x;
        editor_state.world_view_offset.y += delta.y;
    }

    // Handle ESC to cancel pending connection
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        editor_state.world_connection_from = None;
    }

    // Handle zoom with scroll wheel
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            let old_zoom = editor_state.world_view_zoom;
            editor_state.world_view_zoom =
                (editor_state.world_view_zoom * zoom_factor).clamp(0.05, 2.0);

            // Zoom toward cursor position
            if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let cursor_offset_x =
                    cursor_pos.x - available_rect.min.x - editor_state.world_view_offset.x;
                let cursor_offset_y =
                    cursor_pos.y - available_rect.min.y - editor_state.world_view_offset.y;
                let zoom_change = editor_state.world_view_zoom / old_zoom;
                editor_state.world_view_offset.x -= cursor_offset_x * (zoom_change - 1.0);
                editor_state.world_view_offset.y -= cursor_offset_y * (zoom_change - 1.0);
            }
        }
    }

    // Draw background grid for GridVania mode
    let painter = ui.painter_at(available_rect);
    if project.world_config.layout == WorldLayout::GridVania {
        draw_world_grid(&painter, available_rect, editor_state, project);
    }

    // Draw connections between levels
    if editor_state.show_connections {
        draw_connections(&painter, available_rect, editor_state, project);
    }

    // Draw each level
    let mut hovered_level: Option<Uuid> = None;
    let tile_size = get_default_tile_size(project);
    let mut new_connection: Option<bevy_map_core::LevelConnection> = None;
    let mut delete_connection: Option<Uuid> = None;

    for (level_idx, level) in project.levels.iter().enumerate() {
        // Calculate screen position
        let screen_x = available_rect.min.x
            + (level.world_x as f32 * editor_state.world_view_zoom)
            + editor_state.world_view_offset.x;
        let screen_y = available_rect.min.y
            + (level.world_y as f32 * editor_state.world_view_zoom)
            + editor_state.world_view_offset.y;

        // Calculate level size in screen space
        let level_width = level.width as f32 * tile_size as f32 * editor_state.world_view_zoom;
        let level_height = level.height as f32 * tile_size as f32 * editor_state.world_view_zoom;

        let level_rect = egui::Rect::from_min_size(
            egui::pos2(screen_x, screen_y),
            egui::vec2(level_width, level_height),
        );

        // Skip if outside visible area
        if !level_rect.intersects(available_rect) {
            continue;
        }

        // Clip to available area
        let clipped_rect = level_rect.intersect(available_rect);

        // Background color
        let bg_color = level
            .bg_color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or_else(|| default_level_color(level_idx));

        // Check if this level is selected or hovered
        let is_selected = editor_state.selected_level == Some(level.id);
        let is_hovered = editor_state.world_hovered_level == Some(level.id);
        let is_dragging = editor_state.world_dragging_level == Some(level.id);

        // Draw level rectangle
        painter.rect_filled(clipped_rect, 4.0, bg_color);

        // Draw border
        let border_color = if is_selected {
            egui::Color32::YELLOW
        } else if is_hovered || is_dragging {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_gray(80)
        };
        let border_width = if is_selected { 3.0 } else { 1.0 };
        painter.rect_stroke(
            clipped_rect,
            4.0,
            egui::Stroke::new(border_width, border_color),
            egui::StrokeKind::Outside,
        );

        // Draw level name if enough space
        if level_width > 40.0 && level_height > 20.0 {
            let text_pos = level_rect.center_top() + egui::vec2(0.0, 8.0);
            if available_rect.contains(text_pos) {
                painter.text(
                    text_pos,
                    egui::Align2::CENTER_TOP,
                    &level.name,
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            }

            // Show dimensions if enough space
            if level_height > 40.0 {
                let dims_text = format!("{}x{}", level.width, level.height);
                let dims_pos = level_rect.center_bottom() - egui::vec2(0.0, 8.0);
                if available_rect.contains(dims_pos) {
                    painter.text(
                        dims_pos,
                        egui::Align2::CENTER_BOTTOM,
                        dims_text,
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_gray(180),
                    );
                }
            }
        }

        // Handle level interaction
        let level_response = ui.interact(
            level_rect,
            egui::Id::new(("level", level.id)),
            egui::Sense::click_and_drag(),
        );

        if level_response.hovered() {
            hovered_level = Some(level.id);
        }

        // Double-click to open level
        if level_response.double_clicked() {
            result.open_level = Some(level.id);
        }

        // Single click to select (or Shift+Click for connection)
        if level_response.clicked() {
            let shift_held = ui.input(|i| i.modifiers.shift);

            if shift_held {
                // Shift+Click: connection creation
                if let Some(click_pos) = level_response.interact_pointer_pos() {
                    if let Some(edge) = detect_clicked_edge(level_rect, click_pos) {
                        if let Some((from_id, from_dir)) = editor_state.world_connection_from.take()
                        {
                            // Complete connection (if clicking different level)
                            if from_id != level.id {
                                new_connection = Some(bevy_map_core::LevelConnection::new(
                                    from_id, from_dir, level.id, edge,
                                ));
                            }
                        } else {
                            // Start connection
                            editor_state.world_connection_from = Some((level.id, edge));
                        }
                    }
                }
            } else {
                // Normal click: select level and cancel any pending connection
                editor_state.selected_level = Some(level.id);
                editor_state.world_connection_from = None;
            }
        }

        // Right-click context menu (also cancel pending connection)
        let level_connections: Vec<_> = project
            .world_config
            .connections_for(level.id)
            .into_iter()
            .map(|c| {
                (
                    c.id,
                    c.from_level,
                    c.from_direction,
                    c.to_level,
                    c.to_direction,
                )
            })
            .collect();

        level_response.context_menu(|ui| {
            if ui.button("Open Level").clicked() {
                result.open_level = Some(level.id);
                ui.close();
            }
            ui.separator();
            if ui.button("Rename...").clicked() {
                result.rename_level = Some(level.id);
                ui.close();
            }
            if ui.button("Duplicate").clicked() {
                result.duplicate_level = Some(level.id);
                ui.close();
            }

            // Show connections for this level
            if !level_connections.is_empty() {
                ui.separator();
                ui.label("Connections:");
                for (conn_id, from_id, from_dir, _to_id, to_dir) in &level_connections {
                    let label = if *from_id == level.id {
                        format!("{} -> ...", from_dir.short_name())
                    } else {
                        format!("... -> {}", to_dir.short_name())
                    };
                    ui.horizontal(|ui| {
                        ui.label(&label);
                        if ui.small_button("X").clicked() {
                            delete_connection = Some(*conn_id);
                            ui.close();
                        }
                    });
                }
            }

            ui.separator();
            if ui.button("Delete Level").clicked() {
                result.delete_level = Some(level.id);
                ui.close();
            }
        });

        // Handle drag start
        if level_response.drag_started() {
            editor_state.world_dragging_level = Some(level.id);
            if let Some(pos) = level_response.interact_pointer_pos() {
                editor_state.world_drag_start = Some(bevy::math::Vec2::new(pos.x, pos.y));
            }
            // Store original level position at drag start
            editor_state.world_drag_level_start_pos = Some((level.world_x, level.world_y));
            editor_state.selected_level = Some(level.id);
        }
    }

    // Apply deferred connection creation
    if let Some(connection) = new_connection {
        project.world_config.add_connection(connection);
        project.mark_dirty();
    }

    // Apply deferred connection deletion
    if let Some(conn_id) = delete_connection {
        project.world_config.remove_connection(conn_id);
        project.mark_dirty();
    }

    // Draw pending connection visualization
    if let Some((from_id, from_dir)) = editor_state.world_connection_from {
        if let Some(from_level) = project.levels.iter().find(|l| l.id == from_id) {
            let tile_size = get_default_tile_size(project) as f32;

            // Calculate source edge position in screen space
            let (world_x, world_y) = get_edge_center_world(
                from_level.world_x as f32,
                from_level.world_y as f32,
                from_level.width as f32 * tile_size,
                from_level.height as f32 * tile_size,
                from_dir,
            );

            let screen_x = available_rect.min.x
                + (world_x * editor_state.world_view_zoom)
                + editor_state.world_view_offset.x;
            let screen_y = available_rect.min.y
                + (world_y * editor_state.world_view_zoom)
                + editor_state.world_view_offset.y;

            let color = get_direction_color(from_dir);

            // Draw circle at source edge
            painter.circle_filled(egui::pos2(screen_x, screen_y), 8.0, color);

            // Draw line to cursor
            if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
                painter.line_segment(
                    [egui::pos2(screen_x, screen_y), cursor_pos],
                    egui::Stroke::new(2.0, color.gamma_multiply(0.7)),
                );
            }

            // Show hint text
            let hint_text = format!("Shift+Click edge to connect ({})", from_dir.short_name());
            painter.text(
                egui::pos2(available_rect.center().x, available_rect.min.y + 20.0),
                egui::Align2::CENTER_TOP,
                hint_text,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }

    // Handle ongoing drag (update level position)
    if let Some(dragging_id) = editor_state.world_dragging_level {
        if response.dragged() || ui.input(|i| i.pointer.any_down()) {
            if let Some(current_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(start_pos) = editor_state.world_drag_start {
                    if let Some((orig_x, orig_y)) = editor_state.world_drag_level_start_pos {
                        // Calculate total delta from original mouse position
                        let total_delta_x = current_pos.x - start_pos.x;
                        let total_delta_y = current_pos.y - start_pos.y;
                        let world_delta_x = (total_delta_x / editor_state.world_view_zoom) as i32;
                        let world_delta_y = (total_delta_y / editor_state.world_view_zoom) as i32;

                        if let Some(level) = project.levels.iter_mut().find(|l| l.id == dragging_id)
                        {
                            // New position = original position + total delta
                            let mut new_x = orig_x + world_delta_x;
                            let mut new_y = orig_y + world_delta_y;

                            // Apply snapping based on layout mode
                            match project.world_config.layout {
                                WorldLayout::GridVania => {
                                    let grid_w = project.world_config.grid_width as i32;
                                    let grid_h = project.world_config.grid_height as i32;
                                    // Round to nearest grid cell (not truncate)
                                    new_x =
                                        ((new_x as f32 / grid_w as f32).round() as i32) * grid_w;
                                    new_y =
                                        ((new_y as f32 / grid_h as f32).round() as i32) * grid_h;
                                }
                                _ => {}
                            }

                            level.world_x = new_x;
                            level.world_y = new_y;
                        }
                        // NOTE: We do NOT update world_drag_start here - keep original position
                    }
                }
            }
        }
    }

    // Handle drag end
    if !ui.input(|i| i.pointer.any_down()) {
        if editor_state.world_dragging_level.is_some() {
            editor_state.world_dragging_level = None;
            editor_state.world_drag_start = None;
            editor_state.world_drag_level_start_pos = None;
            project.mark_dirty();
        }
    }

    // Update hovered level
    editor_state.world_hovered_level = hovered_level;

    result
}

/// Draw the world grid for GridVania mode
fn draw_world_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    editor_state: &EditorState,
    project: &Project,
) {
    let grid_w = project.world_config.grid_width as f32 * editor_state.world_view_zoom;
    let grid_h = project.world_config.grid_height as f32 * editor_state.world_view_zoom;

    if grid_w < 10.0 || grid_h < 10.0 {
        return; // Grid too small to render
    }

    let offset = editor_state.world_view_offset;
    let line_color = egui::Color32::from_rgba_unmultiplied(100, 100, 100, 50);

    // Calculate grid line range
    let start_x = ((rect.min.x - offset.x) / grid_w).floor() as i32 - 1;
    let end_x = ((rect.max.x - offset.x) / grid_w).ceil() as i32 + 1;
    let start_y = ((rect.min.y - offset.y) / grid_h).floor() as i32 - 1;
    let end_y = ((rect.max.y - offset.y) / grid_h).ceil() as i32 + 1;

    // Limit grid lines to prevent performance issues
    let max_lines = 100;
    if (end_x - start_x) > max_lines || (end_y - start_y) > max_lines {
        return;
    }

    // Draw vertical lines
    for i in start_x..=end_x {
        let x = rect.min.x + (i as f32 * grid_w) + offset.x;
        if x >= rect.min.x && x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, line_color),
            );
        }
    }

    // Draw horizontal lines
    for i in start_y..=end_y {
        let y = rect.min.y + (i as f32 * grid_h) + offset.y;
        if y >= rect.min.y && y <= rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, line_color),
            );
        }
    }
}

/// Draw connections between levels
fn draw_connections(
    painter: &egui::Painter,
    rect: egui::Rect,
    editor_state: &EditorState,
    project: &Project,
) {
    use bevy_map_core::ConnectionDirection;

    let tile_size = get_default_tile_size(project) as f32;

    for connection in &project.world_config.connections {
        let from_level = project
            .levels
            .iter()
            .find(|l| l.id == connection.from_level);
        let to_level = project.levels.iter().find(|l| l.id == connection.to_level);

        if let (Some(from), Some(to)) = (from_level, to_level) {
            // Calculate edge center positions based on direction
            let (from_x, from_y) = get_edge_center_world(
                from.world_x as f32,
                from.world_y as f32,
                from.width as f32 * tile_size,
                from.height as f32 * tile_size,
                connection.from_direction,
            );
            let (to_x, to_y) = get_edge_center_world(
                to.world_x as f32,
                to.world_y as f32,
                to.width as f32 * tile_size,
                to.height as f32 * tile_size,
                connection.to_direction,
            );

            // Transform to screen space
            let from_screen_x = rect.min.x
                + (from_x * editor_state.world_view_zoom)
                + editor_state.world_view_offset.x;
            let from_screen_y = rect.min.y
                + (from_y * editor_state.world_view_zoom)
                + editor_state.world_view_offset.y;
            let to_screen_x = rect.min.x
                + (to_x * editor_state.world_view_zoom)
                + editor_state.world_view_offset.x;
            let to_screen_y = rect.min.y
                + (to_y * editor_state.world_view_zoom)
                + editor_state.world_view_offset.y;

            // Color based on direction
            let color = match connection.from_direction {
                ConnectionDirection::North => egui::Color32::from_rgb(100, 150, 255), // Blue
                ConnectionDirection::South => egui::Color32::from_rgb(100, 200, 100), // Green
                ConnectionDirection::East => egui::Color32::from_rgb(255, 200, 100), // Yellow/Orange
                ConnectionDirection::West => egui::Color32::from_rgb(255, 100, 100), // Red
            };

            painter.line_segment(
                [
                    egui::pos2(from_screen_x, from_screen_y),
                    egui::pos2(to_screen_x, to_screen_y),
                ],
                egui::Stroke::new(2.0, color),
            );

            // Draw arrowhead at destination
            draw_arrowhead(
                painter,
                to_screen_x,
                to_screen_y,
                from_screen_x,
                from_screen_y,
                color,
            );
        }
    }
}

/// Get the center position of a level edge in world coordinates
fn get_edge_center_world(
    level_x: f32,
    level_y: f32,
    level_w: f32,
    level_h: f32,
    direction: bevy_map_core::ConnectionDirection,
) -> (f32, f32) {
    use bevy_map_core::ConnectionDirection;

    match direction {
        ConnectionDirection::North => (level_x + level_w / 2.0, level_y),
        ConnectionDirection::South => (level_x + level_w / 2.0, level_y + level_h),
        ConnectionDirection::East => (level_x + level_w, level_y + level_h / 2.0),
        ConnectionDirection::West => (level_x, level_y + level_h / 2.0),
    }
}

/// Draw an arrowhead pointing from (from_x, from_y) to (to_x, to_y)
fn draw_arrowhead(
    painter: &egui::Painter,
    to_x: f32,
    to_y: f32,
    from_x: f32,
    from_y: f32,
    color: egui::Color32,
) {
    let arrow_size = 10.0;
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return;
    }

    let ux = dx / len;
    let uy = dy / len;

    // Perpendicular vector
    let px = -uy;
    let py = ux;

    let tip = egui::pos2(to_x, to_y);
    let left = egui::pos2(
        to_x - ux * arrow_size + px * arrow_size * 0.5,
        to_y - uy * arrow_size + py * arrow_size * 0.5,
    );
    let right = egui::pos2(
        to_x - ux * arrow_size - px * arrow_size * 0.5,
        to_y - uy * arrow_size - py * arrow_size * 0.5,
    );

    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        egui::Stroke::NONE,
    ));
}

/// Find an empty position for a new level
fn find_empty_position(project: &Project) -> (i32, i32) {
    let tile_size = get_default_tile_size(project) as i32;

    if project.levels.is_empty() {
        return (0, 0);
    }

    // Find rightmost level and place new one next to it
    let mut max_right = i32::MIN;
    let mut right_level_y = 0;

    for level in &project.levels {
        let right_edge = level.world_x + (level.width as i32 * tile_size);
        if right_edge > max_right {
            max_right = right_edge;
            right_level_y = level.world_y;
        }
    }

    // Add some padding
    let padding = match project.world_config.layout {
        WorldLayout::GridVania => project.world_config.grid_width as i32,
        _ => 32,
    };

    (max_right + padding, right_level_y)
}

/// Apply linear horizontal or vertical layout
fn apply_linear_layout(project: &mut Project, horizontal: bool) {
    let tile_size = get_default_tile_size(project) as i32;
    let padding = 32;

    let mut current_pos = 0i32;

    for level in &mut project.levels {
        if horizontal {
            level.world_x = current_pos;
            level.world_y = 0;
            current_pos += (level.width as i32 * tile_size) + padding;
        } else {
            level.world_x = 0;
            level.world_y = current_pos;
            current_pos += (level.height as i32 * tile_size) + padding;
        }
    }
}

/// Snap all levels to the GridVania grid
fn apply_gridvania_snap(project: &mut Project) {
    let grid_w = project.world_config.grid_width as i32;
    let grid_h = project.world_config.grid_height as i32;

    for level in &mut project.levels {
        level.world_x = (level.world_x / grid_w) * grid_w;
        level.world_y = (level.world_y / grid_h) * grid_h;
    }
}

/// Fit all levels into the view
fn fit_world_to_view(editor_state: &mut EditorState, project: &Project, view_size: egui::Vec2) {
    if project.levels.is_empty() {
        editor_state.world_view_zoom = 0.25;
        editor_state.world_view_offset = bevy::math::Vec2::ZERO;
        return;
    }

    let tile_size = get_default_tile_size(project) as f32;

    // Calculate bounding box of all levels
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for level in &project.levels {
        let x = level.world_x as f32;
        let y = level.world_y as f32;
        let w = level.width as f32 * tile_size;
        let h = level.height as f32 * tile_size;

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let world_width = max_x - min_x;
    let world_height = max_y - min_y;

    // Calculate zoom to fit with padding
    let padding = 50.0;
    let available_width = view_size.x - padding * 2.0;
    let available_height = view_size.y - padding * 2.0;

    let zoom_x = available_width / world_width.max(1.0);
    let zoom_y = available_height / world_height.max(1.0);
    let zoom = zoom_x.min(zoom_y).clamp(0.05, 2.0);

    editor_state.world_view_zoom = zoom;

    // Center the content
    let content_width = world_width * zoom;
    let content_height = world_height * zoom;
    let offset_x = (view_size.x - content_width) / 2.0 - min_x * zoom;
    let offset_y = (view_size.y - content_height) / 2.0 - min_y * zoom;

    editor_state.world_view_offset = bevy::math::Vec2::new(offset_x, offset_y);
}

/// Get the default tile size from the first tileset
fn get_default_tile_size(project: &Project) -> u32 {
    project.tilesets.first().map(|t| t.tile_size).unwrap_or(32)
}

/// Detect which edge of a level rectangle was clicked
/// Returns None if click is in center (not on an edge)
fn detect_clicked_edge(
    rect: egui::Rect,
    click_pos: egui::Pos2,
) -> Option<bevy_map_core::ConnectionDirection> {
    use bevy_map_core::ConnectionDirection;

    // Edge detection margin (in pixels)
    let margin = rect.width().min(rect.height()) * 0.25; // 25% of smaller dimension
    let margin = margin.max(15.0).min(40.0); // Clamp to reasonable range

    // Check each edge (prioritize by distance from edge)
    let dist_top = click_pos.y - rect.min.y;
    let dist_bottom = rect.max.y - click_pos.y;
    let dist_left = click_pos.x - rect.min.x;
    let dist_right = rect.max.x - click_pos.x;

    // Find closest edge
    let min_dist = dist_top.min(dist_bottom).min(dist_left).min(dist_right);

    if min_dist > margin {
        return None; // Click is in center, not on edge
    }

    if min_dist == dist_top {
        Some(ConnectionDirection::North)
    } else if min_dist == dist_bottom {
        Some(ConnectionDirection::South)
    } else if min_dist == dist_right {
        Some(ConnectionDirection::East)
    } else {
        Some(ConnectionDirection::West)
    }
}

/// Get color for a connection direction
fn get_direction_color(direction: bevy_map_core::ConnectionDirection) -> egui::Color32 {
    use bevy_map_core::ConnectionDirection;
    match direction {
        ConnectionDirection::North => egui::Color32::from_rgb(100, 150, 255), // Blue
        ConnectionDirection::South => egui::Color32::from_rgb(100, 200, 100), // Green
        ConnectionDirection::East => egui::Color32::from_rgb(255, 200, 100),  // Yellow/Orange
        ConnectionDirection::West => egui::Color32::from_rgb(255, 100, 100),  // Red
    }
}

/// Render the new level dialog
/// Returns Some(NewLevelParams) if user confirms, None otherwise
pub fn render_new_level_dialog(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
) -> Option<NewLevelParams> {
    if !editor_state.world_new_level_dialog_open {
        return None;
    }

    let mut result = None;
    let mut close_dialog = false;

    egui::Window::new("New Level")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add_sized(
                    [200.0, 20.0],
                    egui::TextEdit::singleline(&mut editor_state.world_new_level_name),
                );
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(
                    egui::DragValue::new(&mut editor_state.world_new_level_width)
                        .range(1..=1000)
                        .speed(1),
                );
                ui.label("tiles");
            });

            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(
                    egui::DragValue::new(&mut editor_state.world_new_level_height)
                        .range(1..=1000)
                        .speed(1),
                );
                ui.label("tiles");
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    result = Some(NewLevelParams {
                        name: editor_state.world_new_level_name.clone(),
                        width: editor_state.world_new_level_width,
                        height: editor_state.world_new_level_height,
                        world_x: editor_state.world_new_level_pos.0,
                        world_y: editor_state.world_new_level_pos.1,
                    });
                    close_dialog = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        editor_state.world_new_level_dialog_open = false;
    }

    result
}
