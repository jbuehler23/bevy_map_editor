//! Asset Browser - File system browser panel for the map editor

use bevy_egui::egui;
use std::path::PathBuf;

/// Filter settings for the asset browser
#[derive(Debug, Clone)]
pub struct FileFilter {
    pub show_images: bool,
    pub show_audio: bool,
    pub show_json: bool,
    pub show_folders: bool,
    pub search_text: String,
    pub custom_extensions: String,
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            show_images: true,
            show_audio: false,
            show_json: true,
            show_folders: true,
            search_text: String::new(),
            custom_extensions: String::new(),
        }
    }
}

/// Sort order for files
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SortOrder {
    #[default]
    Name,
    Date,
    Size,
    Type,
}

/// View mode for the browser
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Grid,
    List,
}

/// State for the asset browser
#[derive(Debug, Clone)]
pub struct AssetBrowserState {
    pub current_path: PathBuf,
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    pub filter: FileFilter,
    pub selected_file: Option<PathBuf>,
    pub sort_order: SortOrder,
    pub view_mode: ViewMode,
    pub thumbnail_size: f32,
    /// Cached directory entries
    cached_entries: Option<Vec<FileEntry>>,
    /// Path that was cached
    cached_path: Option<PathBuf>,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        let current_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            current_path: current_path.clone(),
            history: vec![current_path],
            history_index: 0,
            filter: FileFilter::default(),
            selected_file: None,
            sort_order: SortOrder::Name,
            view_mode: ViewMode::Grid,
            thumbnail_size: 64.0,
            cached_entries: None,
            cached_path: None,
        }
    }
}

/// Represents a file or folder entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    /// Pre-computed lowercase name for efficient sorting/searching
    pub name_lower: String,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub size: u64,
}

impl FileEntry {
    fn from_path(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let name_lower = name.to_lowercase();
        let is_dir = path.is_dir();
        let extension = if is_dir {
            None
        } else {
            path.extension().map(|e| e.to_string_lossy().to_lowercase())
        };
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        Some(Self {
            path,
            name,
            name_lower,
            is_dir,
            extension,
            size,
        })
    }

    fn is_image(&self) -> bool {
        matches!(
            self.extension.as_deref(),
            Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
        )
    }

    fn is_audio(&self) -> bool {
        matches!(
            self.extension.as_deref(),
            Some("ogg" | "wav" | "mp3" | "flac")
        )
    }

    fn is_json(&self) -> bool {
        matches!(self.extension.as_deref(), Some("json"))
    }

    fn file_type_icon(&self) -> &'static str {
        if self.is_dir {
            "\u{1F4C1}" // folder icon
        } else if self.is_image() {
            "\u{1F5BC}" // image icon
        } else if self.is_audio() {
            "\u{1F3B5}" // music note
        } else if self.is_json() {
            "\u{1F4C4}" // document
        } else {
            "\u{1F4C3}" // generic file
        }
    }
}

/// Result from rendering the asset browser
#[derive(Default)]
pub struct AssetBrowserResult {
    pub file_activated: Option<PathBuf>,
}

impl AssetBrowserState {
    /// Navigate to a new path
    pub fn navigate_to(&mut self, path: PathBuf) {
        if path.is_dir() && path != self.current_path {
            // Truncate forward history when navigating to new path
            self.history.truncate(self.history_index + 1);
            self.history.push(path.clone());
            self.history_index = self.history.len() - 1;
            self.current_path = path;
            self.selected_file = None;
            self.invalidate_cache();
        }
    }

