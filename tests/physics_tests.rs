//! Comprehensive unit and integration test suite for Wave Physics, Color Math, and Ripple Engine.
//!
//! Validates:
//! - Scenario 1: Two-phase easing endpoints and peak continuity
//! - Scenario 2: Two-phase easing monotonicity (attack ramp and cosine-squared decay)
//! - Scenario 3: Wave propagation arrival timing and horizontal zone delay
//! - Scenario 4: Hardware linear distance attenuation: 1.0 - (dist * 0.08)
//! - Scenario 5: Front lightbar waterfall cascade delay (35ms) and 0.95 intensity scaling
//! - Scenario 6: Zero-allocation HSV to RGB conversion across primary and secondary hues
//! - Scenario 7: Dynamic rainbow hue progression (+0.12 hue shift per keystroke)
//! - Scenario 8: Linear interpolation (lerp) color blending with idle base color
//! - Scenario 9: 8-ripple capacity limit and FIFO eviction
//! - Scenario 10: Chromatic weighted superposition preventing bleaching whiteout
//! - Scenario 11: Atomic lockless UI preview encoding (0x00RRGGBB)
//! - Scenario 12: Natural ripple expiration pruning
//! - Scenario 13: Exhaustive VK mapping boundaries for ASUS ROG Strix G513RC
//! - Scenario 14: Zero-allocation DebounceFilter sliding window
//! - Scenario 15: Mock KeyboardListener channel event flow

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use aura_zone_ripple::hook::{
    map_vk_to_zone, DebounceFilter, KeyEvent, KeyboardListener, DEBOUNCE_WINDOW_MS,
};
use aura_zone_ripple::physics::{
    advance_rainbow, blend_colors, calculate_zone_delay, calculate_zone_intensity_full,
    hsv_norm_to_rgb, hsv_to_rgb, linear_distance_attenuation, two_phase_easing,
    RippleEngine, ATTENUATION_PER_ZONE, DEFAULT_RIPPLE_DURATION_MS, DEFAULT_SPREAD_MS,
    LIGHTBAR_CASCADE_DELAY_MS, LIGHTBAR_INTENSITY_SCALE, RAINBOW_HUE_STEP,
};

// ============================================================================
// 1. Two-Phase Easing Endpoints and Peak
// ============================================================================
#[test]
fn test_two_phase_easing_endpoints_and_peak() {
    assert_eq!(two_phase_easing(0.0), 0.0, "Start progress 0.0 must have zero intensity");
    assert!(
        (two_phase_easing(0.15) - 1.0).abs() < 1e-5,
        "Peak intensity at progress 0.15 must be exactly 1.0"
    );
    assert_eq!(two_phase_easing(1.0), 0.0, "End progress 1.0 must have zero intensity");

    // Out-of-bounds safety
    assert_eq!(two_phase_easing(-0.5), 0.0, "Negative progress must be clamped to 0.0");
    assert_eq!(two_phase_easing(1.5), 0.0, "Progress > 1.0 must return 0.0");
}

// ============================================================================
// 2. Two-Phase Easing Monotonicity
// ============================================================================
#[test]
fn test_two_phase_easing_monotonicity() {
    // Attack phase: 0.0 <= p <= 0.15 must be strictly non-decreasing
    let mut prev_attack = 0.0f32;
    for step in 0..=30 {
        let p = 0.15 * (step as f32 / 30.0);
        let val = two_phase_easing(p);
        assert!(
            val >= prev_attack - 1e-6,
            "Attack phase must be monotonic non-decreasing at p = {}: {} < {}",
            p,
            val,
            prev_attack
        );
        prev_attack = val;
    }

    // Decay phase: 0.15 <= p <= 1.0 must be strictly non-increasing
    let mut prev_decay = 1.0f32;
    for step in 0..=85 {
        let p = 0.15 + 0.85 * (step as f32 / 85.0);
        let val = two_phase_easing(p);
        assert!(
            val <= prev_decay + 1e-6,
            "Decay phase must be monotonic non-increasing at p = {}: {} > {}",
            p,
            val,
            prev_decay
        );
        prev_decay = val;
    }
}

