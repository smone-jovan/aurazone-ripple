//! Custom-painted LenovoRGB-inspired UI widgets for eframe/egui.
//!
//! Zero emojis — all marks, monogram badges, status pills, toggles,
//! brightness track, mode cards, and 4-zone live previews are drawn cleanly
//! via `egui::Painter`.

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Margin, Pos2, Rect, Rounding, Sense, Stroke, Ui,
    Vec2,
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

#[inline]
fn rgb_to_color32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// Metadata driving each LenovoRGB-style preset card.
#[derive(Clone, Debug)]
pub struct PresetCardMeta {
    pub preset: Preset,
    pub monogram: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub accent: Color32,
    pub base_swatch: Color32,
    pub ripple_swatch: Color32,
    pub is_custom: bool,
}

/// Builds card metadata for all 8 presets (live-reflecting custom Profile 1 & 2 swatches).
pub fn build_preset_cards(cfg: &RippleConfig) -> [PresetCardMeta; 8] {
    [
        PresetCardMeta {
            preset: Preset::Cyberpunk,
            monogram: "CY",
            title: "Cyberpunk",
            description: "Neon pink idle deck pulsed by sharp electric cyan waves.",
            accent: ACCENT_CYAN,
            base_swatch: Color32::from_rgb(255, 0, 128),
            ripple_swatch: Color32::from_rgb(0, 210, 255),
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Matrix,
            monogram: "MX",
            title: "Matrix",
            description: "Deep terminal phosphor base ignited by crisp green ripples.",
            accent: SUCCESS,
            base_swatch: Color32::from_rgb(0, 20, 0),
            ripple_swatch: Color32::from_rgb(0, 255, 65),
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Stealth,
            monogram: "ST",
            title: "Stealth",
            description: "Dark keys at rest with clean white bursts on every stroke.",
            accent: Color32::from_rgb(148, 163, 184),
            base_swatch: Color32::from_rgb(0, 0, 0),
            ripple_swatch: Color32::from_rgb(255, 255, 255),
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Volcano,
            monogram: "VL",
            title: "Volcano",
            description: "Smoldering crimson bed swept by molten orange shockwaves.",
            accent: ACCENT_ORANGE,
            base_swatch: Color32::from_rgb(40, 0, 0),
            ripple_swatch: Color32::from_rgb(255, 60, 0),
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Synthwave,
            monogram: "SW",
            title: "Synthwave",
            description: "Ultraviolet twilight base lit by vivid magenta wave crests.",
            accent: ACCENT_MAGENTA,
            base_swatch: Color32::from_rgb(30, 0, 60),
            ripple_swatch: Color32::from_rgb(255, 0, 128),
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Rainbow,
            monogram: "RB",
            title: "Rainbow Wave",
            description: "Full-spectrum HSV ripples advancing hue on each keypress.",
            accent: ACCENT_VIOLET,
            base_swatch: Color32::from_rgb(10, 10, 10),
            ripple_swatch: ACCENT_VIOLET,
            is_custom: false,
        },
        PresetCardMeta {
            preset: Preset::Profile1,
            monogram: "P1",
            title: "Profil 1",
            description: "Custom slot 1 saved physics tuning and dual-zone colors.",
            accent: ACCENT_CYAN,
            base_swatch: rgb_to_color32(cfg.profile_1.color_base),
            ripple_swatch: rgb_to_color32(cfg.profile_1.color_ripple),
            is_custom: true,
        },
        PresetCardMeta {
            preset: Preset::Profile2,
            monogram: "P2",
            title: "Profil 2",
            description: "Custom slot 2 saved physics tuning and dual-zone colors.",
            accent: WARNING,
            base_swatch: rgb_to_color32(cfg.profile_2.color_base),
            ripple_swatch: rgb_to_color32(cfg.profile_2.color_ripple),
            is_custom: true,
        },
    ]
}

