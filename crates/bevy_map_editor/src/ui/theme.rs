//! Editor theme and styling for egui
//!
//! Provides a modern, flat UI theme based on the Bevy Editor Figma designs.
//! Colors extracted directly from Figma CSS export.

use bevy_egui::egui::{self, Color32, CornerRadius, Stroke, Visuals};

/// Bevy-editor inspired dark theme colors and styling
/// All colors are extracted from the official Bevy Editor Figma CSS export
pub struct EditorTheme;

impl EditorTheme {
    // -------------------------------------------------------------------------------════════════
    // Background Colors (from Figma CSS)
    // -------------------------------------------------------------------------------════════════

    /// Main window/tabs background - #1F1F24
    pub const BG_WINDOW: Color32 = Color32::from_rgb(31, 31, 36);

    /// Panel content background - #2A2A2E
    pub const BG_PANEL: Color32 = Color32::from_rgb(42, 42, 46);

    /// Widget/button background - #36373B
    pub const BG_WIDGET: Color32 = Color32::from_rgb(54, 55, 59);

    /// Selected tab/elevated widget background - #46474C
    pub const BG_SELECTED_TAB: Color32 = Color32::from_rgb(70, 71, 76);

    /// Row hover/highlight background - #404040
    pub const BG_ROW: Color32 = Color32::from_rgb(64, 64, 64);

    // -------------------------------------------------------------------------------════════════
    // Border Colors (from Figma CSS)
    // -------------------------------------------------------------------------------════════════

    /// Main border - #303030
    pub const BORDER_MAIN: Color32 = Color32::from_rgb(48, 48, 48);

    /// Widget/panel border - #414142
    pub const BORDER_WIDGET: Color32 = Color32::from_rgb(65, 65, 66);

    /// Side panel border - #373737
    #[allow(dead_code)]
    pub const BORDER_PANEL: Color32 = Color32::from_rgb(55, 55, 55);

    /// Tree indent line - #4A4A4A
    #[allow(dead_code)]
    pub const BORDER_INDENT: Color32 = Color32::from_rgb(74, 74, 74);

    // -------------------------------------------------------------------------------════════════
    // Accent/Selection Colors (from Figma CSS)
    // -------------------------------------------------------------------------------════════════

    /// Primary accent blue - #206EC9 (tab borders, selection, active buttons)
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(32, 110, 201);

    /// Selected tree item background - #273E5D (subtle blue tint)
    pub const SELECTION_BG: Color32 = Color32::from_rgb(39, 62, 93);

