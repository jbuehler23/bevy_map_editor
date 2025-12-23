//! Settings/Preferences dialog

use bevy_egui::egui;

use crate::preferences::EditorPreferences;
use crate::ui::{EditorTool, ToolMode};

/// Render the Settings dialog
pub fn render_settings_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    preferences: &mut EditorPreferences,
) {
    if !*show {
        return;
    }

    let mut close_dialog = false;
    let mut save_and_close = false;

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(true)
        .default_size([450.0, 400.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Startup section
                ui.heading("Startup");
                ui.separator();

                ui.checkbox(
                    &mut preferences.auto_open_last_project,
                    "Auto-open last project on startup",
                );

                ui.add_space(16.0);

                // Default View Settings section
                ui.heading("Default View Settings");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.checkbox(&mut preferences.show_grid, "Show Grid");
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut preferences.show_collisions, "Show Collisions");
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut preferences.snap_to_grid, "Snap to Grid");
                });

                ui.horizontal(|ui| {
                    ui.label("Default Zoom:");
                    ui.add(
                        egui::Slider::new(&mut preferences.zoom, 0.25..=4.0)
                            .suffix("x")
                            .logarithmic(true),
                    );
                });

                ui.add_space(16.0);

                // Default Tool section
                ui.heading("Default Tool");
                ui.separator();

                let tools = [
                    (EditorTool::Select, "Select"),
                    (EditorTool::Paint, "Paint"),
                    (EditorTool::Erase, "Erase"),
                    (EditorTool::Fill, "Fill"),
                    (EditorTool::Terrain, "Terrain"),
                    (EditorTool::Entity, "Entity"),
                ];

                ui.horizontal_wrapped(|ui| {
                    for (tool, name) in tools {
                        if ui
                            .selectable_label(preferences.default_tool == tool, name)
                            .clicked()
                        {
                            preferences.default_tool = tool;
                        }
                    }
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Default Tool Mode:");
                    egui::ComboBox::from_id_salt("default_tool_mode")
                        .selected_text(preferences.default_tool_mode.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut preferences.default_tool_mode,
                                ToolMode::Point,
                                "Point",
                            );
                            ui.selectable_value(
                                &mut preferences.default_tool_mode,
                                ToolMode::Rectangle,
                                "Rectangle",
                            );
                        });
                });

                ui.add_space(16.0);

                // Panel Defaults section
                ui.heading("Panel Defaults");
                ui.separator();

                ui.checkbox(&mut preferences.show_tree_view, "Show Project Tree");
                ui.checkbox(&mut preferences.show_inspector, "Show Inspector");

                ui.horizontal(|ui| {
                    ui.label("Tree View Width:");
                    ui.add(
                        egui::DragValue::new(&mut preferences.tree_view_width).range(100.0..=500.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Inspector Width:");
                    ui.add(
                        egui::DragValue::new(&mut preferences.inspector_width).range(150.0..=600.0),
                    );
                });

                ui.add_space(16.0);

                // Recent Projects section
                ui.heading("Recent Projects");
                ui.separator();

                if preferences.recent_projects.is_empty() {
                    ui.label("No recent projects");
                } else {
                    ui.label(format!(
                        "{} recent project(s)",
                        preferences.recent_projects.len()
                    ));
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save_and_close = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if save_and_close {
        if let Err(e) = preferences.save() {
            bevy::log::error!("Failed to save preferences: {}", e);
        }
        *show = false;
    }

    if close_dialog {
        // Reload preferences to discard changes
        *preferences = EditorPreferences::load();
        *show = false;
    }
}
