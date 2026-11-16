//! Virtual keycode mapping table and per-key debounce filtering for ASUS ROG keyboards.
//!
//! Specifically tailored for the 4-zone Aura RGB layout of ASUS ROG Strix G513RC:
//! - Zone 0: Left / WASD / Esc / F1-F4 / Caps / Tab / Shift / LCtrl / 1-4 / M1-M2
//! - Zone 1: Mid-Left / R-T-Y / F-G-H / V-B-N / 5-7 / F5-F7 / Space (left) / M3
//! - Zone 2: Mid-Right / U-I-O / J-K-L / M / 8-0 / F8-F10 / Comma / RAlt / M4
//! - Zone 3: Right / Numpad / Enter / Backspace / Arrows / Del / F11-F12 / Nav

use std::time::{Duration, Instant};

/// Standard debounce window for keyboard input filtering (60 milliseconds).
pub const DEBOUNCE_WINDOW_MS: u64 = 60;

/// Represents a captured and filtered keyboard input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// Target keyboard lighting zone (0..=3).
    pub zone: usize,
    /// Win32 virtual keycode.
    pub vk_code: u32,
    /// Timestamp when the key event was captured.
    pub timestamp: Instant,
}

/// Compile-time evaluation mapping an 8-bit Virtual Keycode (0..=255) to a keyboard zone (0..=3).
const fn vk_to_zone_const(vk: u8) -> u8 {
    match vk {
        // ====================================================================
        // Zone 0: Left / WASD / Esc / F1-F4 / Caps / Tab / Shift / LCtrl / 1-4
        // ====================================================================
        0x1B // VK_ESCAPE
        | 0xC0 // VK_OEM_3 (`~)
        | 0x31..=0x34 // '1'..='4'
        | 0x70..=0x73 // VK_F1..=VK_F4
        | 0x09 // VK_TAB
        | 0x51 | 0x57 | 0x45 // 'Q', 'W', 'E'
        | 0x14 // VK_CAPITAL (Caps Lock)
        | 0x41 | 0x53 | 0x44 // 'A', 'S', 'D'
        | 0xA0 | 0x10 // VK_LSHIFT, VK_SHIFT
        | 0x5A | 0x58 | 0x43 // 'Z', 'X', 'C'
        | 0xA2 | 0x11 // VK_LCONTROL, VK_CONTROL
        | 0x5B // VK_LWIN
        | 0xA4 | 0x12 // VK_LMENU, VK_MENU (LAlt)
        | 0xAD | 0xAE // VK_VOLUME_MUTE, VK_VOLUME_DOWN (M1, M2 hotkeys)
        | 0xE2 // VK_OEM_102 (<> on ISO keyboards)
        => 0,

        // ====================================================================
        // Zone 1: Mid-Left / R-T-Y / F-G-H / V-B-N / 5-7 / F5-F7 / Space / M3
        // ====================================================================
        0x74..=0x76 // VK_F5..=VK_F7
        | 0x35..=0x37 // '5'..='7'
        | 0x52 | 0x54 | 0x59 // 'R', 'T', 'Y'
        | 0x46 | 0x47 | 0x48 // 'F', 'G', 'H'
        | 0x56 | 0x42 | 0x4E // 'V', 'B', 'N'
        | 0x20 // VK_SPACE (Left-hand strike center)
        | 0xAF // VK_VOLUME_UP (M3 hotkey)
        => 1,

        // ====================================================================
        // Zone 2: Mid-Right / U-I-O / J-K-L / M / 8-0 / F8-F10 / Comma / RAlt
        // ====================================================================
        0x77..=0x79 // VK_F8..=VK_F10
        | 0x38..=0x39 | 0x30 // '8', '9', '0'
        | 0x55 | 0x49 | 0x4F // 'U', 'I', 'O'
        | 0x4A | 0x4B | 0x4C // 'J', 'K', 'L'
        | 0x4D // 'M'
        | 0xBC // VK_OEM_COMMA (,<)
        | 0xA5 // VK_RMENU (RAlt / AltGr)
        => 2,

        // ====================================================================
        // Zone 3: Right / Numpad / Enter / Backspace / Arrows / Nav / Del / Media Keys
        // ====================================================================
        0x7A..=0x7B // VK_F11..=VK_F12
        | 0xBD // VK_OEM_MINUS (-_)
        | 0xBB // VK_OEM_PLUS (=+)
        | 0x08 // VK_BACK (Backspace)
        | 0x50 // 'P'
        | 0xDB | 0xDD | 0xDC // VK_OEM_4, VK_OEM_6, VK_OEM_5 ([, ], \)
        | 0xBA | 0xDE // VK_OEM_1, VK_OEM_7 (;:, '")
        | 0x0D // VK_RETURN (Enter)
        | 0xBE | 0xBF // VK_OEM_PERIOD, VK_OEM_2 (.>, /?)
        | 0xA1 // VK_RSHIFT
        | 0x5C | 0x5D | 0xA3 // VK_RWIN, VK_APPS, VK_RCONTROL
        | 0x25..=0x28 // VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN
        | 0x21..=0x24 | 0x2D | 0x2E // VK_PRIOR, VK_NEXT, VK_END, VK_HOME, VK_INSERT, VK_DELETE
        | 0x2C | 0x13 | 0x91 // VK_SNAPSHOT (PrtSc), VK_PAUSE, VK_SCROLL
        | 0x90 // VK_NUMLOCK
        | 0x60..=0x6F // VK_NUMPAD0..=VK_NUMPAD9, Multiply, Add, Separator, Subtract, Decimal, Divide
        | 0xB0..=0xB3 // VK_MEDIA_NEXT_TRACK (0xB0), VK_MEDIA_PREV_TRACK (0xB1), VK_MEDIA_STOP (0xB2), VK_MEDIA_PLAY_PAUSE (0xB3)
        => 3,

        // Fallback for unmapped keys: interior Zone 1
        _ => 1,
    }
}

