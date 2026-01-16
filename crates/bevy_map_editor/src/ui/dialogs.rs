//! Dialog windows for the editor

use bevy_egui::egui;

use crate::project::Project;
use crate::EditorState;
use crate::{AssetsBasePath, CopyFileCallback};

/// Actions that can be triggered from menus
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    New,
    Open,
    Save,
    SaveAs,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    /// Create a stamp from the current tile selection
    CreateStampFromSelection,
    /// Open the game settings dialog
    OpenGameSettings,
    /// Run the game (save first, then launch)
    RunGame,
    /// Create a new game project using Bevy CLI
    CreateGameProject,
    /// Install Bevy CLI
    InstallBevyCli,
}

/// Render all dialogs
pub fn render_dialogs(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
    assets_base_path: &AssetsBasePath,
) {
    render_new_level_dialog(ctx, editor_state, project);
    render_new_tileset_dialog(ctx, editor_state, project, assets_base_path);
    render_add_tileset_image_dialog(ctx, editor_state, project, assets_base_path);
    render_copy_file_dialog(ctx, editor_state, project, assets_base_path);
    render_about_dialog(ctx, editor_state);
    render_error_dialog(ctx, editor_state);

    // Handle pending file actions
    if let Some(action) = editor_state.pending_action.take() {
        match action {
            PendingAction::New => {
                editor_state.show_new_project_dialog = true;
            }
            PendingAction::Open => {
                #[cfg(feature = "native")]
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Map Project", &["map.json", "json"])
                        .pick_file()
                    {
                        match Project::load(&path) {
                            Ok(loaded) => {
                                *project = loaded;
                                // Add to recent projects
                                editor_state.pending_add_recent_project = Some(path);
                            }
                            Err(e) => {
                                editor_state.error_message =
                                    Some(format!("Failed to load project: {}", e));
                            }
                        }
                    }
                }
            }
            PendingAction::Save => {
                if project.path.is_some() {
                    if let Err(e) = project.save_current() {
                        editor_state.error_message = Some(format!("Failed to save: {}", e));
                    } else {
                        // Auto-sync to game if running (triggers hot-reload)
                        if let crate::game_runner::GameBuildState::Running { .. } =
                            &editor_state.game_build_state
                        {
                            if let (Some(map_path), Some(game_path)) =
                                (&project.path, &project.game_config.project_path)
                            {
                                if let Err(e) =
                                    crate::game_runner::sync_map_to_game(map_path, game_path)
                                {
                                    bevy::log::warn!("Failed to sync to running game: {}", e);
                                } else {
                                    bevy::log::info!(
                                        "Synced map to running game (hot-reload triggered)"
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // No path set, trigger Save As
                    editor_state.pending_action = Some(PendingAction::SaveAs);
                }
            }
            PendingAction::SaveAs => {
                #[cfg(feature = "native")]
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Map Project", &["map.json", "json"])
                        .save_file()
                    {
                        match project.save(&path) {
                            Ok(()) => {
                                // Add to recent projects
                                editor_state.pending_add_recent_project = Some(path);
                            }
                            Err(e) => {
                                editor_state.error_message = Some(format!("Failed to save: {}", e));
                            }
                        }
                    }
                }
            }
            _ => {
                // Put other actions back
                editor_state.pending_action = Some(action);
            }
        }
    }
}

fn render_new_level_dialog(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
) {
    if !editor_state.show_new_level_dialog {
        return;
    }

    egui::Window::new("New Level")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut editor_state.new_level_name);
            });

            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut editor_state.new_level_width).range(1..=1000));
            });

            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut editor_state.new_level_height).range(1..=1000));
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    let level = bevy_map_core::Level::new(
                        editor_state.new_level_name.clone(),
                        editor_state.new_level_width,
                        editor_state.new_level_height,
                    );
                    let level_id = level.id;
                    project.add_level(level);
                    editor_state.selected_level = Some(level_id);
                    editor_state.show_new_level_dialog = false;

                    // Reset dialog state
                    editor_state.new_level_name = "New Level".to_string();
                    editor_state.new_level_width = 50;
                    editor_state.new_level_height = 50;
                }
                if ui.button("Cancel").clicked() {
                    editor_state.show_new_level_dialog = false;
                }
            });
        });
}

