//! Ripple engine rendering concurrent superposition waves at 50 FPS.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use crate::hid::{AuraDevice, AuraError};
use crate::hook::KeyEvent;
use super::wave::{
    calculate_zone_intensity, calculate_zone_intensity_full, ATTENUATION_PER_ZONE,
    DEFAULT_RIPPLE_DURATION_MS, LIGHTBAR_CASCADE_DELAY_MS, LIGHTBAR_INTENSITY_SCALE,
};
use super::color::{advance_rainbow, blend_colors, saturating_add_rgb};

pub const MAX_ACTIVE_RIPPLES: usize = 8;
pub const DEFAULT_WAVE_DURATION_MS: u64 = DEFAULT_RIPPLE_DURATION_MS;
pub const FRAME_INTERVAL_MS: u64 = 20; // 50 FPS

/// Represents an active ripple propagating across keyboard and lightbar zones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveRipple {
    pub origin_zone: usize,
    pub color: [u8; 3],
    pub start_time: Instant,
    pub duration: Duration,
}

/// Core wave simulation and superposition engine.
#[derive(Debug, Clone)]
pub struct RippleEngine {
    ripples: Vec<ActiveRipple>,
    max_ripples: usize,
}

impl Default for RippleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RippleEngine {
    /// Creates a new `RippleEngine` with capacity capped at `MAX_ACTIVE_RIPPLES` (8).
    pub fn new() -> Self {
        Self {
            ripples: Vec::with_capacity(MAX_ACTIVE_RIPPLES),
            max_ripples: MAX_ACTIVE_RIPPLES,
        }
    }

    /// Adds a new ripple with default duration. Evicts oldest ripple if at capacity (FIFO).
    pub fn add_ripple(&mut self, origin_zone: usize, color: [u8; 3]) {
        self.add_ripple_with_duration(
            origin_zone,
            color,
            Duration::from_millis(DEFAULT_WAVE_DURATION_MS),
        );
    }

    /// Adds a new ripple with specific duration. Evicts oldest ripple if at capacity (FIFO).
    pub fn add_ripple_with_duration(
        &mut self,
        origin_zone: usize,
        color: [u8; 3],
        duration: Duration,
    ) {
        if self.ripples.len() >= self.max_ripples {
            self.ripples.remove(0); // FIFO eviction
        }
        self.ripples.push(ActiveRipple {
            origin_zone: origin_zone.min(3),
            color,
            start_time: Instant::now(),
            duration,
        });
    }

    /// Returns the number of currently active ripples.
    pub fn active_ripple_count(&self) -> usize {
        self.ripples.len()
    }

    /// Returns a slice of active ripples.
    pub fn active_ripples(&self) -> &[ActiveRipple] {
        &self.ripples
    }

    /// Prunes ripples whose wave propagation and envelope have completely elapsed across all zones.
    pub fn prune_expired(&mut self, now: Instant, spread: Duration, cascade: Duration) {
        self.ripples.retain(|r| {
            let max_delay = spread * 3 + cascade;
            let max_lifetime = max_delay + r.duration;
            now.duration_since(r.start_time) <= max_lifetime
        });
    }

    /// Superposition render at a specific point in time using optical weighted chromatic blending.
    pub fn render_at_time(
        &self,
        now: Instant,
        base_color: [u8; 3],
        spread: Duration,
        cascade: Duration,
    ) -> [[u8; 3]; 8] {
        let mut zone_intensities = [0.0f32; 8];
        let mut zone_color_sums = [[0.0f32; 3]; 8];

        for ripple in &self.ripples {
            let age = now.saturating_duration_since(ripple.start_time);
            for z in 0..8 {
                let intensity = calculate_zone_intensity_full(
                    ripple.origin_zone,
                    z,
                    age,
                    ripple.duration,
                    spread,
                    cascade,
                    LIGHTBAR_INTENSITY_SCALE,
                    ATTENUATION_PER_ZONE,
                );
                if intensity > 0.0 {
                    zone_intensities[z] += intensity;
                    for c in 0..3 {
                        zone_color_sums[z][c] += ripple.color[c] as f32 * intensity;
                    }
                }
            }
        }

        let mut output = [base_color; 8];
        for z in 0..8 {
            let net_i = zone_intensities[z];
            if net_i > 0.0 {
                let avg_color = [
                    (zone_color_sums[z][0] / net_i).round().clamp(0.0, 255.0) as u8,
                    (zone_color_sums[z][1] / net_i).round().clamp(0.0, 255.0) as u8,
                    (zone_color_sums[z][2] / net_i).round().clamp(0.0, 255.0) as u8,
                ];
                output[z] = blend_colors(base_color, avg_color, net_i.min(1.0));
            }
        }
        output
    }

    /// Deterministic offline render frame (for backward compatibility and mock tests).
    pub fn render_frame(&self, elapsed_simulated: Duration, base_color: &[u8; 3]) -> [[u8; 3]; 8] {
        let mut frame = [*base_color; 8];
        for ripple in &self.ripples {
            for zone_idx in 0..8 {
                let intensity = calculate_zone_intensity(
                    ripple.origin_zone,
                    zone_idx,
                    elapsed_simulated,
                    ripple.duration,
                );
                if intensity > 0.0 {
                    let ripple_contribution = [
                        (ripple.color[0] as f32 * intensity).round() as u8,
                        (ripple.color[1] as f32 * intensity).round() as u8,
                        (ripple.color[2] as f32 * intensity).round() as u8,
                    ];
                    frame[zone_idx] = saturating_add_rgb(frame[zone_idx], ripple_contribution);
                }
            }
        }
        frame
    }
}

/// Runs the 50 FPS reactive loop in a dedicated worker thread.
pub fn run_engine_loop(
    mut device: Box<dyn AuraDevice>,
    rx: Receiver<KeyEvent>,
    ui_preview: Arc<[AtomicU32; 8]>,
    running: Arc<AtomicBool>,
    base_color: [u8; 3],
    default_ripple_color: [u8; 3],
    rainbow_mode: bool,
    spread: Duration,
    duration: Duration,
) -> Result<(), AuraError> {
    let mut engine = RippleEngine::new();
    let mut current_hue = 0.0f32;
    let cascade = Duration::from_millis(LIGHTBAR_CASCADE_DELAY_MS);
    let frame_interval = Duration::from_millis(FRAME_INTERVAL_MS);

    while running.load(Ordering::Relaxed) {
        let frame_start = Instant::now();

        // 1. Drain pending keystroke events from channel non-blockingly
        while let Ok(event) = rx.try_recv() {
            let color = if rainbow_mode {
                advance_rainbow(&mut current_hue)
            } else {
                default_ripple_color
            };
            engine.add_ripple_with_duration(event.zone, color, duration);
        }

        // 2. Prune expired ripples
        engine.prune_expired(frame_start, spread, cascade);

        // 3. Render current frame across 8 zones
        let zone_colors = engine.render_at_time(frame_start, base_color, spread, cascade);

        // 4. Update lockless UI preview (0x00RRGGBB)
        for (z, c) in zone_colors.iter().enumerate() {
            let u32_val = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
            ui_preview[z].store(u32_val, Ordering::Relaxed);
        }

        // 5. Dispatch to Aura Hardware / Mock
        let _ = device.apply_zones(&zone_colors);

        // 6. Sleep for remaining frame time to maintain exactly 50 FPS
        let elapsed = frame_start.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }

    Ok(())
}
