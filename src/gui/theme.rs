//! Sleek minimalist visual styling and clean dark palette for eframe/egui.
//!
//! Crafted for ROG Strix aesthetics:
//! Obsidian stealth background, slate card panels, high-contrast neon accents,
//! rounded corners, and glowing hover states.

use eframe::egui::{Color32, Context, Rounding, Stroke, Visuals};

// Minimalist Dark Palette Constants
pub const COLOR_BG_OBSIDIAN: Color32 = Color32::from_rgb(13, 14, 21); // #0D0E15 (Canvas)
pub const COLOR_PANEL_CARD: Color32 = Color32::from_rgb(24, 26, 36); // #181A24 (Card Panels)
pub const COLOR_BORDER_CARD: Color32 = Color32::from_rgb(42, 45, 61); // #2A2D3D (Subtle Borders)
pub const COLOR_NEON_CYAN: Color32 = Color32::from_rgb(0, 210, 255); // #00D2FF (Primary Accent)
pub const COLOR_CYBER_MAGENTA: Color32 = Color32::from_rgb(255, 0, 127); // #FF007F (Secondary Accent)
pub const COLOR_MATRIX_GREEN: Color32 = Color32::from_rgb(0, 230, 118); // #00E676 (Active / Success)
pub const COLOR_VOLCANO_ORANGE: Color32 = Color32::from_rgb(255, 119, 0); // #FF7700 (Warning / Heat)
pub const COLOR_CRIMSON_RED: Color32 = Color32::from_rgb(255, 51, 102); // #FF3366 (Stop / Error)
pub const COLOR_TEXT_ICE: Color32 = Color32::from_rgb(240, 242, 250); // #F0F2FA (Crisp Ice White)
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(138, 142, 158); // #8A8E9E (Muted Description)

/// Configures the egui Context with the sleek minimalist dark theme.
pub fn apply_minimalist_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();

    // Backgrounds
    visuals.panel_fill = COLOR_BG_OBSIDIAN;
    visuals.window_fill = COLOR_BG_OBSIDIAN;
    visuals.faint_bg_color = COLOR_PANEL_CARD;
    visuals.extreme_bg_color = Color32::from_rgb(10, 11, 16);

    // Window borders and corners
    visuals.window_rounding = Rounding::same(6.0);
    visuals.window_stroke = Stroke::new(1.0_f32, COLOR_BORDER_CARD);

    // Non-interactive containers / frames
    visuals.widgets.noninteractive.bg_fill = COLOR_PANEL_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER_CARD);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_ICE);
    visuals.widgets.noninteractive.rounding = Rounding::same(5.0);

    // Inactive buttons and controls
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 30, 44);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER_CARD);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_ICE);
    visuals.widgets.inactive.rounding = Rounding::same(5.0);

    // Hovered buttons and controls: Neon Cyan stroke outline
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(36, 40, 60);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, COLOR_NEON_CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(5.0);

    // Active (clicked/dragged) buttons and controls
    visuals.widgets.active.bg_fill = Color32::from_rgb(0, 180, 220);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, COLOR_NEON_CYAN);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::BLACK);
    visuals.widgets.active.rounding = Rounding::same(5.0);

    // Open menus / dropdowns
    visuals.widgets.open.bg_fill = COLOR_PANEL_CARD;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, COLOR_NEON_CYAN);
    visuals.widgets.open.rounding = Rounding::same(5.0);

    // Selection highlight
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(0, 210, 255, 60);
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_NEON_CYAN);

    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_color_constants() {
        assert_eq!(COLOR_BG_OBSIDIAN, Color32::from_rgb(13, 14, 21));
        assert_eq!(COLOR_PANEL_CARD, Color32::from_rgb(24, 26, 36));
        assert_eq!(COLOR_BORDER_CARD, Color32::from_rgb(42, 45, 61));
        assert_eq!(COLOR_NEON_CYAN, Color32::from_rgb(0, 210, 255));
        assert_eq!(COLOR_CYBER_MAGENTA, Color32::from_rgb(255, 0, 127));
        assert_eq!(COLOR_MATRIX_GREEN, Color32::from_rgb(0, 230, 118));
        assert_eq!(COLOR_TEXT_ICE, Color32::from_rgb(240, 242, 250));
        assert_eq!(COLOR_TEXT_MUTED, Color32::from_rgb(138, 142, 158));
    }
}