fn render_new_tileset_dialog(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
    _assets_base_path: &AssetsBasePath,
) {
    if !editor_state.show_new_tileset_dialog {
        return;
    }

    // Check if path is valid (file exists)
    let path_status = if editor_state.new_tileset_path.is_empty() {
        PathStatus::Empty
    } else {
        let path = std::path::Path::new(&editor_state.new_tileset_path);
        if path.exists() {
            PathStatus::Valid
        } else {
            PathStatus::NotFound
        }
    };

    egui::Window::new("New Tileset")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut editor_state.new_tileset_name);
            });

            ui.horizontal(|ui| {
                ui.label("Tile Size:");
                ui.add(
                    egui::DragValue::new(&mut editor_state.new_tileset_tile_size).range(1..=256),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Image Path:");
                ui.text_edit_singleline(&mut editor_state.new_tileset_path);
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        editor_state.new_tileset_path = path.to_string_lossy().to_string();
                    }
                }
            });

            // Show path status warning (only for file not found)
            if path_status == PathStatus::NotFound {
                ui.colored_label(egui::Color32::RED, "File not found at this path");
            }

            ui.separator();

            ui.horizontal(|ui| {
                let can_create = !editor_state.new_tileset_path.is_empty()
                    && path_status != PathStatus::NotFound
                    && path_status != PathStatus::Empty;

                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create").clicked() {
                        let path = std::path::PathBuf::from(&editor_state.new_tileset_path);
                        // Use absolute path directly - Bevy's UnapprovedPathMode::Allow handles this
                        create_tileset_from_path(editor_state, project, path);
                    }
                });

                if ui.button("Cancel").clicked() {
                    editor_state.show_new_tileset_dialog = false;
                }
            });
        });
}

/// Path validation status
#[derive(PartialEq)]
enum PathStatus {
    Empty,
    Valid,
    NotFound,
}

/// Helper to create tileset from a path (can be absolute or relative)
fn create_tileset_from_path(
    editor_state: &mut EditorState,
    project: &mut Project,
    path: std::path::PathBuf,
) {
    let path_str = path.to_string_lossy().to_string();

    let tileset = bevy_map_core::Tileset::new(
        editor_state.new_tileset_name.clone(),
        path_str,
        editor_state.new_tileset_tile_size,
        0, // columns - will be determined when texture loads
        0, // rows
    );
    let tileset_id = tileset.id;
    project.add_tileset(tileset);
    editor_state.selected_tileset = Some(tileset_id);
    editor_state.show_new_tileset_dialog = false;

    // Reset dialog state
    editor_state.new_tileset_name = "New Tileset".to_string();
    editor_state.new_tileset_path = String::new();
    editor_state.new_tileset_tile_size = 32;
}

/// Render the copy file confirmation dialog
fn render_copy_file_dialog(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
    assets_base_path: &AssetsBasePath,
) {
    if !editor_state.show_copy_file_dialog {
        return;
    }

    let source_path = match &editor_state.pending_copy_source {
        Some(path) => path.clone(),
        None => {
            editor_state.show_copy_file_dialog = false;
            return;
        }
    };

    let filename = source_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    egui::Window::new("Copy File to Assets")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("The selected file is outside the assets folder.");
            ui.label("Bevy can only load assets from the assets directory.");
            ui.separator();
            ui.label(format!("File: {}", filename));
            ui.label(format!("From: {}", source_path.display()));
            ui.label(format!(
                "To: {}/tiles/{}",
                assets_base_path.path().display(),
                filename
            ));
            ui.separator();
            ui.label("Copy this file to the assets folder?");
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Copy File").clicked() {
                    // Attempt to copy the file
                    match assets_base_path.copy_to_assets(&source_path) {
                        Ok(relative_path) => {
                            // Successfully copied, now execute the callback
                            match editor_state.pending_copy_callback {
                                CopyFileCallback::NewTileset => {
                                    create_tileset_from_path(editor_state, project, relative_path);
                                }
                                CopyFileCallback::AddTilesetImage => {
                                    add_tileset_image_from_path(
                                        editor_state,
                                        project,
                                        relative_path,
                                    );
                                }
                                CopyFileCallback::None => {}
                            }
                        }
                        Err(e) => {
                            editor_state.error_message =
                                Some(format!("Failed to copy file: {}", e));
                        }
                    }

                    // Clean up dialog state
                    editor_state.show_copy_file_dialog = false;
                    editor_state.pending_copy_source = None;
                    editor_state.pending_copy_callback = CopyFileCallback::None;
                }

                if ui.button("Cancel").clicked() {
                    editor_state.show_copy_file_dialog = false;
                    editor_state.pending_copy_source = None;
                    editor_state.pending_copy_callback = CopyFileCallback::None;
                }
            });
        });
}