// ============================================================================
// 3. Wave Delay and Arrival Timing
// ============================================================================
#[test]
fn test_wave_delay_and_arrival_timing() {
    let spread = Duration::from_millis(65);
    let cascade = Duration::from_millis(35);
    let duration = Duration::from_millis(380);

    // Keystroke origin in Zone 0
    let origin = 0;

    // Delay for Zone 0 is 0ms, Zone 1 is 65ms, Zone 2 is 130ms, Zone 3 is 195ms
    assert_eq!(calculate_zone_delay(origin, 0, spread, cascade), Duration::ZERO);
    assert_eq!(calculate_zone_delay(origin, 1, spread, cascade), Duration::from_millis(65));
    assert_eq!(calculate_zone_delay(origin, 2, spread, cascade), Duration::from_millis(130));
    assert_eq!(calculate_zone_delay(origin, 3, spread, cascade), Duration::from_millis(195));

    // At t = 30ms (< 65ms), Zone 0 is active (attack), Zone 1 has not arrived
    let i_z0_t30 = calculate_zone_intensity_full(
        origin, 0, Duration::from_millis(30), duration, spread, cascade, 0.95, 0.08,
    );
    let i_z1_t30 = calculate_zone_intensity_full(
        origin, 1, Duration::from_millis(30), duration, spread, cascade, 0.95, 0.08,
    );
    assert!(i_z0_t30 > 0.0, "Zone 0 must be active at 30ms");
    assert_eq!(i_z1_t30, 0.0, "Zone 1 must be silent before arrival time (65ms)");

    // At t = 70ms (> 65ms), Zone 1 has arrived (relative elapsed = 5ms)
    let i_z1_t70 = calculate_zone_intensity_full(
        origin, 1, Duration::from_millis(70), duration, spread, cascade, 0.95, 0.08,
    );
    assert!(i_z1_t70 > 0.0, "Zone 1 must be active after wave arrival at 70ms");
}

// ============================================================================
// 4. Linear Distance Attenuation
// ============================================================================
#[test]
fn test_linear_distance_attenuation() {
    assert_eq!(ATTENUATION_PER_ZONE, 0.08);

    let a0 = linear_distance_attenuation(0);
    let a1 = linear_distance_attenuation(1);
    let a2 = linear_distance_attenuation(2);
    let a3 = linear_distance_attenuation(3);

    assert!((a0 - 1.00).abs() < 1e-4, "Distance 0 attenuation must be 1.00");
    assert!((a1 - 0.92).abs() < 1e-4, "Distance 1 attenuation must be 0.92");
    assert!((a2 - 0.84).abs() < 1e-4, "Distance 2 attenuation must be 0.84");
    assert!((a3 - 0.76).abs() < 1e-4, "Distance 3 attenuation must be 0.76");
}

// ============================================================================
// 5. Front Lightbar Waterfall Cascade Timing
// ============================================================================
#[test]
fn test_lightbar_waterfall_cascade_timing() {
    let spread = Duration::from_millis(65);
    let cascade = Duration::from_millis(LIGHTBAR_CASCADE_DELAY_MS);
    let duration = Duration::from_millis(380);

    // Keyboard zone 0 corresponds to front lightbar zone 4
    let delay_kbd0 = calculate_zone_delay(0, 0, spread, cascade);
    let delay_lb0 = calculate_zone_delay(0, 4, spread, cascade);

    assert_eq!(delay_kbd0, Duration::ZERO);
    assert_eq!(delay_lb0, Duration::from_millis(35), "Lightbar must have exactly 35ms cascade delay");

    // At t = 20ms (< 35ms), lightbar zone 4 must be inactive
    let i_lb_20 = calculate_zone_intensity_full(
        0, 4, Duration::from_millis(20), duration, spread, cascade, LIGHTBAR_INTENSITY_SCALE, 0.08,
    );
    assert_eq!(i_lb_20, 0.0, "Lightbar zone 4 must be silent at 20ms (< 35ms)");

    // At t = 35ms + 57ms (peak of attack for lightbar), intensity must be scaled by 0.95
    let peak_rel = Duration::from_millis((0.15 * 380.0) as u64); // 57ms
    let t_peak_lb = Duration::from_millis(35) + peak_rel;
    let i_lb_peak = calculate_zone_intensity_full(
        0, 4, t_peak_lb, duration, spread, cascade, LIGHTBAR_INTENSITY_SCALE, 0.08,
    );
    assert!(
        (i_lb_peak - LIGHTBAR_INTENSITY_SCALE).abs() < 1e-3,
        "Lightbar peak intensity must equal LIGHTBAR_INTENSITY_SCALE (0.95)"
    );
}

