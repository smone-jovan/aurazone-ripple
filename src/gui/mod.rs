//! Modern Cyberpunk Native GUI application using egui/eframe.
//!
//! Provides real-time 4-zone keyboard & wraparound lightbar visualizer,
//! parameter controls, preset switcher, and dual-mode synchronization
//! (IPC Named Pipe daemon connection or standalone mock preview).

pub mod ipc_sync;
pub mod theme;
pub mod widgets;

use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::{Preset, RippleConfig};
use crate::hid::MockAuraDevice;
use crate::physics::{advance_rainbow, RippleEngine};
use ipc_sync::{DaemonStatusUpdate, GuiIpcCommand, IpcSyncBridge};
use theme::*;
use widgets::*;

/// Main GUI Application state.
pub struct AuraGuiApp {
    pub config: RippleConfig,
    pub settings_path: PathBuf,
    pub daemon_status: DaemonStatusUpdate,
    pub ipc_bridge: Option<IpcSyncBridge>,
    pub preview_zones: [u32; 8],

    pub wave_speed_ms: u32,
    pub ripple_duration_ms: u32,
    pub idle_dim_pct: u32,
    pub hue_increment_deg: u32,

    pub is_dirty: bool,
    pub status_message: String,
    pub status_timestamp: Instant,

    // Standalone simulation state
    pub mock_engine: RippleEngine,
    pub mock_device: MockAuraDevice,
    pub current_hue: f32,
}

impl AuraGuiApp {
    /// Creates a new AuraGuiApp instance.
    pub fn new() -> Self {
        let settings_path = std::env::var("AURAZONE_SETTINGS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ripple_settings.json"));

        let (config, _) = RippleConfig::load_or_recover(&settings_path);

        let wave_speed_ms = if config.speed > 0.05 {
            (65.0 / config.speed).round() as u32
        } else {
            65
        }
        .clamp(30, 150);

        let ripple_duration_ms = (config.decay * 500.0).round() as u32;
        let ripple_duration_ms = ripple_duration_ms.clamp(150, 800);

        let idle_dim_pct = if config.color_base == [0, 0, 0] { 0 } else { 25 };

        let hue_increment_deg = 18;

        // Base color packed to u32
        let base = config.color_base;
        let u32_base = ((base[0] as u32) << 16) | ((base[1] as u32) << 8) | (base[2] as u32);
        let preview_zones = [u32_base; 8];

        let ipc_bridge = Some(IpcSyncBridge::spawn());

        Self {
            config,
            settings_path,
            daemon_status: DaemonStatusUpdate::default(),
            ipc_bridge,
            preview_zones,
            wave_speed_ms,
            ripple_duration_ms,
            idle_dim_pct,
            hue_increment_deg,
            is_dirty: false,
            status_message: "Ready".to_string(),
            status_timestamp: Instant::now(),
            mock_engine: RippleEngine::new(),
            mock_device: MockAuraDevice::new(),
            current_hue: 0.0,
        }
    }

    /// Triggers an interactive ripple on a specific zone in the simulation engine.
    pub fn trigger_preview_ripple(&mut self, zone: usize) {
        let ripple_color = if self.config.rainbow {
            advance_rainbow(&mut self.current_hue)
        } else {
            self.config.color_ripple
        };

        let duration = Duration::from_millis(self.ripple_duration_ms as u64);
        self.mock_engine
            .add_ripple_with_duration(zone, ripple_color, duration);
    }

    /// Switches the active preset and synchronizes configuration.
    pub fn apply_preset(&mut self, preset: Preset) {
        self.config = RippleConfig::from_preset(preset);

        self.wave_speed_ms = if self.config.speed > 0.05 {
            (65.0 / self.config.speed).round() as u32
        } else {
            65
        }
        .clamp(30, 150);

        self.ripple_duration_ms = (self.config.decay * 500.0).round() as u32;
        self.ripple_duration_ms = self.ripple_duration_ms.clamp(150, 800);

        self.idle_dim_pct = if self.config.color_base == [0, 0, 0] {
            0
        } else {
            25
        };

        self.is_dirty = true;
        self.save_and_apply();
        self.trigger_preview_ripple(0);
    }

    /// Persists configuration to disk and notifies daemon over Named Pipe.
    pub fn save_and_apply(&mut self) {
        match self.config.save_to_path(&self.settings_path) {
            Ok(_) => {
                self.is_dirty = false;
                if let Some(bridge) = &self.ipc_bridge {
                    let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Reload);
                }
                self.status_message = "Configuration saved & reloaded.".to_string();
                self.status_timestamp = Instant::now();
            }
            Err(e) => {
                self.status_message = format!("Save failed: {}", e);
                self.status_timestamp = Instant::now();
            }
        }
    }
}