/// Looks up title and description for the active preset in the sidebar "Now Playing" card.
pub fn preset_summary(preset: Preset) -> (&'static str, &'static str, Color32) {
    match preset {
        Preset::Cyberpunk => (
            "Cyberpunk",
            "Neon pink base with electric cyan shockwaves across all zones.",
            ACCENT_CYAN,
        ),
        Preset::Matrix => (
            "Matrix",
            "Terminal green phosphor waves rippling across the deck.",
            SUCCESS,
        ),
        Preset::Stealth => (
            "Stealth",
            "Zero idle light with crisp white bursts on keystroke.",
            Color32::from_rgb(148, 163, 184),
        ),
        Preset::Volcano => (
            "Volcano",
            "Ember base ignited by molten orange thermal ripples.",
            ACCENT_ORANGE,
        ),
        Preset::Synthwave => (
            "Synthwave",
            "Deep ultraviolet backdrop with vivid magenta waves.",
            ACCENT_MAGENTA,
        ),
        Preset::Rainbow => (
            "Rainbow Wave",
            "Dynamic HSV rainbow ripples cycling with every keypress.",
            ACCENT_VIOLET,
        ),
        Preset::Profile1 => (
            "Profil 1",
            "User-saved custom configuration slot 1.",
            ACCENT_CYAN,
        ),
        Preset::Profile2 => (
            "Profil 2",
            "User-saved custom configuration slot 2.",
            WARNING,
        ),
    }
}

/// Renders the 36x36 LenovoRGB-style 4-bar app mark in the top-left header.
pub fn render_app_mark(ui: &mut Ui) {
    let size = Vec2::splat(36.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, Rounding::same(8.0), SURFACE);
    painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, BORDER));

    let bar_colors = [ACCENT_CYAN, ACCENT_VIOLET, ACCENT_MAGENTA, SUCCESS];
    let bar_w = 3.5;
    let gap = 2.5;
    let total_w = bar_w * 4.0 + gap * 3.0;
    let start_x = rect.center().x - total_w / 2.0;
    let bar_h = 18.0;
    let top_y = rect.center().y - bar_h / 2.0;

    for (i, color) in bar_colors.iter().enumerate() {
        let x = start_x + (i as f32) * (bar_w + gap);
        let bar_rect = Rect::from_min_size(Pos2::new(x, top_y), Vec2::new(bar_w, bar_h));
        painter.rect_filled(bar_rect, Rounding::same(1.75), *color);
    }
}

/// Renders a tinted status pill with a crisp 6px circle dot and label.
pub fn render_status_pill(ui: &mut Ui, text: &str, dot_color: Color32) {
    let bg = blend_color(dot_color, BG_BASE, 0.86);
    let border = blend_color(dot_color, BORDER, 0.72);
    egui::Frame {
        fill: bg,
        rounding: Rounding::same(6.0),
        stroke: Stroke::new(1.0_f32, border),
        inner_margin: Margin::symmetric(10.0, 5.0),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 3.2, dot_color);
            ui.label(
                egui::RichText::new(text)
                    .color(TEXT_SECONDARY)
                    .size(11.5),
            );
        });
    });
}

/// Renders a clickable sidebar navigation item with a 3px left cyan indicator bar when selected.
pub fn render_sidebar_nav_item(
    ui: &mut Ui,
    label: &str,
    count: Option<usize>,
    selected: bool,
) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let bg = if selected {
        SURFACE_ACTIVE
    } else if response.hovered() {
        SURFACE
    } else {
        Color32::TRANSPARENT
    };
    let fg = if selected || response.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(4.0), bg);

    if selected {
        let bar = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(
            bar,
            Rounding {
                nw: 4.0,
                sw: 4.0,
                ne: 0.0,
                se: 0.0,
            },
            ACCENT_CYAN,
        );
    }

    painter.text(
        Pos2::new(rect.left() + 13.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        fg,
    );

    if let Some(n) = count {
        painter.text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            n.to_string(),
            FontId::monospace(11.0),
            if selected { TEXT_SECONDARY } else { TEXT_MUTED },
        );
    }

    response.clicked()
}