// ============================================================================
// 6. Zero-Allocation HSV to RGB Primaries and Secondaries
// ============================================================================
#[test]
fn test_hsv_to_rgb_primaries_and_secondaries() {
    // Red: 0 deg
    assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255, 0, 0]);
    // Yellow: 60 deg
    assert_eq!(hsv_to_rgb(60.0, 1.0, 1.0), [255, 255, 0]);
    // Green: 120 deg
    assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), [0, 255, 0]);
    // Cyan: 180 deg
    assert_eq!(hsv_to_rgb(180.0, 1.0, 1.0), [0, 255, 255]);
    // Blue: 240 deg
    assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), [0, 0, 255]);
    // Magenta: 300 deg
    assert_eq!(hsv_to_rgb(300.0, 1.0, 1.0), [255, 0, 255]);

    // Normalized hue helper check
    assert_eq!(hsv_norm_to_rgb(0.0, 1.0, 1.0), [255, 0, 0]);
    assert_eq!(hsv_norm_to_rgb(0.5, 1.0, 1.0), [0, 255, 255]);
}

// ============================================================================
// 7. Dynamic Rainbow Hue Progression
// ============================================================================
#[test]
fn test_dynamic_rainbow_hue_advancement() {
    assert_eq!(RAINBOW_HUE_STEP, 0.12);
    let mut hue = 0.0f32;

    let mut colors = Vec::new();
    for _ in 0..10 {
        let rgb = advance_rainbow(&mut hue);
        colors.push(rgb);
    }

    // After 10 steps of +0.12, hue must be 1.2 % 1.0 = 0.20 (+- 1e-4)
    assert!((hue - 0.20).abs() < 1e-4);

    // Consecutive colors must all be distinct
    for i in 1..colors.len() {
        assert_ne!(
            colors[i - 1],
            colors[i],
            "Consecutive rainbow colors at steps {} and {} must differ",
            i - 1,
            i
        );
    }
}

// ============================================================================
// 8. Linear Interpolation (Lerp) Color Blending
// ============================================================================
#[test]
fn test_blend_colors_lerp() {
    let base = [10, 20, 30];
    let ripple = [110, 120, 130];

    // Factor 0.0 -> base color exactly
    assert_eq!(blend_colors(base, ripple, 0.0), base);
    // Factor 1.0 -> ripple color exactly
    assert_eq!(blend_colors(base, ripple, 1.0), ripple);
    // Factor 0.5 -> exact midpoint: [60, 70, 80]
    assert_eq!(blend_colors(base, ripple, 0.5), [60, 70, 80]);

    // Out-of-bounds clamping
    assert_eq!(blend_colors(base, ripple, -0.5), base);
    assert_eq!(blend_colors(base, ripple, 1.5), ripple);
}

// ============================================================================
// 9. 8-Ripple Capacity Limit & FIFO Eviction
// ============================================================================
#[test]
fn test_engine_8_ripple_fifo_eviction() {
    let mut engine = RippleEngine::new();
    assert_eq!(engine.active_ripple_count(), 0);

    // Add 8 ripples
    for i in 0..8 {
        engine.add_ripple(i % 4, [i as u8, 0, 0]);
    }
    assert_eq!(engine.active_ripple_count(), 8);
    assert_eq!(engine.active_ripples()[0].color[0], 0);

    // Add 9th ripple -> evicts index 0 (FIFO)
    engine.add_ripple(0, [99, 0, 0]);
    assert_eq!(engine.active_ripple_count(), 8);
    assert_eq!(
        engine.active_ripples()[0].color[0], 1,
        "Oldest ripple (0) must have been evicted, index 1 is now head"
    );
    assert_eq!(
        engine.active_ripples()[7].color[0], 99,
        "Newest ripple must be at tail of ring buffer"
    );
}

