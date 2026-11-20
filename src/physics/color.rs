//! Color space conversion (HSV to RGB), alpha blending, and saturating channel arithmetic.

pub const RAINBOW_HUE_STEP: f32 = 0.12;

/// Zero-allocation HSV to RGB conversion where `h` is in degrees [0.0, 360.0).
#[inline]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = (h % 360.0 + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Zero-allocation HSV to RGB conversion where `h_norm` is normalised in [0.0, 1.0).
#[inline]
pub fn hsv_norm_to_rgb(h_norm: f32, s: f32, v: f32) -> [u8; 3] {
    let h_deg = (h_norm.fract() + 1.0).fract() * 360.0;
    hsv_to_rgb(h_deg, s, v)
}

/// Advances rainbow hue by 0.12 (normalized) and returns the updated RGB color.
#[inline]
pub fn advance_rainbow(current_hue: &mut f32) -> [u8; 3] {
    *current_hue = (*current_hue + RAINBOW_HUE_STEP).fract();
    if *current_hue < 0.0 {
        *current_hue += 1.0;
    }
    hsv_norm_to_rgb(*current_hue, 1.0, 1.0)
}

/// Advances rainbow hue by `step_deg` degrees and returns the updated RGB color.
#[inline]
pub fn advance_rainbow_by(current_hue: &mut f32, step_deg: f32) -> [u8; 3] {
    let norm_step = step_deg / 360.0;
    *current_hue = (*current_hue + norm_step).fract();
    if *current_hue < 0.0 {
        *current_hue += 1.0;
    }
    hsv_norm_to_rgb(*current_hue, 1.0, 1.0)
}

/// Scales an RGB triplet by brightness percentage (`0..=100`).
#[inline]
pub fn scale_rgb_brightness(rgb: [u8; 3], brightness_pct: u8) -> [u8; 3] {
    if brightness_pct >= 100 {
        return rgb;
    }
    if brightness_pct == 0 {
        return [0, 0, 0];
    }
    let factor = brightness_pct as f32 / 100.0;
    [
        (rgb[0] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 * factor).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Linear interpolation (lerp) blending ripple color over idle base color.
#[inline]
pub fn blend_colors(base: [u8; 3], ripple: [u8; 3], intensity: f32) -> [u8; 3] {
    let factor = intensity.clamp(0.0, 1.0);
    [
        (base[0] as f32 * (1.0 - factor) + ripple[0] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (base[1] as f32 * (1.0 - factor) + ripple[1] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (base[2] as f32 * (1.0 - factor) + ripple[2] as f32 * factor).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Clamping channel-wise addition of two RGB colors.
#[inline]
pub fn saturating_add_rgb(c1: [u8; 3], c2: [u8; 3]) -> [u8; 3] {
    [
        c1[0].saturating_add(c2[0]),
        c1[1].saturating_add(c2[1]),
        c1[2].saturating_add(c2[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_rgb_brightness() {
        assert_eq!(scale_rgb_brightness([200, 100, 50], 100), [200, 100, 50]);
        assert_eq!(scale_rgb_brightness([200, 100, 50], 0), [0, 0, 0]);
        assert_eq!(scale_rgb_brightness([200, 100, 50], 50), [100, 50, 25]);
    }

    #[test]
    fn test_advance_rainbow_by() {
        let mut hue = 0.0f32;
        let c1 = advance_rainbow_by(&mut hue, 90.0);
        assert!((hue - 0.25).abs() < 1e-5);
        assert_ne!(c1, [255, 0, 0]);
    }
}