impl Default for AuraGuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for AuraGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain non-blocking IPC status updates
        if let Some(bridge) = &self.ipc_bridge {
            while let Ok(update) = bridge.rx_state.try_recv() {
                self.daemon_status = update;
            }
        }

        // 2. Step internal simulation physics (50 FPS superposition)
        let now = Instant::now();
        let spread = Duration::from_millis(self.wave_speed_ms as u64);
        let cascade = Duration::from_millis(self.config.lightbar_delay_ms);

        self.mock_engine.prune_expired(now, spread, cascade);
        let active_count = self.mock_engine.active_ripple_count();

        if active_count > 0 {
            let zone_colors =
                self.mock_engine
                    .render_at_time(now, self.config.color_base, spread, cascade);
            for (z, c) in zone_colors.iter().enumerate() {
                self.preview_zones[z] =
                    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
            }
        } else {
            let base = self.config.color_base;
            let u32_base = ((base[0] as u32) << 16) | ((base[1] as u32) << 8) | (base[2] as u32);
            self.preview_zones = [u32_base; 8];
        }

        // 3. Render Cyberpunk Interface
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: COLOR_BG_OBSIDIAN,
                inner_margin: Margin::same(16.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Header Bar: Title, Subtitle, and Connection Status
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("AURA ZONE RIPPLE")
                                .color(COLOR_NEON_CYAN)
                                .size(20.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("ASUS ROG Strix G513RC • Stealth Edition")
                                .color(COLOR_TEXT_MUTED)
                                .size(11.0),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.daemon_status.is_connected && self.daemon_status.is_running {
                            let (badge_text, badge_color) = if self.daemon_status.is_active {
                                ("● DAEMON ACTIVE", COLOR_MATRIX_GREEN)
                            } else {
                                ("⏸ DAEMON PAUSED", COLOR_VOLCANO_ORANGE)
                            };

                            ui.label(
                                egui::RichText::new(badge_text)
                                    .color(badge_color)
                                    .strong()
                                    .size(12.0),
                            );

                            let toggle_btn_text = if self.daemon_status.is_active {
                                "Pause"
                            } else {
                                "Resume"
                            };

                            if ui
                                .button(egui::RichText::new(toggle_btn_text).color(COLOR_TEXT_ICE))
                                .clicked()
                            {
                                if let Some(bridge) = &self.ipc_bridge {
                                    let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Toggle);
                                }
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("○ STANDALONE PREVIEW")
                                    .color(COLOR_VOLCANO_ORANGE)
                                    .size(12.0),
                            );
                        }
                    });
                });

                ui.add_space(12.0);

                // Section 1: Live Visualizer Card
                egui::Frame {
                    fill: COLOR_PANEL_CARD,
                    rounding: Rounding::same(6.0),
                    stroke: Stroke::new(1.0_f32, COLOR_BORDER_CARD),
                    inner_margin: Margin::same(12.0),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("LIVE 4-ZONE CHASSIS VISUALIZER")
                            .color(COLOR_NEON_CYAN)
                            .size(13.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Real-time preview of keyboard deck and under-chassis lightbar")
                            .color(COLOR_TEXT_MUTED)
                            .size(10.0),
                    );
                    ui.add_space(8.0);

                    // Procedural Keyboard Deck
                    if let Some(zone) = render_keyboard_deck(ui, &self.preview_zones) {
                        self.trigger_preview_ripple(zone);
                    }

                    ui.add_space(8.0);

                    // Wraparound Lightbar Deck
                    render_lightbar_deck(ui, &self.preview_zones);

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("💡 Tip: Click any keyboard zone above to fire an interactive ripple.")
                                .color(COLOR_TEXT_MUTED)
                                .size(10.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(egui::RichText::new("⚡ Fire Test Ripple").color(COLOR_NEON_CYAN))
                                .clicked()
                            {
                                self.trigger_preview_ripple(0);
                            }
                        });
                    });
                });

                ui.add_space(12.0);

                // Section 2: Preset Badges
                egui::Frame {
                    fill: COLOR_PANEL_CARD,
                    rounding: Rounding::same(6.0),
                    stroke: Stroke::new(1.0_f32, COLOR_BORDER_CARD),
                    inner_margin: Margin::same(12.0),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("PRESET PROFILES")
                            .color(COLOR_NEON_CYAN)
                            .size(13.0)
                            .strong(),
                    );
                    ui.add_space(6.0);

                    if let Some(preset) = render_preset_switcher(ui, self.config.preset) {
                        self.apply_preset(preset);
                    }
                });

                ui.add_space(12.0);

                // Section 3: Parameter Controls
                egui::Frame {
                    fill: COLOR_PANEL_CARD,
                    rounding: Rounding::same(6.0),
                    stroke: Stroke::new(1.0_f32, COLOR_BORDER_CARD),
                    inner_margin: Margin::same(12.0),
                    ..Default::default()
                }
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("EFFECT PARAMETERS & COLOR CONFIGURATION")
                            .color(COLOR_NEON_CYAN)
                            .size(13.0)
                            .strong(),
                    );
                    ui.add_space(6.0);

                    let changed = render_parameter_controls(
                        ui,
                        &mut self.config,
                        &mut self.wave_speed_ms,
                        &mut self.ripple_duration_ms,
                        &mut self.idle_dim_pct,
                        &mut self.hue_increment_deg,
                    );

                    if changed {
                        self.is_dirty = true;
                    }
                });

                ui.add_space(12.0);

                // Bottom Action Bar: Save & Apply, Status Message
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("💾 Save & Apply Changes")
                            .color(if self.is_dirty {
                                Color32::BLACK
                            } else {
                                COLOR_TEXT_ICE
                            })
                            .strong(),
                    )
                    .fill(if self.is_dirty {
                        COLOR_NEON_CYAN
                    } else {
                        Color32::from_rgb(28, 30, 44)
                    })
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER_CARD))
                    .min_size(Vec2::new(160.0, 30.0));

                    if ui.add(save_btn).clicked() {
                        self.save_and_apply();
                    }

                    ui.add_space(12.0);

                    if self.status_timestamp.elapsed() < Duration::from_secs(4) {
                        ui.label(
                            egui::RichText::new(&self.status_message)
                                .color(COLOR_MATRIX_GREEN)
                                .size(11.0),
                        );
                    }
                });
            });

        // 4. Dynamic Repaint Optimization: 50 FPS when ripple is active, 1 Hz when idle
        if active_count > 0 {
            ctx.request_repaint_after(Duration::from_millis(20));
        } else {
            ctx.request_repaint_after(Duration::from_millis(1000));
        }
    }
}

/// Launches the native GUI application window.
pub fn run_gui() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([580.0, 750.0])
            .with_min_inner_size([520.0, 640.0])
            .with_title("AuraZoneRipple — Aura Control Center"),
        ..Default::default()
    };

    eframe::run_native(
        "AuraZoneRipple",
        native_options,
        Box::new(|cc| {
            theme::apply_cyberpunk_theme(&cc.egui_ctx);
            Ok(Box::new(AuraGuiApp::new()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_app_initialization() {
        let mut app = AuraGuiApp::new();
        assert_eq!(app.preview_zones.len(), 8);
        assert!(!app.is_dirty);
        assert_eq!(app.mock_engine.active_ripple_count(), 0);

        // Test triggering preview ripple
        app.trigger_preview_ripple(0);
        assert_eq!(app.mock_engine.active_ripple_count(), 1);

        // Test applying preset
        app.apply_preset(Preset::Volcano);
        assert_eq!(app.config.preset, Preset::Volcano);
        assert_eq!(app.config.color_ripple, [255, 60, 0]);
    }
}
