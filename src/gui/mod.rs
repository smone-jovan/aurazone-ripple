//! Sleek Native GUI application modeled after Shushant17711/LenovoRGB.
//!
//! Features a landscape two-column control center:
//! - Top header bar with 4-bar app mark, status pill, and Settings access
//! - Left sidebar with Library navigation + live "Now Playing" 4-zone & lightbar preview
//! - Right workspace with 2-column preset cards, live Customize studio, and Gaming Elevation Settings
//! - Bottom docked bar with Brightness slider, runtime status, and quick Lights off / Resume

pub mod ipc_sync;
pub mod theme;
pub mod widgets;

use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::{Preset, RippleConfig};
use crate::hid::MockAuraDevice;
use crate::patcher::GHelperPatchStatus;
use crate::physics::{advance_rainbow_by, scale_rgb_brightness, RippleEngine};
use ipc_sync::{DaemonStatusUpdate, GuiIpcCommand, IpcSyncBridge};
use theme::*;
use widgets::*;

/// Primary sidebar & header navigation view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    AllPresets,
    BuiltIn,
    CustomProfiles,
    Customize,
    Settings,
}

/// Self-onboarding, G-Helper patch, and Gaming Task elevation health state.
#[derive(Clone, Debug)]
pub struct OnboardingHealth {
    pub ghelper_status: GHelperPatchStatus,
    pub startup_task_installed: bool,
    pub startup_task_elevated: bool,
    pub last_action_message: Option<(String, bool)>,
}

impl OnboardingHealth {
    pub fn new() -> Self {
        let mut s = Self {
            ghelper_status: GHelperPatchStatus::NotFound,
            startup_task_installed: false,
            startup_task_elevated: false,
            last_action_message: None,
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        self.ghelper_status = crate::patcher::check_ghelper_patch_state();
        self.startup_task_installed = crate::patcher::is_startup_task_installed();
        self.startup_task_elevated = crate::patcher::is_startup_task_elevated();
    }

    pub fn needs_attention(&self) -> bool {
        !matches!(self.ghelper_status, GHelperPatchStatus::Patched(_))
            || !self.startup_task_installed
            || !self.startup_task_elevated
    }
}

/// Main GUI Application state.
pub struct AuraGuiApp {
    pub active_view: ActiveView,
    pub config: RippleConfig,
    pub settings_path: PathBuf,
    pub daemon_status: DaemonStatusUpdate,
    pub ipc_bridge: Option<IpcSyncBridge>,
    pub preview_zones: [u32; 8],

    pub wave_speed_ms: u32,
    pub ripple_duration_ms: u32,
    pub base_glow_pct: u32,
    pub base_unscaled: [u8; 3],

    pub is_dirty: bool,
    pub status_message: String,
    pub status_timestamp: Instant,
    pub pending_auto_apply: Option<Instant>,
    pub post_uac_poll_until: Option<Instant>,
    pub last_uac_poll: Instant,

    // Standalone simulation state
    pub mock_engine: RippleEngine,
    pub mock_device: MockAuraDevice,
    pub current_hue: f32,