    /// Yellow accent - #FFCA39
    #[allow(dead_code)]
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 202, 57);

    // -------------------------------------------------------------------------------════════════
    // Text Colors (from Figma CSS)
    // -------------------------------------------------------------------------------════════════

    /// Brightest text - #ECECEC
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(236, 236, 236);

    /// Primary text - #E6E6E6
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 230);

    /// Standard text - #DDDDDD
    #[allow(dead_code)]
    pub const TEXT_STANDARD: Color32 = Color32::from_rgb(221, 221, 221);

    /// Field labels - #DCDCDC
    #[allow(dead_code)]
    pub const TEXT_LABEL: Color32 = Color32::from_rgb(220, 220, 220);

    /// Filter/input text - #DADADA
    #[allow(dead_code)]
    pub const TEXT_INPUT: Color32 = Color32::from_rgb(218, 218, 218);

    /// Input values - #C2C2C2
    #[allow(dead_code)]
    pub const TEXT_VALUE: Color32 = Color32::from_rgb(194, 194, 194);

    /// Icon strokes - #C4C4C4
    pub const TEXT_ICON: Color32 = Color32::from_rgb(196, 196, 196);

    /// Inactive/muted text - #A8A8A8
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(168, 168, 168);

    /// Very muted text - #838385
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(131, 131, 133);

    /// Active tab text - #F2F2F2
    #[allow(dead_code)]
    pub const TEXT_ACTIVE: Color32 = Color32::from_rgb(242, 242, 242);

    /// Pure white for selected/active elements - maximum contrast
    pub const TEXT_WHITE: Color32 = Color32::WHITE;

    // -------------------------------------------------------------------------------════════════
    // Semantic Colors (from Figma CSS - axis indicators)
    // -------------------------------------------------------------------------------════════════

    /// Error/X-axis - #AB4051
    pub const ERROR: Color32 = Color32::from_rgb(171, 64, 81);

    /// Success/Y-axis - #5D8D0A
    #[allow(dead_code)]
    pub const SUCCESS: Color32 = Color32::from_rgb(93, 141, 10);

    /// Info/Z-axis - #2160A3
    #[allow(dead_code)]
    pub const INFO: Color32 = Color32::from_rgb(33, 96, 163);

    /// Warning color (derived)
    pub const WARNING: Color32 = Color32::from_rgb(200, 160, 60);

    // -------------------------------------------------------------------------------════════════
    // Theme Application
    // -------------------------------------------------------------------------------════════════

    /// Apply the editor theme to the egui context.
    pub fn apply(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let mut visuals = Visuals::dark();

        // ───────────────────────────────────────────────────────────────────────
        // Window and Panel fills (from Figma)
        // ───────────────────────────────────────────────────────────────────────
        visuals.window_fill = Self::BG_WINDOW;
        visuals.panel_fill = Self::BG_PANEL;
        visuals.faint_bg_color = Self::BG_WINDOW;
        visuals.extreme_bg_color = Self::BG_WINDOW;

        // Subtle shadows
        visuals.popup_shadow = egui::Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(80),
        };

        visuals.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(60),
        };

        // ───────────────────────────────────────────────────────────────────────
        // Widget styling - Non-interactive (labels, static elements)
        // ───────────────────────────────────────────────────────────────────────
        visuals.widgets.noninteractive.bg_fill = Self::BG_WIDGET;
        visuals.widgets.noninteractive.weak_bg_fill = Self::BG_PANEL;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_MUTED);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER_WIDGET);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Widget styling - Inactive (interactive but not hovered)
        // ───────────────────────────────────────────────────────────────────────
        visuals.widgets.inactive.bg_fill = Self::BG_WIDGET;
        visuals.widgets.inactive.weak_bg_fill = Self::BG_PANEL;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER_WIDGET);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Widget styling - Hovered
        // ───────────────────────────────────────────────────────────────────────
        visuals.widgets.hovered.bg_fill = Self::BG_SELECTED_TAB;
        visuals.widgets.hovered.weak_bg_fill = Self::BG_ROW;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::TEXT_BRIGHT);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::BORDER_WIDGET);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Widget styling - Active (being clicked/dragged)
        // Solid accent blue background per Figma (background: #206EC9)
        // Pure white text for maximum readability over blue
        // ───────────────────────────────────────────────────────────────────────
        visuals.widgets.active.bg_fill = Self::ACCENT_BLUE;
        visuals.widgets.active.weak_bg_fill = Self::ACCENT_BLUE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5, Self::TEXT_WHITE);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Self::BORDER_WIDGET);
        visuals.widgets.active.corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Widget styling - Open (dropdown open, etc.)
        // Pure white text for readability
        // ───────────────────────────────────────────────────────────────────────
        visuals.widgets.open.bg_fill = Self::BG_SELECTED_TAB;
        visuals.widgets.open.weak_bg_fill = Self::BG_ROW;
        visuals.widgets.open.fg_stroke = Stroke::new(1.5, Self::TEXT_WHITE);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, Self::ACCENT_BLUE);
        visuals.widgets.open.corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Selection (text selection, selected buttons, etc.)
        // Solid accent blue per Figma (background: #206EC9), white text
        // ───────────────────────────────────────────────────────────────────────
        visuals.selection.bg_fill = Self::ACCENT_BLUE;
        visuals.selection.stroke = Stroke::new(1.0, Self::TEXT_WHITE);

        // ───────────────────────────────────────────────────────────────────────
        // Window/menu rounding (from Figma: 6-8px for panels, 4-5px for items)
        // ───────────────────────────────────────────────────────────────────────
        visuals.window_corner_radius = CornerRadius::same(6);
        visuals.menu_corner_radius = CornerRadius::same(5);

        // ───────────────────────────────────────────────────────────────────────
        // Misc colors
        // ───────────────────────────────────────────────────────────────────────
        visuals.hyperlink_color = Self::ACCENT_BLUE;
        visuals.warn_fg_color = Self::WARNING;
        visuals.error_fg_color = Self::ERROR;

        // Striped backgrounds (for tables, lists)
        visuals.striped = true;

        // ───────────────────────────────────────────────────────────────────────
        // Spacing (tighter to match Figma compact layout)
        // ───────────────────────────────────────────────────────────────────────
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
        style.spacing.indent = 16.0;
        style.spacing.interact_size = egui::vec2(40.0, 20.0);
        style.spacing.slider_width = 100.0;
        style.spacing.combo_width = 120.0;
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 8.0,
            handle_min_length: 20.0,
            bar_inner_margin: 2.0,
            bar_outer_margin: 2.0,
            ..Default::default()
        };

        // ───────────────────────────────────────────────────────────────────────
        // Apply
        // ───────────────────────────────────────────────────────────────────────
        style.visuals = visuals;
        ctx.set_style(style);
    }

    // -------------------------------------------------------------------------------════════════
    // Helper Methods
    // -------------------------------------------------------------------------------════════════

    /// Get a frame style for side panels (matches Figma panel styling)
    #[allow(dead_code)]
    pub fn panel_frame() -> egui::Frame {
        egui::Frame {
            fill: Self::BG_PANEL,
            inner_margin: egui::Margin::same(6),
            outer_margin: egui::Margin::ZERO,
            stroke: Stroke::new(1.0, Self::BORDER_MAIN),
            corner_radius: CornerRadius::same(6),
            shadow: egui::Shadow::NONE,
        }
    }

    /// Get a frame style for floating windows
    #[allow(dead_code)]
    pub fn window_frame() -> egui::Frame {
        egui::Frame {
            fill: Self::BG_WINDOW,
            inner_margin: egui::Margin::same(8),
            outer_margin: egui::Margin::same(4),
            stroke: Stroke::new(1.0, Self::BORDER_WIDGET),
            corner_radius: CornerRadius::same(8),
            shadow: egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(60),
            },
        }
    }

    /// Get a frame style for toolbars (top bars in Figma)
    #[allow(dead_code)]
    pub fn toolbar_frame() -> egui::Frame {
        egui::Frame {
            fill: Self::BG_WINDOW,
            inner_margin: egui::Margin::symmetric(8, 4),
            outer_margin: egui::Margin::ZERO,
            stroke: Stroke::new(1.0, Self::BORDER_MAIN),
            corner_radius: CornerRadius::ZERO,
            shadow: egui::Shadow::NONE,
        }
    }

    /// Get a frame style for collapsible component headers (like Transform in Figma)
    #[allow(dead_code)]
    pub fn component_header_frame() -> egui::Frame {
        egui::Frame {
            fill: Self::BG_WIDGET,
            inner_margin: egui::Margin::symmetric(8, 5),
            outer_margin: egui::Margin::ZERO,
            stroke: Stroke::NONE,
            corner_radius: CornerRadius {
                nw: 5,
                ne: 5,
                sw: 0,
                se: 0,
            },
            shadow: egui::Shadow::NONE,
        }
    }
}
