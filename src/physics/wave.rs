//! Two-phase wave propagation physics, easing functions, and distance attenuation.

use std::time::Duration;

pub const LIGHTBAR_CASCADE_DELAY_MS: u64 = 35;
pub const LIGHTBAR_INTENSITY_SCALE: f32 = 0.95;
pub const ATTENUATION_PER_ZONE: f32 = 0.08;
pub const ATTENUATION_FACTOR: f32 = 0.65; // Retained for backward compatibility
pub const DEFAULT_SPREAD_MS: u64 = 65;
pub const DEFAULT_RIPPLE_DURATION_MS: u64 = 380;

/// Pure mathematical two-phase easing curve for progress in [0.0, 1.0].
/// - Phase 1 (0.00 <= p < 0.15): Sine attack ramp from 0.0 to 1.0.
/// - Phase 2 (0.15 <= p <= 1.00): Cosine-squared ease-out decay from 1.0 to 0.0.
#[inline]
pub fn two_phase_easing(progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);
    if p <= 0.0 {
        0.0
    } else if p < 0.15 {
        let t = p / 0.15;
        (t * std::f32::consts::FRAC_PI_2).sin()
    } else if p < 1.0 {
        let t = (p - 0.15) / 0.85;
        let c = (t * std::f32::consts::FRAC_PI_2).cos();
        c * c
    } else {
        0.0
    }
}

/// Calculate wave propagation delay for a given zone (0..7) relative to origin zone (0..3).
#[inline]
pub fn calculate_zone_delay(
    origin_zone: usize,
    target_zone: usize,
    spread: Duration,
    cascade_delay: Duration,
) -> Duration {
    let (is_lightbar, eff_kbd_zone) = if target_zone >= 4 {
        (true, target_zone - 4)
    } else {
        (false, target_zone)
    };
    let dist = (eff_kbd_zone as isize - (origin_zone.min(3)) as isize).abs() as u32;
    let base_delay = spread * dist;
    if is_lightbar {
        base_delay + cascade_delay
    } else {
        base_delay
    }
}

/// Linear distance attenuation factor for hardware ROG Strix keyboard: 1.0 - (dist * 0.08).
#[inline]
pub fn linear_distance_attenuation(dist: usize) -> f32 {
    (1.0 - (dist as f32 * ATTENUATION_PER_ZONE)).max(0.0)
}

/// Calculate the instantaneous intensity of a single ripple on a target zone (0..7).
///
/// Retains backward-compatible exponential attenuation and cascade delay for existing opaque-box harnesses.
#[inline]
pub fn calculate_zone_intensity(
    origin_zone: usize,
    target_zone: usize,
    elapsed: Duration,
    total_duration: Duration,
) -> f32 {
    let (is_lightbar, eff_target_zone) = if target_zone >= 4 {
        (true, target_zone - 4)
    } else {
        (false, target_zone)
    };

    let effective_elapsed = if is_lightbar {
        let delay = Duration::from_millis(LIGHTBAR_CASCADE_DELAY_MS);
        if elapsed < delay {
            return 0.0;
        }
        elapsed - delay
    } else {
        elapsed
    };

    let progress = effective_elapsed.as_secs_f32() / total_duration.as_secs_f32();
    if progress >= 1.0 {
        return 0.0;
    }

    let base_intensity = two_phase_easing(progress);
    let distance = (origin_zone as isize - eff_target_zone as isize).abs() as usize;
    let attenuation = ATTENUATION_FACTOR.powi(distance as i32);
    let scale = if is_lightbar { LIGHTBAR_INTENSITY_SCALE } else { 1.0 };
    base_intensity * attenuation * scale
}

/// Comprehensive physical wave propagation calculation supporting zone delay, lightbar cascade,
/// and linear distance attenuation.
pub fn calculate_zone_intensity_full(
    origin_zone: usize,
    target_zone: usize,
    elapsed: Duration,
    total_duration: Duration,
    spread: Duration,
    cascade_delay: Duration,
    lightbar_scale: f32,
    attenuation_rate: f32,
) -> f32 {
    let (is_lightbar, eff_kbd_zone) = if target_zone >= 4 {
        (true, target_zone - 4)
    } else {
        (false, target_zone)
    };

    let dist = (eff_kbd_zone as isize - (origin_zone.min(3)) as isize).abs() as usize;
    let delay = if is_lightbar {
        spread * (dist as u32) + cascade_delay
    } else {
        spread * (dist as u32)
    };

    if elapsed < delay {
        return 0.0;
    }

    let rel_time = elapsed - delay;
    if rel_time > total_duration || total_duration.is_zero() {
        return 0.0;
    }

    let progress = rel_time.as_secs_f32() / total_duration.as_secs_f32();
    let base_intensity = two_phase_easing(progress);
    let attenuation = (1.0 - (dist as f32 * attenuation_rate)).max(0.0);
    let scale = if is_lightbar { lightbar_scale } else { 1.0 };

    base_intensity * attenuation * scale
}