    // System integration & gaming elevation health
    pub onboarding_health: OnboardingHealth,
}

impl AuraGuiApp {
    /// Creates a new AuraGuiApp instance.
    pub fn new() -> Self {
        let settings_path = crate::config::get_canonical_settings_path();
        let (mut config, _) = RippleConfig::load_or_recover(&settings_path);
        config.brightness = config.brightness.min(100);

        let wave_speed_ms = Self::compute_wave_speed_ms(config.speed);
        let ripple_duration_ms = Self::compute_ripple_duration_ms(config.decay);
        let base_glow_pct = if config.color_base == [0, 0, 0] { 0 } else { 100 };
        let base_unscaled = config.color_base;

        let base_scaled = scale_rgb_brightness(config.color_base, config.brightness);
        let u32_base = ((base_scaled[0] as u32) << 16)
            | ((base_scaled[1] as u32) << 8)
            | (base_scaled[2] as u32);
        let preview_zones = [u32_base; 8];

        let ipc_bridge = Some(IpcSyncBridge::spawn());
        let onboarding_health = OnboardingHealth::new();

        Self {
            active_view: ActiveView::AllPresets,
            config,
            settings_path,
            daemon_status: DaemonStatusUpdate::default(),
            ipc_bridge,
            preview_zones,
            wave_speed_ms,
            ripple_duration_ms,
            base_glow_pct,
            base_unscaled,
            is_dirty: false,
            status_message: String::new(),
            status_timestamp: Instant::now() - Duration::from_secs(10),
            pending_auto_apply: None,
            post_uac_poll_until: None,
            last_uac_poll: Instant::now(),
            mock_engine: RippleEngine::new(),
            mock_device: MockAuraDevice::new(),
            current_hue: 0.0,
            onboarding_health,
        }
    }

    #[inline]
    fn compute_wave_speed_ms(speed: f32) -> u32 {
        if speed > 0.05 {
            (65.0 / speed).round() as u32
        } else {
            65
        }
        .clamp(30, 150)
    }

    #[inline]
    fn compute_ripple_duration_ms(decay: f32) -> u32 {
        (decay * 500.0).round().clamp(150.0, 800.0) as u32
    }

    fn sync_derived_sliders(&mut self) {
        self.wave_speed_ms = Self::compute_wave_speed_ms(self.config.speed);
        self.ripple_duration_ms = Self::compute_ripple_duration_ms(self.config.decay);
        self.base_unscaled = self.config.color_base;
        self.base_glow_pct = if self.config.color_base == [0, 0, 0] {
            0
        } else {
            100
        };
    }

    /// Triggers an interactive ripple on a specific zone in the simulation engine.
    pub fn trigger_preview_ripple(&mut self, zone: usize) {
        let ripple_color = if self.config.rainbow {
            advance_rainbow_by(&mut self.current_hue, self.config.hue_step_deg as f32)
        } else {
            self.config.color_ripple
        };

        let duration = Duration::from_millis(self.ripple_duration_ms as u64);
        self.mock_engine
            .add_ripple_with_duration(zone, ripple_color, duration);
    }

    /// Switches the active preset and immediately applies & reloads daemon configuration.
    pub fn apply_preset(&mut self, preset: Preset) {
        self.config.switch_preset(preset);
        self.sync_derived_sliders();
        self.is_dirty = true;
        self.pending_auto_apply = None;
        self.save_and_apply();
        self.trigger_preview_ripple(0);
    }

    /// Saves the current effect parameters to the chosen user profile slot (1 or 2).
    pub fn save_to_profile(&mut self, slot: u8) {
        self.config.save_current_to_profile(slot);
        self.is_dirty = true;
        self.pending_auto_apply = None;
        self.save_and_apply();
        self.status_message = format!("Saved to Profil {}", slot);
        self.status_timestamp = Instant::now();
    }

    /// Marks configuration dirty and schedules automatic save/reload after 100ms debounce.
    fn schedule_auto_apply(&mut self) {
        self.is_dirty = true;
        self.pending_auto_apply = Some(Instant::now() + Duration::from_millis(100));
    }

