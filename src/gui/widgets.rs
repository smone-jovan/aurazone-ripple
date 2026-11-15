//! Custom visualizer and control widgets for the Cyberpunk-themed GUI.
//!
//! Includes:
//! 1. Procedural 4-Zone Keyboard Deck Visualizer (Zones 0..3)
//! 2. Wraparound Under-Chassis Lightbar Visualizer (Zones 4..7)
//! 3. Interactive Parameter Controls (Speed, Duration, Dimming, Cascade, Colors, Rainbow)
//! 4. 6-Preset Quick Switcher Badges with Cyberpunk styling

use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};

use crate::config::{Preset, RippleConfig};
use crate::gui::theme::*;

/// Unpacks a `0x00RRGGBB` encoded u32 into an `egui::Color32`.
#[inline]
pub fn u32_to_color32(val: u32) -> Color32 {
    let r = ((val >> 16) & 0xFF) as u8;
    let g = ((val >> 8) & 0xFF) as u8;
    let b = (val & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}

/// Renders the procedural 4-zone keyboard deck (Zones 0..3: WASD, Mid-Left, Mid-Right, Numpad).
/// Returns Some(zone_index) if a zone was clicked.
pub fn render_keyboard_deck(ui: &mut Ui, preview_zones: &[u32; 8]) -> Option<usize> {
    let mut clicked_zone = None;
    let available_width = ui.available_width().max(300.0);
    let deck_height = 80.0;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(available_width, deck_height), Sense::hover());
    let rect = response.rect;

    // Deck backdrop container
    painter.rect_filled(rect, Rounding::same(6.0), COLOR_PANEL_CARD);
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, COLOR_BORDER_CARD));

    let pad = 6.0;
    let inner_width = rect.width() - (pad * 2.0);
    let inner_height = rect.height() - (pad * 2.0);
    let gap = 6.0;
    let zone_width = (inner_width - (gap * 3.0)) / 4.0;

    let zone_labels = [
        ("ZONE 0", "WASD / QWER"),
        ("ZONE 1", "MID-LEFT"),
        ("ZONE 2", "MID-RIGHT"),
        ("ZONE 3", "NUMPAD"),
    ];

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let is_clicked = ui.input(|i| i.pointer.primary_clicked());

    for i in 0..4 {
        let x0 = rect.left() + pad + (i as f32 * (zone_width + gap));
        let y0 = rect.top() + pad;
        let zone_rect = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(zone_width, inner_height));

        let is_hovered = pointer_pos.map_or(false, |pos| zone_rect.contains(pos));
        if is_hovered && is_clicked {
            clicked_zone = Some(i);
        }

        let zone_color = u32_to_color32(preview_zones[i]);

        // Zone filled block
        painter.rect_filled(zone_rect, Rounding::same(4.0), zone_color);
        let border_stroke = if is_hovered {
            Stroke::new(2.0_f32, COLOR_NEON_CYAN)
        } else {
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 210, 255, 120))
        };
        painter.rect_stroke(zone_rect, Rounding::same(4.0), border_stroke);

        // Compute high-contrast text color based on luminance
        let r = ((preview_zones[i] >> 16) & 0xFF) as u8;
        let g = ((preview_zones[i] >> 8) & 0xFF) as u8;
        let b = (preview_zones[i] & 0xFF) as u8;
        let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
        let text_color = if lum > 140.0 {
            Color32::BLACK
        } else {
            COLOR_TEXT_ICE
        };
        let hex_color = if lum > 140.0 {
            Color32::from_rgb(30, 30, 30)
        } else {
            COLOR_NEON_CYAN
        };

        // Zone title & description
        painter.text(
            Pos2::new(zone_rect.center().x, zone_rect.top() + 14.0),
            Align2::CENTER_CENTER,
            zone_labels[i].0,
            FontId::proportional(11.0),
            text_color,
        );
        painter.text(
            Pos2::new(zone_rect.center().x, zone_rect.top() + 30.0),
            Align2::CENTER_CENTER,
            zone_labels[i].1,
            FontId::proportional(9.0),
            text_color,
        );

        // Hex readout at bottom
        let hex_str = format!("#{:02X}{:02X}{:02X}", r, g, b);
        painter.text(
            Pos2::new(zone_rect.center().x, zone_rect.bottom() - 14.0),
            Align2::CENTER_CENTER,
            hex_str,
            FontId::monospace(10.0),
            hex_color,
        );
    }

    clicked_zone
}