/// Renders the compact LenovoRGB-style "NOW PLAYING" 4-zone + lightbar live preview in the sidebar.
/// Clicking any of the 4 keyboard zone blocks fires a preview ripple from that zone.
pub fn render_sidebar_preview(ui: &mut Ui, preview_zones: &[u32; 8]) -> Option<usize> {
    let mut clicked_zone = None;
    let width = ui.available_width();
    let height = 46.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), SURFACE_FIELD);
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, BORDER));

    let pad = 7.0;
    let gap = 4.0;
    let inner_w = (rect.width() - pad * 2.0).max(40.0);
    let block_w = (inner_w - gap * 3.0) / 4.0;

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let clicked = response.clicked();

    // Top row: 4 keyboard zones (0..4)
    for i in 0..4 {
        let x0 = rect.left() + pad + (i as f32) * (block_w + gap);
        let y0 = rect.top() + pad;
        let z_rect = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(block_w, 20.0));
        let is_hovered = pointer_pos.map_or(false, |p| z_rect.contains(p));
        if is_hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if clicked {
                clicked_zone = Some(i);
            }
        }

        let col = u32_to_color32(preview_zones[i]);
        let fill = if col == Color32::BLACK {
            Color32::from_rgb(18, 22, 32)
        } else {
            col
        };
        painter.rect_filled(z_rect, Rounding::same(3.5), fill);
        let stroke = if is_hovered {
            Stroke::new(1.2_f32, ACCENT_CYAN)
        } else {
            Stroke::new(1.0_f32, blend_color(fill, BORDER, 0.5))
        };
        painter.rect_stroke(z_rect, Rounding::same(3.5), stroke);
    }

    // Bottom row: 4 under-chassis wraparound lightbar segments (4..8)
    for i in 0..4 {
        let x0 = rect.left() + pad + (i as f32) * (block_w + gap);
        let y0 = rect.top() + pad + 24.0;
        let lb_rect = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(block_w, 5.0));
        let col = u32_to_color32(preview_zones[4 + i]);
        let fill = if col == Color32::BLACK {
            Color32::from_rgb(20, 26, 38)
        } else {
            col
        };
        painter.rect_filled(lb_rect, Rounding::same(2.5), fill);
    }

    clicked_zone
}