    /// Persists configuration to disk and notifies daemon over Named Pipe.
    pub fn save_and_apply(&mut self) {
        self.config.brightness = self.config.brightness.min(100);
        match self.config.save_to_path(&self.settings_path) {
            Ok(_) => {
                self.is_dirty = false;
                self.pending_auto_apply = None;
                if let Some(bridge) = &self.ipc_bridge {
                    let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Reload);
                }
                self.status_message = "Configuration saved".to_string();
                self.status_timestamp = Instant::now();
            }
            Err(e) => {
                self.status_message = format!("Save failed: {}", e);
                self.status_timestamp = Instant::now();
            }
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            render_app_mark(ui);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(
                    egui::RichText::new("AuraZone Ripple")
                        .color(TEXT_PRIMARY)
                        .size(17.5)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("ASUS ROG Strix G513RC · 4-zone keyboard & lightbar")
                        .color(TEXT_MUTED)
                        .size(11.0),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let in_settings = self.active_view == ActiveView::Settings;
                let needs_setup = self.onboarding_health.needs_attention()
                    || (self.daemon_status.is_connected && !self.daemon_status.is_elevated);

                let btn_label = if in_settings {
                    "Back to presets"
                } else {
                    "Settings"
                };

                let btn = egui::Button::new(
                    egui::RichText::new(btn_label)
                        .color(if in_settings {
                            ACCENT_CYAN
                        } else {
                            TEXT_PRIMARY
                        })
                        .size(12.0)
                        .strong(),
                )
                .fill(if in_settings { SURFACE_ACTIVE } else { SURFACE })
                .stroke(Stroke::new(
                    1.0_f32,
                    if in_settings {
                        ACCENT_CYAN
                    } else if needs_setup {
                        WARNING
                    } else {
                        BORDER
                    },
                ))
                .min_size(Vec2::new(84.0, 30.0));

                if ui.add(btn).clicked() {
                    self.active_view = if in_settings {
                        ActiveView::AllPresets
                    } else {
                        ActiveView::Settings
                    };
                }

                ui.add_space(6.0);

                if self.daemon_status.is_connected && self.daemon_status.is_running {
                    if self.daemon_status.is_active {
                        if self.daemon_status.is_elevated {
                            render_status_pill(ui, "Connected · Gaming Ready", SUCCESS);
                        } else {
                            render_status_pill(ui, "Connected · Standard", WARNING);
                        }
                    } else {
                        render_status_pill(ui, "Connected · Paused", WARNING);
                    }
                } else {
                    render_status_pill(ui, "Preview Mode", ACCENT_VIOLET);
                }
            });
        });
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("LIBRARY")
                .color(TEXT_MUTED)
                .size(10.0)
                .strong(),
        );
        ui.add_space(4.0);

        if render_sidebar_nav_item(
            ui,
            "All presets",
            Some(8),
            self.active_view == ActiveView::AllPresets,
        ) {
            self.active_view = ActiveView::AllPresets;
        }
        if render_sidebar_nav_item(
            ui,
            "Built-in",
            Some(6),
            self.active_view == ActiveView::BuiltIn,
        ) {
            self.active_view = ActiveView::BuiltIn;
        }
        if render_sidebar_nav_item(
            ui,
            "Custom profiles",
            Some(2),
            self.active_view == ActiveView::CustomProfiles,
        ) {
            self.active_view = ActiveView::CustomProfiles;
        }
        if render_sidebar_nav_item(
            ui,
            "Customize",
            None,
            self.active_view == ActiveView::Customize,
        ) {
            self.active_view = ActiveView::Customize;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(
            egui::RichText::new("NOW PLAYING")
                .color(TEXT_MUTED)
                .size(10.0)
                .strong(),
        );
        ui.add_space(4.0);

        let (preset_title, preset_desc, preset_accent) = preset_summary(self.config.preset);
        ui.label(
            egui::RichText::new(preset_title)
                .color(TEXT_PRIMARY)
                .size(14.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(preset_desc)
                .color(TEXT_MUTED)
                .size(11.0),
        );

        ui.add_space(10.0);

        if let Some(zone) = render_sidebar_preview(ui, &self.preview_zones) {
            self.trigger_preview_ripple(zone);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let test_btn = egui::Button::new(
                egui::RichText::new("Test wave")
                    .color(preset_accent)
                    .size(11.0),
            )
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER));

            if ui.add(test_btn).clicked() {
                self.trigger_preview_ripple(0);
            }
        });
    }

    fn render_footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.label(
                egui::RichText::new("BRIGHTNESS")
                    .color(TEXT_MUTED)
                    .size(10.5)
                    .strong(),
            );
            ui.add_space(4.0);

            if render_brightness_slider(ui, &mut self.config.brightness) {
                self.schedule_auto_apply();
            }

            ui.label(
                egui::RichText::new(format!("{:>3}%", self.config.brightness))
                    .color(TEXT_SECONDARY)
                    .monospace()
                    .size(11.5),
            );

            ui.add_space(12.0);

            if self.status_timestamp.elapsed() < Duration::from_secs(3) && !self.status_message.is_empty() {
                ui.label(
                    egui::RichText::new(&self.status_message)
                        .color(ACCENT_CYAN)
                        .size(11.5),
                );
            } else if self.daemon_status.is_connected && self.daemon_status.is_running {
                let (title, _, _) = preset_summary(self.config.preset);
                if self.daemon_status.is_active {
                    ui.label(
                        egui::RichText::new(format!("{} is running", title))
                            .color(SUCCESS)
                            .size(11.5),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Lighting paused")
                            .color(TEXT_MUTED)
                            .size(11.5),
                    );
                }
            } else {
                ui.label(
                    egui::RichText::new("Standalone simulation")
                        .color(TEXT_MUTED)
                        .size(11.5),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.daemon_status.is_connected && self.daemon_status.is_running {
                    let (btn_text, btn_color, cmd) = if self.daemon_status.is_active {
                        ("Lights off", DANGER, GuiIpcCommand::Pause)
                    } else {
                        ("Resume", SUCCESS, GuiIpcCommand::Resume)
                    };

                    let btn = egui::Button::new(
                        egui::RichText::new(btn_text)
                            .color(btn_color)
                            .size(12.0)
                            .strong(),
                    )
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .min_size(Vec2::new(88.0, 28.0));

                    if ui.add(btn).clicked() {
                        if let Some(bridge) = &self.ipc_bridge {
                            let _ = bridge.tx_cmd.try_send(cmd);
                        }
                    }
                } else {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("Save config")
                            .color(ACCENT_CYAN)
                            .size(12.0)
                            .strong(),
                    )
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .min_size(Vec2::new(92.0, 28.0));

                    if ui.add(save_btn).clicked() {
                        self.save_and_apply();
                    }
                }
            });
        });
    }

    fn render_presets_view(&mut self, ui: &mut egui::Ui, filter_custom: Option<bool>) {
        let all_cards = build_preset_cards(&self.config);
        let filtered: Vec<PresetCardMeta> = all_cards
            .into_iter()
            .filter(|c| match filter_custom {
                Some(want_custom) => c.is_custom == want_custom,
                None => true,
            })
            .collect();

        let (heading, subtitle) = match filter_custom {
            None => ("All presets", format!("{} effects", filtered.len())),
            Some(false) => ("Built-in", format!("{} factory palettes", filtered.len())),
            Some(true) => ("Custom profiles", format!("{} saved slots", filtered.len())),
        };

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(heading)
                    .color(TEXT_PRIMARY)
                    .size(17.0)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(&subtitle)
                    .color(TEXT_MUTED)
                    .size(11.5),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        egui::RichText::new("Save to Profil 2")
                            .color(WARNING)
                            .size(11.0),
                    )
                    .clicked()
                {
                    self.save_to_profile(2);
                }
                if ui
                    .button(
                        egui::RichText::new("Save to Profil 1")
                            .color(ACCENT_CYAN)
                            .size(11.0),
                    )
                    .clicked()
                {
                    self.save_to_profile(1);
                }
            });
        });

        ui.add_space(12.0);

        let avail_w = ui.available_width().max(420.0);
        let col_gap = 12.0;
        let card_w = ((avail_w - col_gap) / 2.0).max(200.0);

        let mut selected_preset: Option<Preset> = None;
        for pair in filtered.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = col_gap;
                if let Some(left) = pair.first() {
                    if render_preset_card(ui, left, card_w, self.config.preset == left.preset) {
                        selected_preset = Some(left.preset);
                    }
                }
                if let Some(right) = pair.get(1) {
                    if render_preset_card(ui, right, card_w, self.config.preset == right.preset) {
                        selected_preset = Some(right.preset);
                    }
                }
            });
            ui.add_space(10.0);
        }

        if let Some(preset) = selected_preset {
            self.apply_preset(preset);
        }
    }

    fn render_customize_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Customize")
                    .color(TEXT_PRIMARY)
                    .size(17.0)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Wave physics, base glow, and rainbow shift")
                    .color(TEXT_MUTED)
                    .size(11.5),
            );
        });

        ui.add_space(12.0);

        // Card 1: Interactive 4-zone + Lightbar Preview
        egui::Frame {
            fill: SURFACE,
            rounding: Rounding::same(6.0),
            stroke: Stroke::new(1.0_f32, BORDER),
            inner_margin: Margin::same(14.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Interactive 4-Zone Deck")
                        .color(TEXT_PRIMARY)
                        .size(13.5)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Trigger wave")
                                .color(ACCENT_CYAN)
                                .size(11.0),
                        )
                        .clicked()
                    {
                        self.trigger_preview_ripple(0);
                    }
                });
            });
            ui.add_space(8.0);
            if let Some(zone) = render_large_chassis_preview(ui, &self.preview_zones) {
                self.trigger_preview_ripple(zone);
            }
        });

        ui.add_space(12.0);

        // Card 2: Physics & Colors
        egui::Frame {
            fill: SURFACE,
            rounding: Rounding::same(6.0),
            stroke: Stroke::new(1.0_f32, BORDER),
            inner_margin: Margin::same(14.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Wave Physics & Palette")
                    .color(TEXT_PRIMARY)
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(10.0);

            if render_customize_controls(
                ui,
                &mut self.config,
                &mut self.wave_speed_ms,
                &mut self.ripple_duration_ms,
                &mut self.base_glow_pct,
                &mut self.base_unscaled,
            ) {
                self.schedule_auto_apply();
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let apply_btn = egui::Button::new(
                    egui::RichText::new("Save & Apply")
                        .color(if self.is_dirty { Color32::BLACK } else { TEXT_PRIMARY })
                        .size(12.0)
                        .strong(),
                )
                .fill(if self.is_dirty { ACCENT_CYAN } else { SURFACE_ACTIVE })
                .stroke(Stroke::new(1.0_f32, BORDER_HOVER));

                if ui.add(apply_btn).clicked() {
                    self.save_and_apply();
                }

                ui.add_space(8.0);
                if ui
                    .button(egui::RichText::new("Save to Profil 1").color(ACCENT_CYAN).size(11.5))
                    .clicked()
                {
                    self.save_to_profile(1);
                }
                if ui
                    .button(egui::RichText::new("Save to Profil 2").color(WARNING).size(11.5))
                    .clicked()
                {
                    self.save_to_profile(2);
                }
            });
        });
    }

    fn render_settings_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Settings")
                    .color(TEXT_PRIMARY)
                    .size(17.0)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Gaming privileges, Raw Input, G-Helper patch, and diagnostics")
                    .color(TEXT_MUTED)
                    .size(11.5),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("Re-check").color(TEXT_PRIMARY).size(11.5))
                    .clicked()
                {
                    self.onboarding_health.refresh();
                    if let Some(bridge) = &self.ipc_bridge {
                        let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Diag);
                    }
                }
            });
        });

        ui.add_space(12.0);

        // Card 1: Elevated Gaming Mode & Raw Input (Anti-Cheat / ZZZ UIPI Fix)
        egui::Frame {
            fill: SURFACE,
            rounding: Rounding::same(6.0),
            stroke: Stroke::new(
                1.0_f32,
                if !self.onboarding_health.startup_task_elevated
                    || (self.daemon_status.is_connected && !self.daemon_status.is_elevated)
                {
                    WARNING
                } else {
                    BORDER
                },
            ),
            inner_margin: Margin::same(14.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Gaming Elevation & Input Capture")
                    .color(TEXT_PRIMARY)
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Administrator-elevated games (such as Zenless Zone Zero) block keyboard input from standard background processes via Windows UIPI. Install the elevated Task Scheduler entry so the daemon captures Raw Input cleanly during gameplay.",
                )
                .color(TEXT_SECONDARY)
                .size(11.5),
            );
            ui.add_space(10.0);

            ui.horizontal_wrapped(|ui| {
                if self.daemon_status.is_connected {
                    if self.daemon_status.is_elevated {
                        render_status_pill(ui, "Daemon: Elevated (Admin)", SUCCESS);
                    } else {
                        render_status_pill(ui, "Daemon: Standard (Blocked by Admin Games)", WARNING);
                    }
                    if self.daemon_status.raw_input_active {
                        render_status_pill(ui, "Raw Input: Active", SUCCESS);
                    } else {
                        render_status_pill(ui, "Raw Input: Fallback Only", WARNING);
                    }
                } else {
                    render_status_pill(ui, "Daemon: Offline", DANGER);
                }

                if self.onboarding_health.startup_task_elevated {
                    render_status_pill(ui, "Task: HighestAvailable", SUCCESS);
                } else if self.onboarding_health.startup_task_installed {
                    render_status_pill(ui, "Task: Non-Elevated", WARNING);
                } else {
                    render_status_pill(ui, "Task: Not Installed", WARNING);
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let needs_elevated = !self.onboarding_health.startup_task_elevated
                    || (self.daemon_status.is_connected && !self.daemon_status.is_elevated);

                let install_btn = egui::Button::new(
                    egui::RichText::new(if needs_elevated {
                        "Install Elevated Gaming Task & Restart"
                    } else {
                        "Re-install Elevated Gaming Task"
                    })
                    .color(Color32::BLACK)
                    .size(12.0)
                    .strong(),
                )
                .fill(if needs_elevated { WARNING } else { ACCENT_CYAN });

                if ui.add(install_btn).clicked() {
                    match crate::patcher::launch_elevated_task_installer() {
                        Ok(_) => {
                            self.onboarding_health.last_action_message = Some((
                                "Elevated gaming task installer launched. Accept the UAC prompt to switch daemon to Admin mode.".into(),
                                false,
                            ));
                            self.post_uac_poll_until = Some(Instant::now() + Duration::from_secs(8));
                            self.onboarding_health.refresh();
                            if let Some(bridge) = &self.ipc_bridge {
                                let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Diag);
                            }
                        }
                        Err(e) => {
                            self.onboarding_health.last_action_message = Some((
                                format!("Failed to elevate task installer: {}", e),
                                true,
                            ));
                        }
                    }
                }

                if self.onboarding_health.startup_task_installed {
                    if ui
                        .button(egui::RichText::new("Remove Task").color(DANGER).size(11.5))
                        .clicked()
                    {
                        match crate::patcher::uninstall_startup_task() {
                            Ok(_) => {
                                self.onboarding_health.last_action_message =
                                    Some(("Startup task removed.".into(), false));
                                self.onboarding_health.refresh();
                            }
                            Err(e) => {
                                self.onboarding_health.last_action_message =
                                    Some((format!("Remove failed: {}", e), true));
                            }
                        }
                    }
                }
            });
        });

        ui.add_space(12.0);

        // Card 2: G-Helper Integration
        egui::Frame {
            fill: SURFACE,
            rounding: Rounding::same(6.0),
            stroke: Stroke::new(1.0_f32, BORDER),
            inner_margin: Margin::same(14.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("G-Helper Integration")
                    .color(TEXT_PRIMARY)
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(6.0);

            match self.onboarding_health.ghelper_status.clone() {
                GHelperPatchStatus::Patched(path) => {
                    ui.horizontal(|ui| {
                        render_status_pill(ui, "Ripple Mode Patched", SUCCESS);
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .color(TEXT_MUTED)
                                .monospace()
                                .size(10.5),
                        );
                    });
                    ui.add_space(8.0);
                    if ui
                        .button(
                            egui::RichText::new("Restore Original G-Helper")
                                .color(TEXT_SECONDARY)
                                .size(11.5),
                        )
                        .clicked()
                    {
                        match crate::patcher::unpatch_ghelper_file(&path) {
                            Ok(_) => {
                                self.onboarding_health.last_action_message = Some((
                                    "G-Helper restored to original binary.".into(),
                                    false,
                                ));
                                self.onboarding_health.refresh();
                            }
                            Err(e) => {
                                self.onboarding_health.last_action_message =
                                    Some((format!("Unpatch failed: {}", e), true));
                            }
                        }
                    }
                }
                GHelperPatchStatus::NeedsPatch(path) => {
                    ui.horizontal(|ui| {
                        render_status_pill(ui, "Needs Ripple Patch", WARNING);
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .color(TEXT_MUTED)
                                .monospace()
                                .size(10.5),
                        );
                    });
                    ui.add_space(8.0);
                    let patch_btn = egui::Button::new(
                        egui::RichText::new("Patch & Restart G-Helper")
                            .color(Color32::BLACK)
                            .size(12.0)
                            .strong(),
                    )
                    .fill(ACCENT_CYAN);
                    if ui.add(patch_btn).clicked() {
                        match crate::patcher::patch_ghelper_file(&path) {
                            Ok(_) => {
                                self.onboarding_health.last_action_message = Some((
                                    "G-Helper patched and restarted with Ripple mode enabled.".into(),
                                    false,
                                ));
                                self.onboarding_health.refresh();
                            }
                            Err(e) => {
                                self.onboarding_health.last_action_message =
                                    Some((format!("Failed to patch G-Helper: {}", e), true));
                            }
                        }
                    }
                }
                GHelperPatchStatus::NotFound => {
                    render_status_pill(ui, "GHelper.exe Not Found", TEXT_MUTED);
                }
                GHelperPatchStatus::Error(e) => {
                    render_status_pill(ui, &format!("Check Error: {}", e), DANGER);
                }
            }

            if let Some((msg, is_err)) = &self.onboarding_health.last_action_message {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(msg)
                        .color(if *is_err { DANGER } else { SUCCESS })
                        .size(11.5),
                );
            }
        });

        ui.add_space(12.0);

        // Card 3: Canonical Storage & Diagnostics
        egui::Frame {
            fill: SURFACE,
            rounding: Rounding::same(6.0),
            stroke: Stroke::new(1.0_f32, BORDER),
            inner_margin: Margin::same(14.0),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Configuration & Diagnostics")
                    .color(TEXT_PRIMARY)
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("Settings path: {}", self.settings_path.display()))
                    .color(TEXT_SECONDARY)
                    .monospace()
                    .size(10.5),
            );
            if !self.daemon_status.raw_response.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Last IPC: {}", self.daemon_status.raw_response))
                        .color(TEXT_MUTED)
                        .monospace()
                        .size(10.5),
                );
            }
        });
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

        // 2. Process debounced auto-apply for Customize & Brightness sliders
        if let Some(deadline) = self.pending_auto_apply {
            if Instant::now() >= deadline {
                self.save_and_apply();
            }
        }

        // 3. Post-UAC background refresh (detects newly installed elevated daemon automatically)
        if let Some(until) = self.post_uac_poll_until {
            let now = Instant::now();
            if now >= until {
                self.post_uac_poll_until = None;
            } else if self.last_uac_poll.elapsed() >= Duration::from_millis(800) {
                self.last_uac_poll = now;
                self.onboarding_health.refresh();
                if let Some(bridge) = &self.ipc_bridge {
                    let _ = bridge.tx_cmd.try_send(GuiIpcCommand::Diag);
                }
            }
        }

        // 4. Step internal simulation physics (50 FPS superposition)
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
                let scaled = scale_rgb_brightness(*c, self.config.brightness);
                self.preview_zones[z] = ((scaled[0] as u32) << 16)
                    | ((scaled[1] as u32) << 8)
                    | (scaled[2] as u32);
            }
        } else {
            let base = scale_rgb_brightness(self.config.color_base, self.config.brightness);
            let u32_base =
                ((base[0] as u32) << 16) | ((base[1] as u32) << 8) | (base[2] as u32);
            self.preview_zones = [u32_base; 8];
        }

        // 5. Render LenovoRGB-inspired 4-region Chrome: Top Header, Bottom Dock, Left Sidebar, Main Workspace
        egui::TopBottomPanel::top("top_header")
            .exact_height(64.0)
            .frame(egui::Frame {
                fill: BG_BASE,
                inner_margin: Margin::symmetric(20.0, 12.0),
                stroke: Stroke::new(1.0_f32, BORDER),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_header(ui);
            });

        egui::TopBottomPanel::bottom("bottom_footer")
            .exact_height(54.0)
            .frame(egui::Frame {
                fill: BG_ELEVATED,
                inner_margin: Margin::symmetric(20.0, 10.0),
                stroke: Stroke::new(1.0_f32, BORDER),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_footer(ui);
            });

        egui::SidePanel::left("left_sidebar")
            .exact_width(224.0)
            .resizable(false)
            .frame(egui::Frame {
                fill: BG_BASE,
                inner_margin: Margin::symmetric(16.0, 14.0),
                stroke: Stroke::new(1.0_f32, BORDER),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: BG_BASE,
                inner_margin: Margin::symmetric(20.0, 16.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_view {
                        ActiveView::AllPresets => self.render_presets_view(ui, None),
                        ActiveView::BuiltIn => self.render_presets_view(ui, Some(false)),
                        ActiveView::CustomProfiles => self.render_presets_view(ui, Some(true)),
                        ActiveView::Customize => self.render_customize_view(ui),
                        ActiveView::Settings => self.render_settings_view(ui),
                    });
            });

        // 6. Dynamic Repaint Optimization: 50 FPS during wave/auto-save/UAC polling, 2 Hz otherwise
        if active_count > 0
            || self.pending_auto_apply.is_some()
            || self.post_uac_poll_until.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(20));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }
}

/// Launches the native GUI application window.
pub fn run_gui() -> Result<(), eframe::Error> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::StationsAndDesktops::{OpenDesktopW, SetThreadDesktop};
        let default_name: Vec<u16> = "Default\0".encode_utf16().collect();
        let hdesk = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x1FF);
        if hdesk != 0 as _ {
            SetThreadDesktop(hdesk);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 620.0])
            .with_min_inner_size([780.0, 520.0])
            .with_title("AuraZoneRipple — Aura Control Center"),
        ..Default::default()
    };

    eframe::run_native(
        "AuraZoneRipple",
        native_options,
        Box::new(|cc| {
            theme::apply_minimalist_theme(&cc.egui_ctx);
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
        assert_eq!(app.active_view, ActiveView::AllPresets);
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

    #[test]
    fn test_onboarding_health_initialization() {
        let health = OnboardingHealth::new();
        let _ = health.needs_attention();
        assert!(matches!(
            health.ghelper_status,
            GHelperPatchStatus::Patched(_)
                | GHelperPatchStatus::NeedsPatch(_)
                | GHelperPatchStatus::NotFound
                | GHelperPatchStatus::Error(_)
        ));
    }
}