/// Renders the wraparound under-chassis lightbar visualizer (Zones 4..7).
pub fn render_lightbar_deck(ui: &mut Ui, preview_zones: &[u32; 8]) {
    let available_width = ui.available_width().max(300.0);
    let bar_height = 36.0;
    let (response, painter) =
        ui.allocate_painter(Vec2::new(available_width, bar_height), Sense::hover());
    let rect = response.rect;

    // Lightbar container
    painter.rect_filled(rect, Rounding::same(6.0), COLOR_PANEL_CARD);
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, COLOR_BORDER_CARD));

    let pad = 6.0;
    let inner_width = rect.width() - (pad * 2.0);
    let inner_height = rect.height() - (pad * 2.0);
    let gap = 5.0;
    let pill_width = (inner_width - (gap * 3.0)) / 4.0;

    let pill_labels = ["L-WRAP", "MID-L", "MID-R", "R-WRAP"];

    for i in 0..4 {
        let zone_idx = 4 + i;
        let x0 = rect.left() + pad + (i as f32 * (pill_width + gap));
        let y0 = rect.top() + pad;
        let pill_rect = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(pill_width, inner_height));

        let zone_color = u32_to_color32(preview_zones[zone_idx]);

        // Pill shape with smooth rounding
        painter.rect_filled(pill_rect, Rounding::same(6.0), zone_color);
        painter.rect_stroke(
            pill_rect,
            Rounding::same(6.0),
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 210, 255, 90)),
        );

        let r = ((preview_zones[zone_idx] >> 16) & 0xFF) as u8;
        let g = ((preview_zones[zone_idx] >> 8) & 0xFF) as u8;
        let b = (preview_zones[zone_idx] & 0xFF) as u8;
        let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
        let text_color = if lum > 140.0 {
            Color32::BLACK
        } else {
            COLOR_TEXT_ICE
        };

        painter.text(
            pill_rect.center(),
            Align2::CENTER_CENTER,
            format!("Z{} {}", zone_idx, pill_labels[i]),
            FontId::proportional(9.0),
            text_color,
        );
    }
}

/// Renders the 6-preset badge quick switcher.
/// Returns Some(Preset) if the user clicked one of the presets.
pub fn render_preset_switcher(ui: &mut Ui, current_preset: Preset) -> Option<Preset> {
    let mut selected_preset = None;

    let presets = [
        (Preset::Cyberpunk, "Cyberpunk", COLOR_NEON_CYAN),
        (Preset::Matrix, "Matrix", COLOR_MATRIX_GREEN),
        (Preset::Stealth, "Stealth", COLOR_TEXT_ICE),
        (Preset::Volcano, "Volcano", COLOR_VOLCANO_ORANGE),
        (Preset::Synthwave, "Synthwave", COLOR_CYBER_MAGENTA),
        (Preset::Rainbow, "Rainbow", COLOR_NEON_CYAN),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        for (preset, label, accent) in presets {
            let is_active = current_preset == preset;
            let (bg_color, stroke) = if is_active {
                (
                    Color32::from_rgb(32, 38, 56),
                    Stroke::new(1.5_f32, accent),
                )
            } else {
                (
                    Color32::from_rgb(20, 22, 30),
                    Stroke::new(1.0_f32, COLOR_BORDER_CARD),
                )
            };

            let button = egui::Button::new(
                egui::RichText::new(label)
                    .color(if is_active { accent } else { COLOR_TEXT_MUTED })
                    .strong(),
            )
            .fill(bg_color)
            .stroke(stroke)
            .rounding(Rounding::same(5.0))
            .min_size(Vec2::new(76.0, 26.0));

            if ui.add(button).clicked() {
                selected_preset = Some(preset);
            }
        }
    });

    selected_preset
}