    /// Go back in history
    pub fn go_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.selected_file = None;
            self.invalidate_cache();
        }
    }

    /// Go forward in history
    pub fn go_forward(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            self.selected_file = None;
            self.invalidate_cache();
        }
    }

    /// Go up one directory
    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf());
        }
    }

    /// Invalidate the directory cache
    pub fn invalidate_cache(&mut self) {
        self.cached_entries = None;
        self.cached_path = None;
    }

    /// Get filtered and sorted entries for current directory
    fn get_entries(&mut self) -> Vec<FileEntry> {
        // Check if cache is valid
        if self.cached_path.as_ref() == Some(&self.current_path) {
            if let Some(ref entries) = self.cached_entries {
                return self.filter_and_sort(entries.clone());
            }
        }

        // Read directory
        let entries: Vec<FileEntry> = std::fs::read_dir(&self.current_path)
            .ok()
            .map(|rd| {
                rd.filter_map(|entry| entry.ok())
                    .filter_map(|entry| FileEntry::from_path(entry.path()))
                    .collect()
            })
            .unwrap_or_default();

        // Cache the raw entries
        self.cached_entries = Some(entries.clone());
        self.cached_path = Some(self.current_path.clone());

        self.filter_and_sort(entries)
    }

    /// Apply filters and sorting to entries
    fn filter_and_sort(&self, mut entries: Vec<FileEntry>) -> Vec<FileEntry> {
        // Pre-compute search text lowercase once (not per entry)
        let search_lower = self.filter.search_text.to_lowercase();

        // Pre-parse custom extensions once
        let custom_exts: Vec<&str> = if !self.filter.custom_extensions.is_empty() {
            self.filter
                .custom_extensions
                .split(',')
                .map(|s| s.trim().trim_start_matches('.'))
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        // Filter
        entries.retain(|entry| {
            // Always show directories if filter enabled
            if entry.is_dir {
                return self.filter.show_folders;
            }

            // Check file type filters
            let type_match = (self.filter.show_images && entry.is_image())
                || (self.filter.show_audio && entry.is_audio())
                || (self.filter.show_json && entry.is_json());

            // Check custom extensions
            let custom_match = if !custom_exts.is_empty() {
                entry.extension.as_ref().map_or(false, |ext| {
                    custom_exts.iter().any(|ce| ce.eq_ignore_ascii_case(ext))
                })
            } else {
                false
            };

            // Check search text (use cached lowercase name)
            let search_match = search_lower.is_empty() || entry.name_lower.contains(&search_lower);

            (type_match
                || custom_match
                || custom_exts.is_empty()
                    && !self.filter.show_images
                    && !self.filter.show_audio
                    && !self.filter.show_json)
                && search_match
        });

        // Sort - directories first, then by sort order
        entries.sort_by(|a, b| {
            // Directories always first
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Same type, apply sort order (use cached lowercase)
                    match self.sort_order {
                        SortOrder::Name => a.name_lower.cmp(&b.name_lower),
                        SortOrder::Size => a.size.cmp(&b.size),
                        SortOrder::Type => {
                            let ext_a = a.extension.as_deref().unwrap_or("");
                            let ext_b = b.extension.as_deref().unwrap_or("");
                            ext_a.cmp(ext_b).then_with(|| a.name.cmp(&b.name))
                        }
                        SortOrder::Date => a.name.cmp(&b.name), // TODO: actual date sorting
                    }
                }
            }
        });

        entries
    }
}

/// Render the asset browser panel
pub fn render_asset_browser(
    ui: &mut egui::Ui,
    state: &mut AssetBrowserState,
) -> AssetBrowserResult {
    let mut result = AssetBrowserResult::default();

    // Toolbar row
    ui.horizontal(|ui| {
        // Navigation buttons
        if ui
            .add_enabled(state.history_index > 0, egui::Button::new("\u{2190}"))
            .clicked()
        {
            state.go_back();
        }
        if ui
            .add_enabled(
                state.history_index < state.history.len() - 1,
                egui::Button::new("\u{2192}"),
            )
            .clicked()
        {
            state.go_forward();
        }
        if ui.button("\u{2191}").on_hover_text("Go up").clicked() {
            state.go_up();
        }
        if ui.button("\u{21BB}").on_hover_text("Refresh").clicked() {
            state.invalidate_cache();
        }

        ui.separator();

        // Path display (clickable breadcrumbs)
        ui.horizontal(|ui| {
            // Clone the path to avoid borrow issues
            let current_path = state.current_path.clone();
            let components: Vec<_> = current_path.iter().collect();
            let mut click_path = PathBuf::new();
            let mut navigate_target: Option<PathBuf> = None;

            for (i, component) in components.iter().enumerate() {
                click_path.push(component);

                if i > 0 {
                    ui.label("/");
                }

                let comp_str = component.to_string_lossy();
                if ui.small_button(&*comp_str).clicked() {
                    navigate_target = Some(click_path.clone());
                }
            }

            if let Some(path) = navigate_target {
                state.navigate_to(path);
            }
        });
    });

    ui.separator();

    // Filter row
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.checkbox(&mut state.filter.show_images, "Images");
        ui.checkbox(&mut state.filter.show_json, "JSON");
        ui.checkbox(&mut state.filter.show_audio, "Audio");
        ui.checkbox(&mut state.filter.show_folders, "Folders");

        ui.separator();

        ui.label("Ext:");
        let ext_response = ui.add(
            egui::TextEdit::singleline(&mut state.filter.custom_extensions)
                .desired_width(80.0)
                .hint_text("e.g. .txt,.md"),
        );
        if ext_response.changed() {
            state.invalidate_cache();
        }

        ui.separator();

        ui.label("\u{1F50D}");
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut state.filter.search_text)
                .desired_width(120.0)
                .hint_text("Search..."),
        );
        if search_response.changed() {
            // Search doesn't need cache invalidation, just re-filter
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // View mode toggle
            ui.selectable_value(&mut state.view_mode, ViewMode::List, "\u{2630}");
            ui.selectable_value(&mut state.view_mode, ViewMode::Grid, "\u{25A6}");

            // Sort order
            egui::ComboBox::from_id_salt("sort_order")
                .selected_text(match state.sort_order {
                    SortOrder::Name => "Name",
                    SortOrder::Date => "Date",
                    SortOrder::Size => "Size",
                    SortOrder::Type => "Type",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.sort_order, SortOrder::Name, "Name");
                    ui.selectable_value(&mut state.sort_order, SortOrder::Date, "Date");
                    ui.selectable_value(&mut state.sort_order, SortOrder::Size, "Size");
                    ui.selectable_value(&mut state.sort_order, SortOrder::Type, "Type");
                });
        });
    });

    ui.separator();

    // File listing area
    let entries = state.get_entries();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| match state.view_mode {
            ViewMode::Grid => render_grid_view(ui, &entries, state, &mut result),
            ViewMode::List => render_list_view(ui, &entries, state, &mut result),
        });

    result
}