/// Renders a LenovoRGB-style mode card with a left accent bar, tinted monogram badge,
/// title, dual color swatches, and 2-line muted description.
pub fn render_preset_card(ui: &mut Ui, meta: &PresetCardMeta, width: f32, active: bool) -> bool {
    let height = 92.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let bg = if active {
        SURFACE_ACTIVE
    } else if response.hovered() {
        SURFACE_HOVER
    } else {
        SURFACE
    };

    let border_stroke = if active {
        Stroke::new(1.5_f32, ACCENT_CYAN)
    } else if response.hovered() {
        Stroke::new(1.0_f32, BORDER_HOVER)
    } else {
        Stroke::new(1.0_f32, BORDER)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), bg);

    // Left vertical accent indicator bar
    let accent_rect = Rect::from_min_size(rect.min, Vec2::new(3.5, rect.height()));
    painter.rect_filled(
        accent_rect,
        Rounding {
            nw: 6.0,
            sw: 6.0,
            ne: 0.0,
            se: 0.0,
        },
        meta.accent,
    );

    // Outer card border
    painter.rect_stroke(rect, Rounding::same(6.0), border_stroke);

    // 34x34 monogram square badge
    let badge_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 16.0, rect.center().y - 17.0),
        Vec2::splat(34.0),
    );
    let badge_bg = blend_color(meta.accent, bg, 0.80);
    painter.rect_filled(badge_rect, Rounding::same(6.0), badge_bg);
    painter.rect_stroke(
        badge_rect,
        Rounding::same(6.0),
        Stroke::new(1.0_f32, blend_color(meta.accent, BORDER, 0.55)),
    );
    painter.text(
        badge_rect.center(),
        Align2::CENTER_CENTER,
        meta.monogram,
        FontId::proportional(11.5),
        meta.accent,
    );

    // Dual color swatch dots in top-right
    let swatch_y = rect.top() + 21.0;
    let r_pos = Pos2::new(rect.right() - 16.0, swatch_y);
    let b_pos = Pos2::new(rect.right() - 28.0, swatch_y);
    painter.circle_filled(b_pos, 4.0, meta.base_swatch);
    painter.circle_stroke(b_pos, 4.0, Stroke::new(1.0_f32, BORDER_HOVER));
    painter.circle_filled(r_pos, 4.0, meta.ripple_swatch);
    painter.circle_stroke(r_pos, 4.0, Stroke::new(1.0_f32, BORDER_HOVER));

    // Title
    let text_left = badge_rect.right() + 14.0;
    let text_right = rect.right() - 40.0;
    let text_w = (text_right - text_left).max(60.0);

    painter.text(
        Pos2::new(text_left, rect.top() + 15.0),
        Align2::LEFT_TOP,
        meta.title,
        FontId::proportional(14.0),
        TEXT_PRIMARY,
    );

    // Wrapped 2-line muted description
    let galley = painter.layout(
        meta.description.to_string(),
        FontId::proportional(11.5),
        TEXT_SECONDARY,
        text_w,
    );
    painter.galley(Pos2::new(text_left, rect.top() + 36.0), galley, TEXT_SECONDARY);

    response.clicked()
}

/// Renders a custom-painted pill toggle switch (42x22 px).
pub fn render_toggle_switch(ui: &mut Ui, value: &mut bool) -> bool {
    let size = Vec2::new(42.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let mut changed = false;
    if response.clicked() {
        *value = !*value;
        changed = true;
    }

    let on = *value;
    let track_fill = if on { ACCENT_CYAN } else { SURFACE_ACTIVE };
    let track_stroke = if on {
        Stroke::new(1.0_f32, ACCENT_CYAN)
    } else {
        Stroke::new(1.0_f32, BORDER_HOVER)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(11.0), track_fill);
    painter.rect_stroke(rect, Rounding::same(11.0), track_stroke);

    let knob_x = if on {
        rect.right() - 11.0
    } else {
        rect.left() + 11.0
    };
    painter.circle_filled(Pos2::new(knob_x, rect.center().y), 7.5, Color32::WHITE);

    changed
}

/// Renders the LenovoRGB-style slim bottom brightness slider with rectangular cyan-outlined thumb.
pub fn render_brightness_slider(ui: &mut Ui, brightness: &mut u8) -> bool {
    let desired = Vec2::new(165.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let mut changed = false;
    let pad_x = 6.0;
    let track_left = rect.left() + pad_x;
    let track_right = rect.right() - pad_x;
    let track_w = (track_right - track_left).max(1.0);

    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let norm = ((pos.x - track_left) / track_w).clamp(0.0, 1.0);
            let new_val = (norm * 100.0).round() as u8;
            if new_val != *brightness {
                *brightness = new_val;
                changed = true;
            }
        }
    }

    let norm = (*brightness as f32 / 100.0).clamp(0.0, 1.0);
    let thumb_x = track_left + norm * track_w;

    let painter = ui.painter();
    let track_rect = Rect::from_min_max(
        Pos2::new(track_left, rect.center().y - 3.5),
        Pos2::new(track_right, rect.center().y + 3.5),
    );
    painter.rect_filled(track_rect, Rounding::same(2.0), SURFACE_FIELD);
    painter.rect_stroke(track_rect, Rounding::same(2.0), Stroke::new(1.0_f32, BORDER));

    if norm > 0.0 {
        let fill_rect = Rect::from_min_max(
            track_rect.min,
            Pos2::new(thumb_x, track_rect.max.y),
        );
        painter.rect_filled(fill_rect, Rounding::same(2.0), blend_color(ACCENT_CYAN, BG_BASE, 0.35));
    }

    let thumb_rect = Rect::from_center_size(Pos2::new(thumb_x, rect.center().y), Vec2::new(10.0, 14.0));
    painter.rect_filled(thumb_rect, Rounding::same(2.0), BG_ELEVATED);
    painter.rect_stroke(thumb_rect, Rounding::same(2.0), Stroke::new(1.5_f32, ACCENT_CYAN));

    changed
}