/// Renders the interactive parameter control sliders and pickers.
/// Returns true if any parameter was modified.
pub fn render_parameter_controls(
    ui: &mut Ui,
    config: &mut RippleConfig,
    wave_speed_ms: &mut u32,
    ripple_duration_ms: &mut u32,
    idle_dim_pct: &mut u32,
    hue_increment_deg: &mut u32,
) -> bool {
    let mut changed = false;

    // Wave Speed Slider (30 ms to 150 ms/zone)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("⚡ Wave Speed:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        let speed_slider = egui::Slider::new(wave_speed_ms, 30..=150)
            .suffix(" ms/zone");
        if ui.add(speed_slider).changed() {
            // Speed factor inversely proportional: speed = 65.0 / wave_speed_ms
            config.speed = 65.0 / (*wave_speed_ms as f32).max(1.0);
            changed = true;
        }
    });

    // Ripple Duration Slider (150 ms to 800 ms)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("⏱ Ripple Duration:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        let duration_slider = egui::Slider::new(ripple_duration_ms, 150..=800)
            .suffix(" ms");
        if ui.add(duration_slider).changed() {
            // Map duration to attack / decay envelope
            let dur = *ripple_duration_ms as f32;
            config.decay = (dur / 500.0).clamp(0.4, 0.95);
            changed = true;
        }
    });

    // Idle Brightness / Dimming Slider (0% to 100%)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🌙 Idle Dimming:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        let dim_slider = egui::Slider::new(idle_dim_pct, 0..=100)
            .suffix("%");
        if ui.add(dim_slider).changed() {
            if *idle_dim_pct == 0 {
                config.color_base = [0, 0, 0];
            }
            changed = true;
        }
    });

    // Front Lightbar Cascade Delay Slider (10 ms to 100 ms)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🌊 Lightbar Cascade:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        let mut cascade_val = config.lightbar_delay_ms as u32;
        let cascade_slider = egui::Slider::new(&mut cascade_val, 10..=100)
            .suffix(" ms");
        if ui.add(cascade_slider).changed() {
            config.lightbar_delay_ms = cascade_val as u64;
            changed = true;
        }
    });

    ui.add_space(4.0);

    // Color Pickers
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🎨 Base Color:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        if ui.color_edit_button_srgb(&mut config.color_base).changed() {
            changed = true;
        }

        ui.add_space(16.0);

        ui.label(
            egui::RichText::new("✨ Ripple Color:")
                .color(COLOR_TEXT_ICE)
                .strong(),
        );
        if ui
            .color_edit_button_srgb(&mut config.color_ripple)
            .changed()
        {
            changed = true;
        }
    });

    ui.add_space(4.0);

    // Dynamic HSV Rainbow Mode Toggle & Hue Increment Slider
    ui.horizontal(|ui| {
        let rainbow_cb = egui::Checkbox::new(&mut config.rainbow, "🌈 Dynamic HSV Rainbow Mode");
        if ui.add(rainbow_cb).changed() {
            changed = true;
        }

        if config.rainbow {
            ui.add_space(8.0);
            let hue_slider = egui::Slider::new(hue_increment_deg, 5..=45)
                .suffix("° / key");
            if ui.add(hue_slider).changed() {
                changed = true;
            }
        }
    });

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_to_color32_unpacking() {
        let packed = 0x00FF0080u32; // R=255, G=0, B=128
        let c = u32_to_color32(packed);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 0);
        assert_eq!(c.b(), 128);

        let black = u32_to_color32(0x00000000);
        assert_eq!(black, Color32::from_rgb(0, 0, 0));

        let white = u32_to_color32(0x00FFFFFF);
        assert_eq!(white, Color32::from_rgb(255, 255, 255));
    }
}