/// Static 256-byte compile-time lookup table placed in `.rdata`.
const fn build_vk_zone_lut() -> [u8; 256] {
    let mut lut = [1u8; 256];
    let mut i = 0;
    while i < 256 {
        lut[i] = vk_to_zone_const(i as u8);
        i += 1;
    }
    lut
}

pub static VK_ZONE_LUT: [u8; 256] = build_vk_zone_lut();

/// Maps a Windows Virtual Keycode to one of the 4 horizontal keyboard zones (0..=3).
///
/// Executes in O(1) single-instruction lookup with zero heap allocations and zero branching
/// for all standard Win32 keycodes (0..=255).
#[inline]
pub fn map_vk_to_zone(vk_code: u32) -> usize {
    if (vk_code as usize) < 256 {
        VK_ZONE_LUT[vk_code as usize] as usize
    } else {
        1 // Safe interior fallback for out-of-range synthetic codes
    }
}

/// High-performance zero-allocation per-key sliding window debounce filter.
///
/// Uses an inline 256-element array of `Option<Instant>` (4096 bytes total),
/// completely eliminating heap allocations, hashing overhead, and pointer indirection.
#[derive(Debug, Clone)]
pub struct DebounceFilter {
    last_press_times: [Option<Instant>; 256],
    debounce_duration: Duration,
}

impl Default for DebounceFilter {
    fn default() -> Self {
        Self::new(DEBOUNCE_WINDOW_MS)
    }
}

impl DebounceFilter {
    /// Creates a new `DebounceFilter` with the specified debounce duration in milliseconds.
    pub const fn new(debounce_ms: u64) -> Self {
        Self {
            last_press_times: [None; 256],
            debounce_duration: Duration::from_millis(debounce_ms),
        }
    }

    /// Evaluates whether a virtual keypress event should be processed or filtered out.
    ///
    /// Returns `true` if the key has not been pressed within the debounce window,
    /// recording the timestamp and allowing the event. Returns `false` if debounced.
    #[inline]
    pub fn should_process(&mut self, vk_code: u32, now: Instant) -> bool {
        if (vk_code as usize) < self.last_press_times.len() {
            let idx = vk_code as usize;
            if let Some(last_time) = self.last_press_times[idx] {
                if let Some(elapsed) = now.checked_duration_since(last_time) {
                    if elapsed < self.debounce_duration {
                        return false;
                    }
                }
            }
            self.last_press_times[idx] = Some(now);
            true
        } else {
            // Out-of-range fallback: allow processing
            true
        }
    }

    /// Clears all recorded keypress timestamps.
    #[inline]
    pub fn clear(&mut self) {
        self.last_press_times = [None; 256];
    }

    /// Returns the current debounce window duration.
    #[inline]
    pub fn debounce_duration(&self) -> Duration {
        self.debounce_duration
    }

    /// Updates the debounce window duration.
    #[inline]
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }
}
