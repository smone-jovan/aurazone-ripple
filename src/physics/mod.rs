//! Ripple wave physics, color mathematics, and superposition engine.

pub mod color;
pub mod engine;
pub mod wave;

pub use color::{
    advance_rainbow, blend_colors, hsv_norm_to_rgb, hsv_to_rgb, saturating_add_rgb,
    RAINBOW_HUE_STEP,
};
pub use engine::{
    run_engine_loop, ActiveRipple, RippleEngine, DEFAULT_WAVE_DURATION_MS, FRAME_INTERVAL_MS,
    MAX_ACTIVE_RIPPLES,
};
pub use wave::{
    calculate_zone_delay, calculate_zone_intensity, calculate_zone_intensity_full,
    linear_distance_attenuation, two_phase_easing, ATTENUATION_FACTOR, ATTENUATION_PER_ZONE,
    DEFAULT_RIPPLE_DURATION_MS, DEFAULT_SPREAD_MS, LIGHTBAR_CASCADE_DELAY_MS,
    LIGHTBAR_INTENSITY_SCALE,
};