/// Renders the expanded clickable 4-zone keyboard + wraparound lightbar preview for the Customize view.
pub fn render_large_chassis_preview(ui: &mut Ui, preview_zones: &[u32; 8]) -> Option<usize> {
    let mut clicked_zone = None;
    let available_width = ui.available_width().max(320.0);

    // 4 keyboard zones
    let deck_height = 76.0;
    let (deck_rect, deck_resp) =
        ui.allocate_exact_size(Vec2::new(available_width, deck_height), Sense::click());
    let painter = ui.painter();

    painter.rect_filled(deck_rect, Rounding::same(6.0), SURFACE_FIELD);
    painter.rect_stroke(deck_rect, Rounding::same(6.0), Stroke::new(1.0_f32, BORDER));

    let pad = 6.0;
    let gap = 6.0;
    let inner_w = deck_rect.width() - pad * 2.0;
    let inner_h = deck_rect.height() - pad * 2.0;
    let zone_w = (inner_w - gap * 3.0) / 4.0;

    let zone_labels = [
        ("ZONE 0", "WASD / QWER"),
        ("ZONE 1", "MID-LEFT"),
        ("ZONE 2", "MID-RIGHT"),
        ("ZONE 3", "NUMPAD"),
    ];

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let deck_clicked = deck_resp.clicked();

    for i in 0..4 {
        let x0 = deck_rect.left() + pad + (i as f32) * (zone_w + gap);
        let y0 = deck_rect.top() + pad;
        let z_rect = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(zone_w, inner_h));
        let hovered = pointer_pos.map_or(false, |p| z_rect.contains(p));
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if deck_clicked {
                clicked_zone = Some(i);
            }
        }

        let col = u32_to_color32(preview_zones[i]);
        let r = col.r();
        let g = col.g();
        let b = col.b();
        let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
        let text_color = if lum > 140.0 {
            Color32::BLACK
        } else {
            TEXT_PRIMARY
        };
        let hex_color = if lum > 140.0 {
            Color32::from_rgb(24, 24, 24)
        } else {
            ACCENT_CYAN
        };

        painter.rect_filled(z_rect, Rounding::same(4.0), col);
        let stroke = if hovered {
            Stroke::new(1.5_f32, ACCENT_CYAN)
        } else {
            Stroke::new(1.0_f32, BORDER_HOVER)
        };
        painter.rect_stroke(z_rect, Rounding::same(4.0), stroke);

        painter.text(
            Pos2::new(z_rect.center().x, z_rect.top() + 14.0),
            Align2::CENTER_CENTER,
            zone_labels[i].0,
            FontId::proportional(11.0),
            text_color,
        );
        painter.text(
            Pos2::new(z_rect.center().x, z_rect.top() + 29.0),
            Align2::CENTER_CENTER,
            zone_labels[i].1,
            FontId::proportional(9.5),
            text_color,
        );
        painter.text(
            Pos2::new(z_rect.center().x, z_rect.bottom() - 13.0),
            Align2::CENTER_CENTER,
            format!("#{:02X}{:02X}{:02X}", r, g, b),
            FontId::monospace(10.0),
            hex_color,
        );
    }

    ui.add_space(6.0);

    // Wraparound under-chassis lightbar (Zones 4..7)
    let bar_height = 30.0;
    let (bar_rect, _) =
        ui.allocate_exact_size(Vec2::new(available_width, bar_height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(bar_rect, Rounding::same(6.0), SURFACE_FIELD);
    painter.rect_stroke(bar_rect, Rounding::same(6.0), Stroke::new(1.0_f32, BORDER));

    let lb_w = (bar_rect.width() - pad * 2.0 - gap * 3.0) / 4.0;
    let lb_h = bar_rect.height() - pad * 2.0;
    let lb_labels = ["Z4 L-WRAP", "Z5 MID-L", "Z6 MID-R", "Z7 R-WRAP"];

    for i in 0..4 {
        let z_idx = 4 + i;
        let x0 = bar_rect.left() + pad + (i as f32) * (lb_w + gap);
        let y0 = bar_rect.top() + pad;
        let pill = Rect::from_min_size(Pos2::new(x0, y0), Vec2::new(lb_w, lb_h));

        let col = u32_to_color32(preview_zones[z_idx]);
        let lum = 0.299 * (col.r() as f32) + 0.587 * (col.g() as f32) + 0.114 * (col.b() as f32);
        let fg = if lum > 140.0 {
            Color32::BLACK
        } else {
            TEXT_PRIMARY
        };

        painter.rect_filled(pill, Rounding::same(4.0), col);
        painter.rect_stroke(pill, Rounding::same(4.0), Stroke::new(1.0_f32, BORDER_HOVER));
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            lb_labels[i],
            FontId::proportional(9.5),
            fg,
        );
    }

    clicked_zone
}

