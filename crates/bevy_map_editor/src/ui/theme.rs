//! Editor theme and styling for egui
//!
//! Provides a modern, flat UI theme based on the official Bevy Feathers design.
//! Colors are derived from OKLCH values in bevy_feathers for perceptual uniformity.

use bevy_egui::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// Track if fonts have already been configured (only need to do once)
static FONTS_CONFIGURED: AtomicBool = AtomicBool::new(false);

/// Bevy-inspired dark theme colors and styling
/// Colors derived from official Bevy Feathers OKLCH palette (converted to sRGB)
pub struct EditorTheme;

impl EditorTheme {
    // -------------------------------------------------------------------------------
    // Background Colors (from Bevy Feathers OKLCH palette)
    // -------------------------------------------------------------------------------

    /// Main window/tabs background - GRAY_0 oklcha(0.2414, 0.0095, 285.67)
    pub const BG_WINDOW: Color32 = Color32::from_rgb(40, 41, 47);

    /// Panel content background - GRAY_1 oklcha(0.2866, 0.0072, 285.93)
    pub const BG_PANEL: Color32 = Color32::from_rgb(52, 54, 60);

    /// Widget/button background - GRAY_2 oklcha(0.3373, 0.0071, 274.77)
    pub const BG_WIDGET: Color32 = Color32::from_rgb(67, 68, 75);

    /// Selected tab/elevated widget background - GRAY_3 oklcha(0.3992, 0.0101, 278.38)
    pub const BG_SELECTED_TAB: Color32 = Color32::from_rgb(85, 86, 94);

    /// Row hover/highlight background (between GRAY_2 and GRAY_3)
    pub const BG_ROW: Color32 = Color32::from_rgb(76, 77, 84);

    // -------------------------------------------------------------------------------
    // Border Colors (from Bevy Feathers)
    // -------------------------------------------------------------------------------

    /// Main border - WARM_GRAY_1 oklcha(0.3757, 0.0017, 286.32)
    pub const BORDER_MAIN: Color32 = Color32::from_rgb(75, 76, 80);

    /// Widget/panel border (slightly lighter)
    pub const BORDER_WIDGET: Color32 = Color32::from_rgb(85, 86, 90);

    /// Side panel border
    #[allow(dead_code)]
    pub const BORDER_PANEL: Color32 = Color32::from_rgb(70, 71, 75);

    /// Tree indent line
    #[allow(dead_code)]
    pub const BORDER_INDENT: Color32 = Color32::from_rgb(80, 81, 85);

    // -------------------------------------------------------------------------------
    // Accent/Selection Colors (from Bevy Feathers)
    // -------------------------------------------------------------------------------

    /// Primary accent blue - ACCENT oklcha(0.542, 0.1594, 255.4)
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(45, 130, 209);

    /// Selected tree item background (subtle blue tint)
    pub const SELECTION_BG: Color32 = Color32::from_rgb(50, 70, 100);

    /// Yellow accent (for warnings/highlights)
    #[allow(dead_code)]
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 202, 57);

    // -------------------------------------------------------------------------------
    // Text Colors (from Bevy Feathers)
    // -------------------------------------------------------------------------------

    /// Brightest text - LIGHT_GRAY_1 oklcha(0.7607, 0.0014, 286.37)
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(191, 191, 193);

    /// Primary text
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(185, 185, 187);

    /// Standard text
    #[allow(dead_code)]
    pub const TEXT_STANDARD: Color32 = Color32::from_rgb(175, 175, 177);

    /// Field labels
    #[allow(dead_code)]
    pub const TEXT_LABEL: Color32 = Color32::from_rgb(170, 170, 172);

    /// Filter/input text
    #[allow(dead_code)]
    pub const TEXT_INPUT: Color32 = Color32::from_rgb(165, 165, 167);

    /// Input values
    #[allow(dead_code)]
    pub const TEXT_VALUE: Color32 = Color32::from_rgb(155, 155, 157);

    /// Icon strokes
    pub const TEXT_ICON: Color32 = Color32::from_rgb(160, 160, 162);

    /// Inactive/muted text - LIGHT_GRAY_2 oklcha(0.6106, 0.003, 286.31)
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(147, 148, 150);

    /// Very muted text
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(120, 120, 122);

    /// Active tab text
    #[allow(dead_code)]
    pub const TEXT_ACTIVE: Color32 = Color32::from_rgb(200, 200, 202);

    /// Pure white for selected/active elements - maximum contrast
    pub const TEXT_WHITE: Color32 = Color32::WHITE;

    // -------------------------------------------------------------------------------
    // Semantic Colors (from Bevy Feathers X/Y/Z axis colors)
    // -------------------------------------------------------------------------------

    /// Error/X-axis - X_AXIS oklcha(0.5232, 0.1404, 13.84) - coral/red
    pub const ERROR: Color32 = Color32::from_rgb(156, 82, 92);

    /// Success/Y-axis - Y_AXIS oklcha(0.5866, 0.1543, 129.84) - olive/green
    #[allow(dead_code)]
    pub const SUCCESS: Color32 = Color32::from_rgb(119, 148, 65);

    /// Info/Z-axis - Z_AXIS oklcha(0.4847, 0.1249, 253.08) - steel blue
    #[allow(dead_code)]
    pub const INFO: Color32 = Color32::from_rgb(72, 113, 161);

    /// Warning color
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

        // Flat design - no shadows per Bevy Feathers style
        visuals.popup_shadow = egui::Shadow::NONE;
        visuals.window_shadow = egui::Shadow::NONE;

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

        // Configure fonts (only once per session)
        Self::configure_fonts(ctx);
    }

    /// Configure fonts for a clean, monospace-first UI aesthetic.
    ///
    /// Uses egui's built-in monospace font (Hack) as the primary font
    /// for a consistent, code-editor-like appearance matching Bevy's style.
    fn configure_fonts(ctx: &egui::Context) {
        // Only configure fonts once
        if FONTS_CONFIGURED.swap(true, Ordering::SeqCst) {
            return;
        }

        // Configure text styles to use appropriate font sizes
        // Monospace gives a clean, technical aesthetic that matches Bevy's editor
        let mut style = (*ctx.style()).clone();

        // Use monospace for body text (gives the "code editor" feel)
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Monospace));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::new(11.0, FontFamily::Monospace));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new(13.0, FontFamily::Monospace));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(16.0, FontFamily::Monospace));
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        );

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
            shadow: egui::Shadow::NONE,
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