/// Render files in grid view
fn render_grid_view(
    ui: &mut egui::Ui,
    entries: &[FileEntry],
    state: &mut AssetBrowserState,
    result: &mut AssetBrowserResult,
) {
    let available_width = ui.available_width();
    let item_size = state.thumbnail_size + 16.0; // padding
    let columns = ((available_width / item_size) as usize).max(1);

    egui::Grid::new("asset_grid")
        .num_columns(columns)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (i, entry) in entries.iter().enumerate() {
                let is_selected = state.selected_file.as_ref() == Some(&entry.path);

                let response = ui.allocate_ui(
                    egui::vec2(state.thumbnail_size, state.thumbnail_size + 20.0),
                    |ui| {
                        ui.vertical_centered(|ui| {
                            // Icon/thumbnail
                            let icon_size = state.thumbnail_size - 16.0;
                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(icon_size, icon_size),
                                egui::Sense::hover(),
                            );

                            // Draw background for selection
                            if is_selected {
                                ui.painter().rect_filled(
                                    rect.expand(4.0),
                                    4.0,
                                    ui.visuals().selection.bg_fill,
                                );
                            }

                            // Draw icon
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                entry.file_type_icon(),
                                egui::FontId::proportional(icon_size * 0.6),
                                ui.visuals().text_color(),
                            );

                            // Filename (truncated)
                            let max_chars = (state.thumbnail_size / 7.0) as usize;
                            let display_name = if entry.name.len() > max_chars {
                                format!("{}...", &entry.name[..max_chars.saturating_sub(3)])
                            } else {
                                entry.name.clone()
                            };
                            ui.small(&display_name);
                        });
                    },
                );

                // Handle clicks
                let response = response.response.interact(egui::Sense::click());
                if response.clicked() {
                    state.selected_file = Some(entry.path.clone());
                }
                if response.double_clicked() {
                    if entry.is_dir {
                        state.navigate_to(entry.path.clone());
                    } else {
                        result.file_activated = Some(entry.path.clone());
                    }
                }

                // Context menu
                response.context_menu(|ui| {
                    render_context_menu(ui, entry, state, result);
                });

                // New row after N columns
                if (i + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
}

/// Render files in list view
fn render_list_view(
    ui: &mut egui::Ui,
    entries: &[FileEntry],
    state: &mut AssetBrowserState,
    result: &mut AssetBrowserResult,
) {
    for entry in entries {
        let is_selected = state.selected_file.as_ref() == Some(&entry.path);
        let size_str = if entry.is_dir {
            String::new()
        } else {
            format_size(entry.size)
        };

        let response = ui.selectable_label(
            is_selected,
            format!("{} {} {}", entry.file_type_icon(), entry.name, size_str),
        );

        if response.clicked() {
            state.selected_file = Some(entry.path.clone());
        }
        if response.double_clicked() {
            if entry.is_dir {
                state.navigate_to(entry.path.clone());
            } else {
                result.file_activated = Some(entry.path.clone());
            }
        }

        response.context_menu(|ui| {
            render_context_menu(ui, entry, state, result);
        });
    }
}

/// Render context menu for a file entry
fn render_context_menu(
    ui: &mut egui::Ui,
    entry: &FileEntry,
    _state: &mut AssetBrowserState,
    result: &mut AssetBrowserResult,
) {
    if entry.is_dir {
        if ui.button("Open Folder").clicked() {
            result.file_activated = Some(entry.path.clone());
            ui.close();
        }
    } else {
        if entry.is_image() {
            if ui.button("Add as Tileset Image").clicked() {
                // TODO: Implement
                ui.close();
            }
            if ui.button("Add as Sprite Sheet").clicked() {
                // TODO: Implement
                ui.close();
            }
        }
        if ui.button("Open").clicked() {
            result.file_activated = Some(entry.path.clone());
            ui.close();
        }
    }

    ui.separator();

    if ui.button("Copy Path").clicked() {
        ui.ctx().copy_text(entry.path.to_string_lossy().to_string());
        ui.close();
    }

    #[cfg(target_os = "windows")]
    if ui.button("Show in Explorer").clicked() {
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&entry.path)
            .spawn();
        ui.close();
    }
}

/// Format file size for display
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