/// Scales an unscaled RGB color by percentage (`0..=100`).
pub fn scale_base_color_pct(base_unscaled: [u8; 3], pct: u32) -> [u8; 3] {
    if pct == 0 {
        return [0, 0, 0];
    }
    let f = (pct.min(100) as f32) / 100.0;
    [
        (base_unscaled[0] as f32 * f).round().clamp(0.0, 255.0) as u8,
        (base_unscaled[1] as f32 * f).round().clamp(0.0, 255.0) as u8,
        (base_unscaled[2] as f32 * f).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Renders clean, emoji-free physics & color controls for the Customize view.
/// Returns `true` if any parameter was changed.
pub fn render_customize_controls(
    ui: &mut Ui,
    config: &mut RippleConfig,
    wave_speed_ms: &mut u32,
    ripple_duration_ms: &mut u32,
    base_glow_pct: &mut u32,
    base_unscaled: &mut [u8; 3],
) -> bool {
    let mut changed = false;

    egui::Grid::new("customize_physics_grid")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            // Wave Speed
            ui.label(egui::RichText::new("Wave speed").color(TEXT_SECONDARY).size(12.5));
            let speed_slider = egui::Slider::new(wave_speed_ms, 30..=150).suffix(" ms/zone");
            if ui.add(speed_slider).changed() {
                config.speed = 65.0 / (*wave_speed_ms as f32).max(1.0);
                changed = true;
            }
            ui.end_row();

            // Ripple Duration
            ui.label(egui::RichText::new("Ripple duration").color(TEXT_SECONDARY).size(12.5));
            let dur_slider = egui::Slider::new(ripple_duration_ms, 150..=800).suffix(" ms");
            if ui.add(dur_slider).changed() {
                config.decay = (*ripple_duration_ms as f32 / 500.0).clamp(0.3, 1.6);
                changed = true;
            }
            ui.end_row();

            // Lightbar Cascade Delay
            ui.label(egui::RichText::new("Lightbar cascade").color(TEXT_SECONDARY).size(12.5));
            let mut cascade_val = config.lightbar_delay_ms as u32;
            let cascade_slider = egui::Slider::new(&mut cascade_val, 10..=100).suffix(" ms");
            if ui.add(cascade_slider).changed() {
                config.lightbar_delay_ms = cascade_val as u64;
                changed = true;
            }
            ui.end_row();

            // Idle Base Glow Intensity
            ui.label(egui::RichText::new("Base glow level").color(TEXT_SECONDARY).size(12.5));
            let glow_slider = egui::Slider::new(base_glow_pct, 0..=100).suffix("%");
            if ui.add(glow_slider).changed() {
                if *base_unscaled == [0, 0, 0] && *base_glow_pct > 0 {
                    *base_unscaled = if config.color_ripple != [0, 0, 0] {
                        config.color_ripple
                    } else {
                        [34, 211, 238]
                    };
                }
                config.color_base = scale_base_color_pct(*base_unscaled, *base_glow_pct);
                changed = true;
            }
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(10.0);

    // Colors row
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Base color").color(TEXT_SECONDARY).size(12.5));
        if ui.color_edit_button_srgb(&mut config.color_base).changed() {
            *base_unscaled = config.color_base;
            *base_glow_pct = if config.color_base == [0, 0, 0] { 0 } else { 100 };
            changed = true;
        }
        ui.label(
            egui::RichText::new(format!(
                "#{:02X}{:02X}{:02X}",
                config.color_base[0], config.color_base[1], config.color_base[2]
            ))
            .color(TEXT_MUTED)
            .monospace()
            .size(11.0),
        );

        ui.add_space(20.0);

        ui.label(egui::RichText::new("Ripple color").color(TEXT_SECONDARY).size(12.5));
        if ui.color_edit_button_srgb(&mut config.color_ripple).changed() {
            changed = true;
        }
        ui.label(
            egui::RichText::new(format!(
                "#{:02X}{:02X}{:02X}",
                config.color_ripple[0], config.color_ripple[1], config.color_ripple[2]
            ))
            .color(TEXT_MUTED)
            .monospace()
            .size(11.0),
        );
    });

    ui.add_space(10.0);

    // Dynamic HSV Rainbow Mode toggle & hue shift
    ui.horizontal(|ui| {
        if render_toggle_switch(ui, &mut config.rainbow) {
            changed = true;
        }
        ui.label(
            egui::RichText::new("Dynamic HSV rainbow cycle")
                .color(TEXT_PRIMARY)
                .size(12.5),
        );

        if config.rainbow {
            ui.add_space(14.0);
            let mut step = config.hue_step_deg as u32;
            let hue_slider = egui::Slider::new(&mut step, 5..=60).suffix(" deg/key");
            if ui.add(hue_slider).changed() {
                config.hue_step_deg = step as u16;
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
        let packed = 0x00FF0080u32;
        let c = u32_to_color32(packed);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 0);
        assert_eq!(c.b(), 128);

        assert_eq!(u32_to_color32(0x00000000), Color32::from_rgb(0, 0, 0));
        assert_eq!(u32_to_color32(0x00FFFFFF), Color32::from_rgb(255, 255, 255));
    }

    #[test]
    fn test_build_preset_cards_count_and_custom_swatches() {
        let mut cfg = RippleConfig::default();
        cfg.profile_1.color_ripple = [11, 22, 33];
        let cards = build_preset_cards(&cfg);
        assert_eq!(cards.len(), 8);
        assert_eq!(cards[0].monogram, "CY");
        assert_eq!(cards[6].ripple_swatch, Color32::from_rgb(11, 22, 33));
        assert!(cards[6].is_custom);
        assert!(cards[7].is_custom);
    }

    #[test]
    fn test_scale_base_color_pct() {
        assert_eq!(scale_base_color_pct([200, 100, 40], 0), [0, 0, 0]);
        assert_eq!(scale_base_color_pct([200, 100, 40], 50), [100, 50, 20]);
        assert_eq!(scale_base_color_pct([200, 100, 40], 100), [200, 100, 40]);
    }
}
