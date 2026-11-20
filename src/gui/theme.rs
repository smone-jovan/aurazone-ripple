//! LenovoRGB-inspired cool-obsidian visual design system for eframe/egui.
//!
//! Crafted for precision desktop RGB control:
//! Deep obsidian base canvas, cool slate surfaces, crisp 1px borders,
//! cyan primary accents, and color-tinted status/monogram surfaces.

use eframe::egui::{Color32, Context, Rounding, Stroke, Vec2, Visuals};

// Core Canvas & Surface Tokens
pub const BG_BASE: Color32 = Color32::from_rgb(8, 10, 15); // #080A0F
pub const BG_ELEVATED: Color32 = Color32::from_rgb(12, 16, 23); // #0C1017
pub const SURFACE: Color32 = Color32::from_rgb(17, 23, 37); // #111725
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(22, 31, 48); // #161F30
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(26, 36, 56); // #1A2438
pub const SURFACE_FIELD: Color32 = Color32::from_rgb(14, 20, 32); // #0E1420

// Borders
pub const BORDER: Color32 = Color32::from_rgb(32, 43, 63); // #202B3F
pub const BORDER_HOVER: Color32 = Color32::from_rgb(48, 64, 92); // #30405C

// Typography
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(239, 244, 252); // #EFF4FC
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(166, 179, 201); // #A6B3C9
pub const TEXT_MUTED: Color32 = Color32::from_rgb(120, 134, 159); // #78869F

// Accents & Semantic States
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(34, 211, 238); // #22D3EE
pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(139, 92, 246); // #8B5CF6
pub const ACCENT_MAGENTA: Color32 = Color32::from_rgb(244, 114, 182); // #F472B6
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(251, 146, 60); // #FB923C
pub const SUCCESS: Color32 = Color32::from_rgb(52, 211, 153); // #34D399
pub const WARNING: Color32 = Color32::from_rgb(251, 191, 36); // #FBBF24
pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113); // #F87171

/// Linear RGB blend from `a` towards `b` by normalized factor `t` in `[0.0, 1.0]`.
#[inline]
pub fn blend_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgb(
        (a.r() as f32 * inv + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * inv + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * inv + b.b() as f32 * t).round() as u8,
    )
}

/// Applies the LenovoRGB-inspired dark theme and spacing rhythm to `egui::Context`.
pub fn apply_minimalist_theme(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.handle_min_length = 28.0;

    let mut visuals = Visuals::dark();

    // Backgrounds
    visuals.panel_fill = BG_BASE;
    visuals.window_fill = BG_BASE;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.extreme_bg_color = SURFACE_FIELD;

    // Window & containers
    visuals.window_rounding = Rounding::same(6.0);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);

    // Non-interactive surfaces
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    // Default interactive controls
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    // Hovered controls
    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_HOVER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    // Active / pressed controls
    visuals.widgets.active.bg_fill = SURFACE_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, ACCENT_CYAN);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    // Open menus / popups
    visuals.widgets.open.bg_fill = SURFACE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, ACCENT_CYAN);
    visuals.widgets.open.rounding = Rounding::same(6.0);

    // Selection highlight
    visuals.selection.bg_fill = blend_color(ACCENT_CYAN, BG_BASE, 0.72);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_CYAN);

    style.visuals = visuals;
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_color_constants() {
        assert_eq!(BG_BASE, Color32::from_rgb(8, 10, 15));
        assert_eq!(BG_ELEVATED, Color32::from_rgb(12, 16, 23));
        assert_eq!(SURFACE, Color32::from_rgb(17, 23, 37));
        assert_eq!(BORDER, Color32::from_rgb(32, 43, 63));
        assert_eq!(ACCENT_CYAN, Color32::from_rgb(34, 211, 238));
        assert_eq!(TEXT_PRIMARY, Color32::from_rgb(239, 244, 252));
        assert_eq!(SUCCESS, Color32::from_rgb(52, 211, 153));
    }

    #[test]
    fn test_blend_color() {
        assert_eq!(blend_color(Color32::BLACK, Color32::WHITE, 0.0), Color32::BLACK);
        assert_eq!(blend_color(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
        let mid = blend_color(Color32::from_rgb(0, 0, 0), Color32::from_rgb(100, 200, 50), 0.5);
        assert_eq!(mid, Color32::from_rgb(50, 100, 25));
    }
}