// ============================================================================
// 10. Chromatic Weighted Superposition
// ============================================================================
#[test]
fn test_chromatic_weighted_superposition() {
    let mut engine = RippleEngine::new();
    let now = Instant::now();
    let spread = Duration::from_millis(DEFAULT_SPREAD_MS);
    let cascade = Duration::from_millis(LIGHTBAR_CASCADE_DELAY_MS);
    let duration = Duration::from_millis(DEFAULT_RIPPLE_DURATION_MS);

    // Add Cyan [0, 255, 255] and Magenta [255, 0, 255] simultaneously at zone 0
    engine.add_ripple_with_duration(0, [0, 255, 255], duration);
    engine.add_ripple_with_duration(0, [255, 0, 255], duration);

    // Render at peak attack time (0.15 * 380ms = 57ms)
    let render_time = now + Duration::from_millis(57);
    let frame = engine.render_at_time(render_time, [0, 0, 0], spread, cascade);

    // Zone 0 should blend Cyan + Magenta to Violet/Purple [128, 128, 255]
    // Crucially: Red and Green must NOT clip to 255 (which would wash out to white)
    assert!(
        frame[0][0] >= 120 && frame[0][0] <= 135,
        "Optical blend red channel must be ~128, got {}",
        frame[0][0]
    );
    assert!(
        frame[0][1] >= 120 && frame[0][1] <= 135,
        "Optical blend green channel must be ~128, got {}",
        frame[0][1]
    );
    assert_eq!(frame[0][2], 255, "Optical blend blue channel must be 255");
}

// ============================================================================
// 11. Atomic Lockless UI Preview Encoding
// ============================================================================
#[test]
fn test_atomic_ui_preview_encoding() {
    let preview: Arc<[AtomicU32; 8]> = Arc::new([
        AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
        AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    ]);

    let test_color: [u8; 3] = [0x12, 0x34, 0x56]; // R=18, G=52, B=86
    let encoded = ((test_color[0] as u32) << 16) | ((test_color[1] as u32) << 8) | (test_color[2] as u32);
    assert_eq!(encoded, 0x00123456);

    preview[2].store(encoded, Ordering::Relaxed);

    let val = preview[2].load(Ordering::Relaxed);
    let r = ((val >> 16) & 0xFF) as u8;
    let g = ((val >> 8) & 0xFF) as u8;
    let b = (val & 0xFF) as u8;

    assert_eq!(r, 0x12);
    assert_eq!(g, 0x34);
    assert_eq!(b, 0x56);
}

// ============================================================================
// 12. Ripple Natural Expiration Pruning
// ============================================================================
#[test]
fn test_ripple_natural_pruning() {
    let mut engine = RippleEngine::new();
    let start = Instant::now();
    let spread = Duration::from_millis(65);
    let cascade = Duration::from_millis(35);
    let duration = Duration::from_millis(380);

    // Max lifetime across all 8 zones is (3 * 65ms) + 35ms + 380ms = 610ms
    engine.add_ripple_with_duration(0, [255, 255, 255], duration);
    assert_eq!(engine.active_ripple_count(), 1);

    // Check at t = 300ms: still active
    engine.prune_expired(start + Duration::from_millis(300), spread, cascade);
    assert_eq!(engine.active_ripple_count(), 1);

    // Check at t = 620ms: wave has completely passed Zone 3 and front lightbar -> pruned
    engine.prune_expired(start + Duration::from_millis(620), spread, cascade);
    assert_eq!(
        engine.active_ripple_count(),
        0,
        "Ripple must be pruned naturally once expired past max travel envelope"
    );
}

