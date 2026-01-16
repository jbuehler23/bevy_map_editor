//! Game project settings dialog
//!
//! This dialog allows users to configure the associated game project,
//! including the project path, starting level, and build options.

use bevy_egui::egui;
use std::path::PathBuf;
use uuid::Uuid;

use crate::bevy_cli;
use crate::project::Project;

/// State for the game settings dialog
#[derive(Default)]
pub struct GameSettingsDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// Full path to the game project (e.g., C:\Dev\Games\my_game)
    pub project_path_input: String,
    /// Selected starting level ID
    pub selected_starting_level: Option<Uuid>,
    /// Whether to use release build
    pub use_release_build: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Whether Bevy CLI is installed (cached)
    pub cli_installed: Option<bool>,
}

impl GameSettingsDialogState {
    /// Initialize dialog state from project config
    pub fn load_from_project(&mut self, project: &Project) {
        self.project_path_input = project
            .game_config
            .project_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.selected_starting_level = project.game_config.starting_level;
        self.use_release_build = project.game_config.use_release_build;
        self.status_message = None;
    }

    /// Check and cache CLI installation status
    pub fn check_cli_status(&mut self) {
        if self.cli_installed.is_none() {
            self.cli_installed = Some(bevy_cli::is_bevy_cli_installed());
        }
    }

    /// Extract the project name from the path (last component)
    pub fn get_project_name(&self) -> Option<String> {
        let path = PathBuf::from(&self.project_path_input);
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    /// Get the parent directory of the project path
    pub fn get_parent_dir(&self) -> Option<PathBuf> {
        let path = PathBuf::from(&self.project_path_input);
        path.parent().map(|p| p.to_path_buf())
    }
}

/// Result of rendering the game settings dialog
#[derive(Default)]
pub struct GameSettingsDialogResult {
    /// User wants to save the settings
    pub save_requested: bool,
    /// User wants to create a new game project
    pub create_project_requested: bool,
    /// User wants to install Bevy CLI
    pub install_cli_requested: bool,
}

/// Render the game settings dialog
pub fn render_game_settings_dialog(
    ctx: &egui::Context,
    state: &mut GameSettingsDialogState,
    project: &mut Project,
) -> GameSettingsDialogResult {
    let mut result = GameSettingsDialogResult::default();

    if !state.open {
        return result;
    }

    // Check CLI status on first open
    state.check_cli_status();

    // Modal overlay - blocks all input behind the dialog
    egui::Area::new(egui::Id::new("game_settings_modal_overlay"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| {
                i.raw.screen_rect.unwrap_or(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                ))
            });
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click_and_drag());
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));
            // Consume all interactions
            response.context_menu(|_| {});
        });

    egui::Window::new("Game Project Settings")
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.heading("Game Project Configuration");
            ui.separator();

            // CLI Status
            let cli_installed = state.cli_installed.unwrap_or(false);
            ui.horizontal(|ui| {
                ui.label("Bevy CLI:");
                if cli_installed {
                    ui.colored_label(egui::Color32::GREEN, "Installed");
                    if let Some(version) = bevy_cli::get_bevy_cli_version() {
                        ui.label(format!("({})", version));
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Not installed");
                    if ui.button("Install").clicked() {
                        result.install_cli_requested = true;
                    }
                }
            });

            ui.add_space(8.0);

            // Project Path - single full path input
            ui.label("Game Project Path:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_path_input)
                        .desired_width(350.0)
                        .hint_text("C:\\Dev\\Games\\my_game"),
                );
                #[cfg(feature = "native")]
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_directory(std::env::current_dir().unwrap_or_default())
                        .pick_folder()
                    {
                        state.project_path_input = path.to_string_lossy().to_string();
                    }
                }
            });

            // Show path status and derived project name
            let path = PathBuf::from(&state.project_path_input);
            let project_name = state.get_project_name();
            let project_exists = path.join("Cargo.toml").exists();

            if !state.project_path_input.is_empty() {
                if project_exists {
                    ui.colored_label(
                        egui::Color32::GREEN,
                        "Valid game project found - ready to run",
                    );
                } else if path.exists() {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Directory exists but no Cargo.toml - use Create to scaffold",
                    );
                } else if let Some(ref name) = project_name {
                    ui.colored_label(
                        egui::Color32::LIGHT_GRAY,
                        format!("Will create new project \"{}\"", name),
                    );
                } else {
                    ui.colored_label(egui::Color32::RED, "Invalid path");
                }
            }

            ui.add_space(8.0);

            // Starting Level dropdown
            ui.horizontal(|ui| {
                ui.label("Starting Level:");

                let current_name = state
                    .selected_starting_level
                    .and_then(|id| project.get_level(id))
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "(Select a level)".to_string());

                egui::ComboBox::from_id_salt("starting_level_combo")
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        for level in &project.levels {
                            let is_selected = state.selected_starting_level == Some(level.id);
                            if ui.selectable_label(is_selected, &level.name).clicked() {
                                state.selected_starting_level = Some(level.id);
                            }
                        }
                    });
            });

            ui.add_space(8.0);

            // Build options
            ui.checkbox(
                &mut state.use_release_build,
                "Use release build (slower to compile, faster to run)",
            );

            // Status message
            if let Some(msg) = &state.status_message {
                ui.separator();
                ui.label(msg);
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                // Create Game Project button - enabled when CLI installed, path set, name valid, and doesn't exist
                let can_create = cli_installed && project_name.is_some() && !project_exists;

                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create Game Project").clicked() {
                        result.create_project_requested = true;
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        state.open = false;
                    }

                    // Can save if path is set and starting level selected
                    let can_save = !state.project_path_input.is_empty()
                        && state.selected_starting_level.is_some();

                    ui.add_enabled_ui(can_save, |ui| {
                        if ui.button("Save").clicked() {
                            // Update project config with full path
                            project.game_config.project_path =
                                Some(PathBuf::from(&state.project_path_input));
                            project.game_config.starting_level = state.selected_starting_level;
                            project.game_config.use_release_build = state.use_release_build;
                            project.mark_dirty();

                            result.save_requested = true;
                            state.open = false;
                        }
                    });
                });
            });
        });

    result
}