/// Helper to add image to tileset from a path (can be absolute or relative)
fn add_tileset_image_from_path(
    editor_state: &mut EditorState,
    project: &mut Project,
    path: std::path::PathBuf,
) {
    if let Some(tileset_id) = editor_state.selected_tileset {
        if let Some(tileset) = project.tilesets.iter_mut().find(|t| t.id == tileset_id) {
            tileset.add_image(
                editor_state.add_image_name.clone(),
                path.to_string_lossy().to_string(),
                8, // Default columns - will be recalculated when loaded
                8, // Default rows
            );
            project.mark_dirty();
        }
    }

    editor_state.show_add_tileset_image_dialog = false;
    editor_state.add_image_name.clear();
    editor_state.add_image_path.clear();
}

fn render_about_dialog(ctx: &egui::Context, editor_state: &mut EditorState) {
    if !editor_state.show_about_dialog {
        return;
    }

    egui::Window::new("About bevy_map_editor")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("bevy_map_editor");
            ui.label("A full-featured tilemap editor for Bevy games");
            ui.separator();
            ui.label("Features:");
            ui.label("- Tile painting with brush tools");
            ui.label("- Tiled-compatible terrain autotiling");
            ui.label("- Undo/redo support");
            ui.label("- Copy/paste operations");
            ui.label("- Multi-image tilesets");
            ui.separator();
            if ui.button("Close").clicked() {
                editor_state.show_about_dialog = false;
            }
        });
}

fn render_error_dialog(ctx: &egui::Context, editor_state: &mut EditorState) {
    let Some(error_msg) = editor_state.error_message.clone() else {
        return;
    };

    egui::Window::new("Error")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(&error_msg);
            ui.separator();
            if ui.button("OK").clicked() {
                editor_state.error_message = None;
            }
        });
}

fn render_add_tileset_image_dialog(
    ctx: &egui::Context,
    editor_state: &mut EditorState,
    project: &mut Project,
    _assets_base_path: &AssetsBasePath,
) {
    if !editor_state.show_add_tileset_image_dialog {
        return;
    }

    let tileset_name = editor_state
        .selected_tileset
        .and_then(|id| project.tilesets.iter().find(|t| t.id == id))
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Check if path is valid (file exists)
    let path_status = if editor_state.add_image_path.is_empty() {
        PathStatus::Empty
    } else {
        let path = std::path::Path::new(&editor_state.add_image_path);
        if path.exists() {
            PathStatus::Valid
        } else {
            PathStatus::NotFound
        }
    };

    egui::Window::new(format!("Add Image to {}", tileset_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Image Name:");
                ui.text_edit_singleline(&mut editor_state.add_image_name);
            });

            ui.horizontal(|ui| {
                ui.label("Image Path:");
                ui.add(
                    egui::TextEdit::singleline(&mut editor_state.add_image_path)
                        .desired_width(200.0),
                );
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        editor_state.add_image_path = path.to_string_lossy().to_string();
                    }
                }
            });

            // Show path status warning (only for file not found)
            if path_status == PathStatus::NotFound {
                ui.colored_label(egui::Color32::RED, "File not found at this path");
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    editor_state.show_add_tileset_image_dialog = false;
                    editor_state.add_image_name.clear();
                    editor_state.add_image_path.clear();
                }

                let can_add = !editor_state.add_image_name.is_empty()
                    && !editor_state.add_image_path.is_empty()
                    && editor_state.selected_tileset.is_some()
                    && path_status != PathStatus::NotFound
                    && path_status != PathStatus::Empty;

                ui.add_enabled_ui(can_add, |ui| {
                    if ui.button("Add Image").clicked() {
                        let path = std::path::PathBuf::from(&editor_state.add_image_path);
                        // Use absolute path directly - Bevy's UnapprovedPathMode::Allow handles this
                        add_tileset_image_from_path(editor_state, project, path);
                    }
                });
            });
        });
}