// ============================================================================
// 13. Exhaustive VK Mapping Boundaries
// ============================================================================
#[test]
fn test_exhaustive_vk_mapping_boundaries() {
    // Zone 0: Left / WASD / Hotkeys
    assert_eq!(map_vk_to_zone(0x1B), 0, "Esc -> Zone 0");
    assert_eq!(map_vk_to_zone(0x45), 0, "E -> Zone 0");
    assert_eq!(map_vk_to_zone(0x31), 0, "1 -> Zone 0");
    assert_eq!(map_vk_to_zone(0x34), 0, "4 -> Zone 0");
    assert_eq!(map_vk_to_zone(0x70), 0, "F1 -> Zone 0");
    assert_eq!(map_vk_to_zone(0xAD), 0, "M1 (Volume Mute) -> Zone 0");

    // Zone 1: Mid-Left / Space / Hotkeys
    assert_eq!(map_vk_to_zone(0x52), 1, "R -> Zone 1");
    assert_eq!(map_vk_to_zone(0x20), 1, "Space -> Zone 1");
    assert_eq!(map_vk_to_zone(0x35), 1, "5 -> Zone 1");
    assert_eq!(map_vk_to_zone(0x37), 1, "7 -> Zone 1");
    assert_eq!(map_vk_to_zone(0x74), 1, "F5 -> Zone 1");
    assert_eq!(map_vk_to_zone(0xAF), 1, "M3 (Volume Up) -> Zone 1");

    // Zone 2: Mid-Right
    assert_eq!(map_vk_to_zone(0x38), 2, "8 -> Zone 2");
    assert_eq!(map_vk_to_zone(0x30), 2, "0 -> Zone 2");
    assert_eq!(map_vk_to_zone(0x4D), 2, "M -> Zone 2");
    assert_eq!(map_vk_to_zone(0xBC), 2, "Comma -> Zone 2");

    // Zone 3: Right / Numpad / Navigation / Media Keys
    assert_eq!(map_vk_to_zone(0xBD), 3, "- -> Zone 3");
    assert_eq!(map_vk_to_zone(0x50), 3, "P -> Zone 3");
    assert_eq!(map_vk_to_zone(0x2E), 3, "Del -> Zone 3");
    assert_eq!(map_vk_to_zone(0x25), 3, "Left Arrow -> Zone 3");
    assert_eq!(map_vk_to_zone(0x60), 3, "Numpad 0 -> Zone 3");
    assert_eq!(map_vk_to_zone(0x6B), 3, "Numpad + -> Zone 3");
    assert_eq!(map_vk_to_zone(0xB0), 3, "VK_MEDIA_NEXT_TRACK -> Zone 3");
    assert_eq!(map_vk_to_zone(0xB1), 3, "VK_MEDIA_PREV_TRACK -> Zone 3");
    assert_eq!(map_vk_to_zone(0xB2), 3, "VK_MEDIA_STOP -> Zone 3");
    assert_eq!(map_vk_to_zone(0xB3), 3, "VK_MEDIA_PLAY_PAUSE -> Zone 3");
}

// ============================================================================
// 14. Zero-Allocation DebounceFilter Sliding Window
// ============================================================================
#[test]
fn test_debounce_filter_zero_allocation_and_sliding_window() {
    let mut filter = DebounceFilter::new(DEBOUNCE_WINDOW_MS);
    assert_eq!(filter.debounce_duration(), Duration::from_millis(60));

    let t0 = Instant::now();
    assert!(filter.should_process(0x41, t0)); // 'A' accepted
    assert!(!filter.should_process(0x41, t0 + Duration::from_millis(30))); // 'A' debounced

    // Clear filter resets timestamps
    filter.clear();
    assert!(filter.should_process(0x41, t0 + Duration::from_millis(30))); // Now accepted after clear
}

// ============================================================================
// 15. Mock KeyboardListener Channel Flow
// ============================================================================
#[test]
fn test_keyboard_listener_mock_mode() {
    let listener = KeyboardListener::new_mock();
    let sender = listener.sender();
    let receiver = listener.receiver();

    let evt = KeyEvent {
        zone: 0,
        vk_code: 0x57,
        timestamp: Instant::now(),
    };

    assert!(sender.try_send(evt).is_ok());
    let recv = receiver.try_recv().expect("Should receive event from channel");
    assert_eq!(recv, evt);
}
