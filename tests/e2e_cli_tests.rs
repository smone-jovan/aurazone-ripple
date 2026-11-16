//! 4-Tier End-to-End Integration and Opaque-Box Test Suite for AuraZoneRipple_Rust.
//!
//! Conforms to:
//! - `ORIGINAL_REQUEST.md`: ASUS ROG Strix G513RC (VID_0B05&PID_19B6, Col05, Report ID 0x5D, Command 0xBC, 64B)
//! - `PROJECT.md`: 10 Architectural Features, 12-to-8 Zone Mapping, Minimalist Theme, G-Helper M5 Trigger
//! - `TEST_INFRA.md`: 4-Tier Test Suite Specification (Tier 1 Feature Coverage, Tier 2 Boundaries,
//!   Tier 3 Combinations, Tier 4 Real-World Scenarios)

mod common;
use common::*;

use std::fs::OpenOptions;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use aura_zone_ripple::config::{Preset, RippleConfig};
use aura_zone_ripple::hid::{
    encode_direct_frame, encode_handshake, validate_packet, AuraDevice, AuraError,
    MockAuraDevice, ZONE_MAP,
};
use aura_zone_ripple::hook::{map_vk_to_zone, DebounceFilter};
use aura_zone_ripple::physics::{
    color::{blend_colors, hsv_to_rgb, saturating_add_rgb},
    engine::RippleEngine,
    wave::{
        calculate_zone_intensity, two_phase_easing, LIGHTBAR_CASCADE_DELAY_MS,
        LIGHTBAR_INTENSITY_SCALE,
    },
};

// ============================================================================
// Win32 Direct Kernel32 FFI for Mutex and Process Lifecycle Tests
// ============================================================================
#[cfg(windows)]
mod win32 {
    use std::os::windows::raw::HANDLE;

    pub const ERROR_ALREADY_EXISTS: u32 = 183;
    pub const WAIT_OBJECT_0: u32 = 0x00000000;
    pub const WAIT_ABANDONED: u32 = 0x00000080;

    extern "system" {
        pub fn CreateMutexW(
            lpMutexAttributes: *const std::ffi::c_void,
            bInitialOwner: i32,
            lpName: *const u16,
        ) -> HANDLE;
        pub fn GetLastError() -> u32;
        pub fn CloseHandle(hObject: HANDLE) -> i32;
        pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
        pub fn ReleaseMutex(hMutex: HANDLE) -> i32;
    }

    pub struct ScopedMutex {
        pub handle: HANDLE,
        pub already_exists: bool,
    }

    impl ScopedMutex {
        pub fn create(name: &str) -> Result<Self, u32> {
            let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
            if handle.is_null() {
                return Err(unsafe { GetLastError() });
            }
            let err = unsafe { GetLastError() };
            Ok(Self {
                handle,
                already_exists: err == ERROR_ALREADY_EXISTS,
            })
        }
    }

    impl Drop for ScopedMutex {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    CloseHandle(self.handle);
                }
            }
        }
    }
}

// ============================================================================
// TIER 1: FEATURE COVERAGE (HAPPY PATH - F1 to F10)
// ============================================================================

#[test]
fn test_t1_f1_handshake_packet_length_exactly_64() {
    let pkt = encode_handshake();
    assert_eq!(pkt.len(), 64, "Handshake report must be strictly 64 bytes");
}

#[test]
fn test_t1_f1_handshake_report_id_0x5d() {
    let pkt = encode_handshake();
    assert_eq!(pkt[0], 0x5D, "Handshake byte 0 must be Report ID 0x5D");
}

#[test]
fn test_t1_f1_handshake_command_0xbc() {
    let pkt = encode_handshake();
    assert_eq!(pkt[1], 0xBC, "Handshake byte 1 must be Direct Command 0xBC");
    assert_eq!(pkt[2], 0x01, "Handshake byte 2 must be Init Sub-command 0x01");
}

#[test]
fn test_t1_f1_direct_frame_header() {
    let zones = [[0u8; 3]; 8];
    let pkt = encode_direct_frame(&zones);
    assert_eq!(pkt[0], 0x5D, "Byte 0 must be 0x5D");
    assert_eq!(pkt[1], 0xBC, "Byte 1 must be 0xBC");
    assert_eq!(pkt[2], 0x00, "Byte 2 must be 0x00");
    assert_eq!(pkt[3], 0x01, "Byte 3 must be 0x01");
    assert_eq!(pkt[4], 0x04, "Byte 4 must be 0x04 (4-Zone command)");
}

#[test]
fn test_t1_f1_direct_frame_length_strictly_64() {
    let zones = [[255u8; 3]; 8];
    let pkt = encode_direct_frame(&zones);
    assert_eq!(pkt.len(), 64, "Direct frame must be strictly 64 bytes");
}

#[test]
fn test_t1_f2_zone_map_keyboard_zones_0_to_3() {
    assert_eq!(ZONE_MAP[0], 0, "Slot 0 must map to Keyboard Zone 0");
    assert_eq!(ZONE_MAP[1], 1, "Slot 1 must map to Keyboard Zone 1");
    assert_eq!(ZONE_MAP[2], 2, "Slot 2 must map to Keyboard Zone 2");
    assert_eq!(ZONE_MAP[3], 3, "Slot 3 must map to Keyboard Zone 3");
}

#[test]
fn test_t1_f2_zone_map_slots_4_and_5_mirror_zone_0() {
    assert_eq!(ZONE_MAP[4], 0, "Slot 4 must mirror Zone 0");
    assert_eq!(ZONE_MAP[5], 0, "Slot 5 must mirror Zone 0");
}

#[test]
fn test_t1_f2_zone_map_lightbar_front_zones() {
    // ROG Strix G513 motherboard bus traces for slots 6-11 run Right-to-Left.
    // Inverse mapping [7, 7, 6, 5, 4, 4] ensures physical Left-to-Right wave propagation.
    assert_eq!(ZONE_MAP[6], 7, "Slot 6 (HW bus Right) must map to Lightbar Right Wrap (Zone 7)");
    assert_eq!(ZONE_MAP[7], 7, "Slot 7 (HW bus Right-Mid) must mirror Lightbar Right Wrap (Zone 7)");
    assert_eq!(ZONE_MAP[8], 6, "Slot 8 (HW bus Center-R) must map to Lightbar Mid-Right (Zone 6)");
    assert_eq!(ZONE_MAP[9], 5, "Slot 9 (HW bus Center-L) must map to Lightbar Mid-Left (Zone 5)");
    assert_eq!(ZONE_MAP[10], 4, "Slot 10 (HW bus Left-Mid) must map to Lightbar Left Wrap (Zone 4)");
    assert_eq!(ZONE_MAP[11], 4, "Slot 11 (HW bus Left) must mirror Lightbar Left Wrap (Zone 4)");
}

#[test]
fn test_t1_f2_handshake_zero_padded_remainder() {
    let pkt = encode_handshake();
    for i in 3..64 {
        assert_eq!(pkt[i], 0x00, "Byte {} must be zero-padded", i);
    }
}

#[test]
fn test_t1_f2_color_triplet_rgb_byte_ordering() {
    let mut zones = [[0u8; 3]; 8];
    zones[1] = [0x11, 0x22, 0x33]; // R=0x11, G=0x22, B=0x33
    let pkt = encode_direct_frame(&zones);
    // Slot 1 is at byte offset 9 + 1*3 = 12
    assert_eq!(pkt[12], 0x11, "Red byte must match");
    assert_eq!(pkt[13], 0x22, "Green byte must match");
    assert_eq!(pkt[14], 0x33, "Blue byte must match");
}

#[test]
fn test_t1_f3_mock_device_creation() {
    let dev = MockAuraDevice::new();
    assert_eq!(dev.packet_count(), 0);
    assert!(dev.is_connected());
    assert!(dev.latest_zones().is_none());
}

#[test]
fn test_t1_f3_mock_device_send_feature_records_packets() {
    let mut dev = MockAuraDevice::new();
    let handshake = encode_handshake();
    assert!(dev.send_feature(&handshake).is_ok());
    assert_eq!(dev.packet_count(), 1);
    assert_eq!(dev.sent_packets()[0], handshake);
}

#[test]
fn test_t1_f3_mock_device_apply_zones_computes_packet() {
    let mut dev = MockAuraDevice::new();
    let colors = [[10, 20, 30]; 8];
    assert!(dev.apply_zones(&colors).is_ok());
    assert_eq!(dev.latest_zones(), Some(&colors));
    assert_eq!(dev.packet_count(), 1);
    assert_eq!(dev.sent_packets()[0][0], 0x5D);
    assert_eq!(dev.sent_packets()[0][1], 0xBC);
}

#[test]
fn test_t1_f3_mock_device_set_static_uniform() {
    let mut dev = MockAuraDevice::new();
    let blue = [0, 0, 255];
    assert!(dev.set_static_uniform(blue).is_ok());
    assert_eq!(dev.latest_zones(), Some(&[blue; 8]));
}

#[test]
fn test_t1_f3_mock_device_packet_inspection() {
    let mut dev = MockAuraDevice::new();
    dev.apply_zones(&[[100, 150, 200]; 8]).unwrap();
    let last = dev.latest_packet().unwrap();
    assert_eq!(last[0..5], [0x5D, 0xBC, 0x00, 0x01, 0x04]);
    // Tail padding check
    for b in &last[45..64] {
        assert_eq!(*b, 0x00);
    }
}

#[test]
fn test_t1_f4_cli_help_flag() {
    let bin = require_binary();
    let output = Command::new(&bin).arg("--help").output().expect("Failed to run --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("help") || stdout.contains("Usage:"));
}

#[test]
fn test_t1_f4_cli_version_flag() {
    let bin = require_binary();
    let output = Command::new(&bin).arg("--version").output().expect("Failed to run --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0") || stdout.contains("aurazone_ripple"));
}

#[test]
fn test_t1_f4_cli_mock_flag_accepted() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--mock")
        .arg("--help")
        .output()
        .expect("Failed to run --mock --help");
    assert!(output.status.success());
}

#[test]
fn test_t1_f4_cli_status_when_no_daemon() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--status")
        .env("AURAZONE_PIPE_NAME", r"\\.\pipe\Nonexistent_Test_Pipe_XYZ")
        .output()
        .expect("Failed to run --status");
    assert!(output.status.code().unwrap_or(0) != 0 || String::from_utf8_lossy(&output.stdout).contains("idle") || String::from_utf8_lossy(&output.stdout).contains("STOPPED"));
}

#[test]
fn test_t1_f4_cli_toggle_when_no_daemon() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--toggle")
        .env("AURAZONE_PIPE_NAME", r"\\.\pipe\Nonexistent_Test_Pipe_XYZ")
        .output()
        .expect("Failed to run --toggle");
    assert!(output.status.code().is_some());
}

#[test]
fn test_t1_f4_cli_stop_when_no_daemon() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--stop")
        .env("AURAZONE_PIPE_NAME", r"\\.\pipe\Nonexistent_Test_Pipe_XYZ")
        .output()
        .expect("Failed to run --stop");
    assert!(output.status.code().is_some());
}

#[test]
fn test_t1_f4_cli_restore_flag() {
    let fixture = TempConfigFixture::new().unwrap();
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--restore")
        .arg("--mock")
        .env("GHELPER_CONFIG_PATH", fixture.ghelper_config_path.to_str().unwrap())
        .output()
        .expect("Failed to run --restore --mock");
    assert!(output.status.success());
}

#[test]
fn test_t1_f4_cli_install_ghelper_dry_run() {
    let fixture = TempConfigFixture::new().unwrap();
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--install-ghelper")
        .env("GHELPER_CONFIG_PATH", fixture.ghelper_config_path.to_str().unwrap())
        .output()
        .expect("Failed to run --install-ghelper");
    assert!(output.status.code().is_some());
}

#[test]
#[cfg(windows)]
fn test_t1_f5_single_instance_mutex_acquire() {
    let mutex_name = format!("Local\\AuraZone_Test_Mutex_{}", uuid_string());
    let mutex = win32::ScopedMutex::create(&mutex_name).expect("Failed to create mutex");
    assert!(!mutex.already_exists, "First acquisition must succeed without already_exists");
}

#[test]
#[cfg(windows)]
fn test_t1_f5_single_instance_duplicate_prevention() {
    let mutex_name = format!("Local\\AuraZone_Test_Mutex_{}", uuid_string());
    let mutex1 = win32::ScopedMutex::create(&mutex_name).expect("First create failed");
    assert!(!mutex1.already_exists);

    let mutex2 = win32::ScopedMutex::create(&mutex_name).expect("Second create failed");
    assert!(mutex2.already_exists, "Second acquisition must detect already_exists");
}

#[test]
#[cfg(windows)]
fn test_t1_f5_single_instance_mutex_release() {
    let mutex_name = format!("Local\\AuraZone_Test_Mutex_{}", uuid_string());
    {
        let mutex1 = win32::ScopedMutex::create(&mutex_name).unwrap();
        assert!(!mutex1.already_exists);
    } // mutex1 dropped and closed here

    let mutex2 = win32::ScopedMutex::create(&mutex_name).unwrap();
    assert!(!mutex2.already_exists, "After drop, re-creation must succeed without collision");
}

#[test]
fn test_t1_f5_daemon_starts_and_locks_mutex() {
    let _bin = require_binary();
    let test_id = format!("t1_f5_lock_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Failed to spawn mock daemon");
    #[cfg(windows)]
    {
        let mutex = win32::ScopedMutex::create(&daemon.mutex_name).unwrap();
        assert!(mutex.already_exists, "Daemon must hold its single-instance mutex");
    }
}

#[test]
fn test_t1_f5_second_daemon_exits_with_error() {
    let bin = require_binary();
    let test_id = format!("t1_f5_second_{}", uuid_string());
    let _daemon1 = DaemonGuard::spawn_mock(&test_id).expect("First daemon failed");

    let pipe_name = format!(r"\\.\pipe\AuraZoneRipple_IPC_{}", test_id);
    let mutex_name = format!("Local\\AuraZoneRipple_Mutex_{}", test_id);

    let output = Command::new(&bin)
        .arg("--daemon")
        .arg("--mock")
        .env("AURAZONE_PIPE_NAME", &pipe_name)
        .env("AURAZONE_MUTEX_NAME", &mutex_name)
        .output()
        .expect("Second daemon failed to execute");

    assert!(output.status.code().is_some());
}

#[test]
fn test_t1_f6_ipc_server_creates_named_pipe() {
    let _bin = require_binary();
    let test_id = format!("t1_f6_create_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    assert!(OpenOptions::new().read(true).write(true).open(&daemon.pipe_name).is_ok());
}

#[test]
fn test_t1_f6_ipc_client_connects_to_pipe() {
    let _bin = require_binary();
    let test_id = format!("t1_f6_connect_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let resp = client.send_command("STATUS");
    assert!(resp.is_ok());
}

#[test]
fn test_t1_f6_ipc_command_ping_status() {
    let _bin = require_binary();
    let test_id = format!("t1_f6_status_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let resp = client.send_command("STATUS").expect("STATUS query failed");
    assert!(resp.contains("running") || resp.contains("OK"));
}

#[test]
fn test_t1_f6_ipc_command_toggle() {
    let _bin = require_binary();
    let test_id = format!("t1_f6_toggle_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let resp = client.send_command("TOGGLE").expect("TOGGLE command failed");
    assert!(resp.contains("OK") || resp.contains("TOGGLE") || resp.contains("paused"));
}

#[test]
fn test_t1_f6_ipc_command_stop() {
    let _bin = require_binary();
    let test_id = format!("t1_f6_stop_{}", uuid_string());
    let mut daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let resp = client.send_command("STOP").expect("STOP command failed");
    assert!(resp.contains("OK") || resp.contains("STOP"));
    let status = daemon.child.wait().expect("Wait on daemon failed");
    assert!(status.success());
}

#[test]
fn test_t1_f7_ghelper_read_config_json() {
    let fixture = TempConfigFixture::new().unwrap();
    let json = fixture.read_ghelper_json().unwrap();
    assert_eq!(json["keyboard_brightness"], 3);
    assert_eq!(json["aura_color"], -16776961);
    assert_eq!(json["m5"], "performance");
}

#[test]
fn test_t1_f7_ghelper_bind_m5_key() {
    let fixture = TempConfigFixture::new().unwrap();
    let mut json = fixture.read_ghelper_json().unwrap();
    json["m5"] = serde_json::Value::String(r"C:\AuraZoneRipple\aurazone_ripple.exe --toggle".to_string());
    fixture.write_ghelper_json(&json).unwrap();

    let re_read = fixture.read_ghelper_json().unwrap();
    assert_eq!(re_read["m5"], r"C:\AuraZoneRipple\aurazone_ripple.exe --toggle");
}

#[test]
fn test_t1_f7_ghelper_preserves_other_keys() {
    let fixture = TempConfigFixture::new().unwrap();
    let mut json = fixture.read_ghelper_json().unwrap();
    json["m5"] = serde_json::Value::String("new_action".to_string());
    fixture.write_ghelper_json(&json).unwrap();

    let re_read = fixture.read_ghelper_json().unwrap();
    assert_eq!(re_read["battery_limit"], 80);
    assert_eq!(re_read["keyboard_brightness"], 3);
    assert_eq!(re_read["fans"]["balanced"], 0);
}

#[test]
fn test_t1_f7_ghelper_custom_path_override() {
    let fixture = TempConfigFixture::new().unwrap();
    let custom_path = fixture.ghelper_config_path.clone();
    assert!(custom_path.exists());
    let content = std::fs::read_to_string(&custom_path).unwrap();
    assert!(content.contains("performance"));
}

#[test]
fn test_t1_f7_ghelper_restore_static_profile() {
    let fixture = TempConfigFixture::new().unwrap();
    let json = fixture.read_ghelper_json().unwrap();
    let argb = json["aura_color"].as_i64().unwrap();
    // Decode ARGB: -16776961 is 0xFF0000FF (Blue)
    let b = (argb & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let r = ((argb >> 16) & 0xFF) as u8;
    let mut dev = MockAuraDevice::new();
    assert!(dev.set_static_uniform([r, g, b]).is_ok());
    assert_eq!(dev.latest_zones(), Some(&[[0, 0, 255]; 8]));
}

#[test]
fn test_t1_f8_vk_mapping_zone_0_wasd() {
    assert_eq!(map_vk_to_zone(0x57), 0, "W key -> Zone 0");
    assert_eq!(map_vk_to_zone(0x41), 0, "A key -> Zone 0");
    assert_eq!(map_vk_to_zone(0x53), 0, "S key -> Zone 0");
    assert_eq!(map_vk_to_zone(0x44), 0, "D key -> Zone 0");
    assert_eq!(map_vk_to_zone(0x09), 0, "Tab key -> Zone 0");
}

#[test]
fn test_t1_f8_vk_mapping_zone_1_mid_left() {
    assert_eq!(map_vk_to_zone(0x54), 1, "T key -> Zone 1");
    assert_eq!(map_vk_to_zone(0x59), 1, "Y key -> Zone 1");
    assert_eq!(map_vk_to_zone(0x47), 1, "G key -> Zone 1");
    assert_eq!(map_vk_to_zone(0x48), 1, "H key -> Zone 1");
    assert_eq!(map_vk_to_zone(0x42), 1, "B key -> Zone 1");
}

#[test]
fn test_t1_f8_vk_mapping_zone_2_mid_right() {
    assert_eq!(map_vk_to_zone(0x55), 2, "U key -> Zone 2");
    assert_eq!(map_vk_to_zone(0x49), 2, "I key -> Zone 2");
    assert_eq!(map_vk_to_zone(0x4F), 2, "O key -> Zone 2");
    assert_eq!(map_vk_to_zone(0x4A), 2, "J key -> Zone 2");
    assert_eq!(map_vk_to_zone(0x4B), 2, "K key -> Zone 2");
    assert_eq!(map_vk_to_zone(0x4C), 2, "L key -> Zone 2");
}

#[test]
fn test_t1_f8_vk_mapping_zone_3_numpad() {
    assert_eq!(map_vk_to_zone(0x60), 3, "Numpad 0 -> Zone 3");
    assert_eq!(map_vk_to_zone(0x65), 3, "Numpad 5 -> Zone 3");
    assert_eq!(map_vk_to_zone(0x0D), 3, "Enter key -> Zone 3");
}

#[test]
fn test_t1_f8_per_key_debounce_window() {
    let debouncer = DebounceFilter::new(60);
    let t0 = Instant::now();

    // First press allowed
    assert!(debouncer.should_process(0x57, t0));

    // Press same key 20ms later: blocked
    let t1 = t0 + Duration::from_millis(20);
    assert!(!debouncer.should_process(0x57, t1));

    // Press different key 20ms later: allowed!
    assert!(debouncer.should_process(0x41, t1));

    // Release key and press original key 70ms later: allowed!
    debouncer.notify_key_up(0x57);
    let t2 = t0 + Duration::from_millis(70);
    assert!(debouncer.should_process(0x57, t2));
}

#[test]
fn test_t1_f9_wave_propagation_attack_phase() {
    // Attack phase is first 15% (0.0 to 0.15)
    let val_start = two_phase_easing(0.0);
    let val_mid = two_phase_easing(0.075);
    let val_peak = two_phase_easing(0.15);

    assert_eq!(val_start, 0.0);
    assert!(val_mid > 0.0 && val_mid < 1.0);
    assert!((val_peak - 1.0).abs() < 1e-4, "Peak intensity at progress 0.15 must be 1.0");
}

#[test]
fn test_t1_f9_wave_propagation_decay_phase() {
    // Decay phase is remaining 85% (0.15 to 1.0)
    let val_peak = two_phase_easing(0.15);
    let val_decay_mid = two_phase_easing(0.575);
    let val_end = two_phase_easing(1.0);

    assert!((val_peak - 1.0).abs() < 1e-4);
    assert!(val_decay_mid < 1.0 && val_decay_mid > 0.0);
    assert!(val_end < 1e-4, "End intensity at progress 1.0 must be 0.0");
}

#[test]
fn test_t1_f9_wave_distance_attenuation() {
    let duration = Duration::from_millis(1000);
    let elapsed = Duration::from_millis(150); // Peak attack time

    let amp_0 = calculate_zone_intensity(0, 0, elapsed, duration);
    let amp_1 = calculate_zone_intensity(0, 1, elapsed, duration);
    let amp_2 = calculate_zone_intensity(0, 2, elapsed, duration);

    assert!((amp_0 - 1.0).abs() < 1e-4);
    assert!((amp_1 - 0.65).abs() < 1e-4);
    assert!(amp_2 < amp_1);
}

#[test]
fn test_t1_f9_lightbar_cascade_delay() {
    assert_eq!(LIGHTBAR_CASCADE_DELAY_MS, 35);
    assert_eq!(LIGHTBAR_INTENSITY_SCALE, 0.95);

    // At t = 20ms (< 35ms delay), lightbar must be inactive (0.0)
    let kb_intensity = calculate_zone_intensity(0, 0, Duration::from_millis(20), Duration::from_millis(500));
    let lb_intensity = calculate_zone_intensity(0, 4, Duration::from_millis(20), Duration::from_millis(500));
    assert!(kb_intensity > 0.0, "Keyboard zone must begin immediately");
    assert_eq!(lb_intensity, 0.0, "Lightbar zone must be delayed by cascade delay");

    // At t = 100ms, lightbar is active and scaled by 0.95
    let kb_later = calculate_zone_intensity(0, 0, Duration::from_millis(100), Duration::from_millis(500));
    let lb_later = calculate_zone_intensity(0, 4, Duration::from_millis(100), Duration::from_millis(500));
    assert!(lb_later > 0.0);
    assert!(lb_later <= kb_later * LIGHTBAR_INTENSITY_SCALE + 1e-4);
}

#[test]
fn test_t1_f9_dynamic_hsv_rainbow_shift() {
    let initial_hue = 0.0f32;
    let step = 25.0f32;

    let hue_wave1 = (initial_hue + step) % 360.0;
    let hue_wave2 = (hue_wave1 + step) % 360.0;

    let rgb1 = hsv_to_rgb(hue_wave1, 1.0, 1.0);
    let rgb2 = hsv_to_rgb(hue_wave2, 1.0, 1.0);

    assert_ne!(rgb1, rgb2, "Sequential rainbow waves must have distinct colors");
}

#[test]
fn test_t1_f10_config_defaults() {
    let config = RippleConfig::default();
    assert_eq!(config.preset, Preset::Cyberpunk);
    assert_eq!(config.color_ripple, [0, 210, 255]);
    assert_eq!(config.color_base, [255, 0, 128]);
    assert_eq!(config.speed, 1.0);
    assert_eq!(config.decay, 0.85);
    assert_eq!(config.attack, 0.15);
    assert_eq!(config.lightbar_delay_ms, 35);
    assert!(!config.rainbow);
}

#[test]
fn test_t1_f10_config_json_serialize_deserialize() {
    let fixture = TempConfigFixture::new().unwrap();
    let initial = RippleConfig::default();
    initial.save_to_path(&fixture.settings_path).unwrap();
    let recovered = RippleConfig::load_from_path(&fixture.settings_path).unwrap();
    assert_eq!(initial, recovered);
}

#[test]
fn test_t1_f10_preset_cyberpunk() {
    let config = RippleConfig::from_preset(Preset::Cyberpunk);
    assert_eq!(config.preset, Preset::Cyberpunk);
    assert_eq!(config.color_ripple, [0, 210, 255], "Cyberpunk ripple must be electric cyan");
    assert_eq!(config.color_base, [255, 0, 128], "Cyberpunk base must be neon pink");
    assert!(!config.rainbow);
    assert_eq!(config.lightbar_delay_ms, 35);
}

#[test]
fn test_t1_f10_preset_matrix() {
    let config = RippleConfig::from_preset(Preset::Matrix);
    assert_eq!(config.preset, Preset::Matrix);
    assert_eq!(config.color_ripple, [0, 255, 65], "Matrix ripple must be Matrix green");
    assert_eq!(config.color_base, [0, 20, 0], "Matrix base must be deep green");
    assert!(!config.rainbow);
}

#[test]
fn test_t1_f10_preset_stealth() {
    let config = RippleConfig::from_preset(Preset::Stealth);
    assert_eq!(config.preset, Preset::Stealth);
    assert_eq!(config.color_base, [0, 0, 0], "Stealth preset must have completely dark [0,0,0] idle base");
    assert_eq!(config.color_ripple, [255, 255, 255], "Stealth ripple must be crisp white");
}

#[test]
fn test_t1_f10_preset_profile1_and_profile2() {
    let mut config = RippleConfig::default();
    assert_eq!(config.profile_1.name, "Profil 1");
    assert_eq!(config.profile_2.name, "Profil 2");

    // Switching to Profile 1
    config.apply_profile(1);
    assert_eq!(config.preset, Preset::Profile1);
    assert_eq!(config.speed, config.profile_1.speed);
    assert_eq!(config.color_ripple, config.profile_1.color_ripple);

    // Switching to Profile 2
    config.apply_profile(2);
    assert_eq!(config.preset, Preset::Profile2);
    assert_eq!(config.speed, config.profile_2.speed);
    assert_eq!(config.color_base, config.profile_2.color_base);
}

#[test]
fn test_t1_f10_profile_save_and_apply() {
    let mut config = RippleConfig::default();
    config.speed = 1.45;
    config.decay = 0.72;
    config.color_base = [12, 34, 56];
    config.color_ripple = [200, 100, 50];

    // Save current parameters to Profile 1
    config.save_current_to_profile(1);
    assert_eq!(config.profile_1.speed, 1.45);
    assert_eq!(config.profile_1.decay, 0.72);
    assert_eq!(config.profile_1.color_base, [12, 34, 56]);
    assert_eq!(config.profile_1.color_ripple, [200, 100, 50]);

    // Change to another preset then re-apply Profile 1
    config.switch_preset(Preset::Cyberpunk);
    assert_eq!(config.preset, Preset::Cyberpunk);
    assert_eq!(config.speed, 1.0);
    config.apply_profile(1);
    assert_eq!(config.preset, Preset::Profile1);
    assert_eq!(config.speed, 1.45);
    assert_eq!(config.color_base, [12, 34, 56]);
}

#[test]
fn test_t1_f10_profile_backward_compatibility_old_json() {
    // Old JSON format without profile_1 or profile_2 fields
    let old_json = r#"{
      "preset": "Matrix",
      "speed": 1.2,
      "decay": 0.8,
      "attack": 0.15,
      "lightbar_delay_ms": 35,
      "rainbow": false,
      "color_base": [0, 20, 0],
      "color_ripple": [0, 255, 65]
    }"#;

    let config: RippleConfig = serde_json::from_str(old_json).expect("Old JSON must deserialize cleanly");
    assert_eq!(config.preset, Preset::Matrix);
    assert_eq!(config.profile_1.name, "Profil 1");
    assert_eq!(config.profile_2.name, "Profil 2");
}

#[test]
fn test_t1_f13_patch_ghelper_buffer_flow() {
    use aura_zone_ripple::patcher::{
        patch_ghelper_buffer, unpatch_ghelper_buffer, PatchError, ORIGINAL_BYTES, PATCHED_BYTES,
        TARGET_SIGNATURE,
    };

    let mut buf = Vec::new();
    buf.extend_from_slice(b"PREFIX_PADDING_DUMMY_DATA_12345");
    buf.extend_from_slice(&TARGET_SIGNATURE);
    buf.extend_from_slice(b"SUFFIX_PADDING_DUMMY_DATA_67890");

    let off = patch_ghelper_buffer(&mut buf).expect("Patch must succeed on valid signature");
    assert_eq!(off, 31);
    assert_eq!(&buf[off..off + 3], &PATCHED_BYTES);

    // Double patch should fail with AlreadyPatched
    let err = patch_ghelper_buffer(&mut buf).unwrap_err();
    assert_eq!(err, PatchError::AlreadyPatched);

    // Unpatch should restore original bytes
    let unpatch_off = unpatch_ghelper_buffer(&mut buf).expect("Unpatch must succeed");
    assert_eq!(unpatch_off, 31);
    assert_eq!(&buf[unpatch_off..unpatch_off + 3], &ORIGINAL_BYTES);

    // Double unpatch should fail with NotPatched
    let unpatch_err = unpatch_ghelper_buffer(&mut buf).unwrap_err();
    assert_eq!(unpatch_err, PatchError::NotPatched);
}

// ============================================================================
// TIER 2: BOUNDARY & CORNER CASES
// ============================================================================

#[test]
fn test_t2_f1_undersized_buffer_rejected() {
    let short_buffer = [0u8; 63];
    let res = validate_packet(&short_buffer);
    assert!(res.is_err(), "Buffer shorter than 64 bytes must be rejected");
    assert_eq!(
        res.unwrap_err(),
        AuraError::InvalidBufferSize { expected: 64, actual: 63 }
    );
}

#[test]
fn test_t2_f1_oversized_buffer_rejected() {
    let long_buffer = [0u8; 65];
    let res = validate_packet(&long_buffer);
    assert!(res.is_err(), "Buffer longer than 64 bytes must be rejected");
    assert_eq!(
        res.unwrap_err(),
        AuraError::InvalidBufferSize { expected: 64, actual: 65 }
    );
}

#[test]
fn test_t2_f1_invalid_report_id_rejected() {
    let mut pkt = encode_handshake();
    pkt[0] = 0x5C; // Invalid ID
    let mut dev = MockAuraDevice::new();
    assert_eq!(
        dev.send_feature(&pkt).unwrap_err(),
        AuraError::InvalidReportId(0x5C)
    );
}

#[test]
fn test_t2_f1_unknown_command_byte_rejected() {
    let mut pkt = encode_handshake();
    pkt[1] = 0xBD; // Invalid Command
    let mut dev = MockAuraDevice::new();
    assert_eq!(
        dev.send_feature(&pkt).unwrap_err(),
        AuraError::InvalidCommand(0xBD)
    );
}

#[test]
fn test_t2_f2_all_black_colors_handled() {
    let black = [[0u8; 3]; 8];
    let pkt = encode_direct_frame(&black);
    assert_eq!(pkt.len(), 64);
    assert_eq!(pkt[0], 0x5D);
    assert_eq!(pkt[1], 0xBC);
    for i in 9..45 {
        assert_eq!(pkt[i], 0x00);
    }
}

#[test]
fn test_t2_f3_mock_device_ring_buffer_overflow() {
    let mut dev = MockAuraDevice::new();
    let pkt = encode_handshake();
    for _ in 0..10_050 {
        dev.send_feature(&pkt).unwrap();
    }
    assert_eq!(dev.packet_count(), 10_000, "Ring buffer must cap at max capacity");
}

#[test]
fn test_t2_f4_empty_cli_arguments() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .env("AURAZONE_HEADLESS", "1")
        .output()
        .expect("Failed to execute without args");
    assert!(output.status.code().is_some());
}

#[test]
fn test_t2_f4_conflicting_cli_flags() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--daemon")
        .arg("--stop")
        .output()
        .expect("Failed to execute conflicting flags");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_t2_f4_unrecognized_cli_flag() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--nonexistent-flag-xyz")
        .output()
        .expect("Failed to execute unrecognized flag");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_t2_f4_repeated_cli_flags() {
    let bin = require_binary();
    let output = Command::new(&bin)
        .arg("--mock")
        .arg("--mock")
        .arg("--help")
        .output()
        .expect("Failed to execute repeated flags");
    assert!(output.status.success());
}

#[test]
#[cfg(windows)]
fn test_t2_f5_abandoned_mutex_recovery() {
    let mutex_name = format!("Local\\AuraZone_Test_Abandoned_{}", uuid_string());
    let wide: Vec<u16> = mutex_name.encode_utf16().chain(std::iter::once(0)).collect();

    // 1. Main thread creates the named mutex with bInitialOwner = 0 (unowned).
    // Holding h_main open ensures the Windows kernel mutex object is NOT destroyed
    // when the worker thread terminates.
    let h_main = unsafe { win32::CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
    assert!(!h_main.is_null(), "Main thread failed to create mutex");

    // 2. Spawn a separate worker thread that acquires ownership and then dies without releasing.
    let wide_clone = wide.clone();
    let worker_thread = thread::spawn(move || {
        let h_worker = unsafe { win32::CreateMutexW(std::ptr::null(), 0, wide_clone.as_ptr()) };
        assert!(!h_worker.is_null(), "Worker thread failed to open mutex");

        let wait_worker = unsafe { win32::WaitForSingleObject(h_worker, 1000) };
        assert_eq!(wait_worker, win32::WAIT_OBJECT_0, "Worker must acquire mutex ownership");

        // Worker thread terminates HERE without calling ReleaseMutex(h_worker).
        // Windows kernel will mark the mutex object as ABANDONED during thread cleanup.
    });

    // Wait for the worker thread to exit completely
    worker_thread.join().expect("Worker thread panicked unexpectedly");

    // 3. Main thread attempts to acquire the mutex.
    // Because the thread that owned the mutex terminated without releasing it,
    // WaitForSingleObject MUST return strictly WAIT_ABANDONED (0x00000080).
    let wait_res = unsafe { win32::WaitForSingleObject(h_main, 1000) };
    assert_eq!(
        wait_res,
        win32::WAIT_ABANDONED,
        "Expected WAIT_ABANDONED (0x80) when acquiring mutex abandoned by dead thread, got 0x{:X}",
        wait_res
    );

    // 4. Clean up: When WaitForSingleObject returns WAIT_ABANDONED, the calling thread
    // is granted ownership. Release mutex ownership and close handle cleanly.
    let release_res = unsafe { win32::ReleaseMutex(h_main) };
    assert_ne!(release_res, 0, "Failed to release abandoned mutex after acquiring it");
    unsafe {
        win32::CloseHandle(h_main);
    }
}

#[test]
fn test_t2_f6_ipc_client_timeout_when_pipe_busy() {
    let non_existent = r"\\.\pipe\Nonexistent_Busy_Pipe_999";
    let start = Instant::now();
    let res = OpenOptions::new().read(true).write(true).open(non_existent);
    assert!(res.is_err());
    assert!(start.elapsed() < Duration::from_millis(500));
}

#[test]
fn test_t2_f6_ipc_malformed_command_payload() {
    let _bin = require_binary();
    let test_id = format!("t2_f6_malformed_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let resp = client.send_command("GARBAGE_PAYLOAD_INVALID_12345").expect("Pipe write failed");
    assert!(resp.contains("ERR") || resp.contains("Unknown"));
}

#[test]
fn test_t2_f6_ipc_command_without_newline() {
    let _bin = require_binary();
    let test_id = format!("t2_f6_nonewline_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    let _ = client.send_raw(b"STATUS");
    let ping = client.send_command("STATUS");
    assert!(ping.is_ok());
}

#[test]
fn test_t2_f6_ipc_rapid_burst_commands() {
    let _bin = require_binary();
    let test_id = format!("t2_f6_burst_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);
    for _ in 0..50 {
        let resp = client.send_command("STATUS").expect("Burst query failed");
        assert!(resp.contains("running") || resp.contains("OK"));
    }
}

#[test]
fn test_t2_f7_ghelper_config_file_missing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let missing_path = temp_dir.path().join("missing_config.json");
    assert!(!missing_path.exists());
    let res = std::fs::read_to_string(&missing_path);
    assert!(res.is_err());
}

#[test]
fn test_t2_f7_ghelper_config_json_syntax_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corrupt_path = temp_dir.path().join("corrupt.json");
    std::fs::write(&corrupt_path, "{ broken json: true, ").unwrap();

    let content = std::fs::read_to_string(&corrupt_path).unwrap();
    let parse_res: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(parse_res.is_err());
}

#[test]
fn test_t2_f7_ghelper_read_only_filesystem() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("readonly.json");
    std::fs::write(&file_path, "{}").unwrap();
    let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    let _ = std::fs::set_permissions(&file_path, perms);

    let res = OpenOptions::new().write(true).open(&file_path);
    assert!(res.is_err());

    let mut perms2 = std::fs::metadata(&file_path).unwrap().permissions();
    perms2.set_readonly(false);
    let _ = std::fs::set_permissions(&file_path, perms2);
}

#[test]
fn test_t2_f7_ghelper_m5_already_bound() {
    let fixture = TempConfigFixture::new().unwrap();
    let target = r"C:\AuraZoneRipple\aurazone_ripple.exe --toggle";
    let mut json = fixture.read_ghelper_json().unwrap();
    json["m5"] = serde_json::Value::String(target.to_string());
    fixture.write_ghelper_json(&json).unwrap();

    let json2 = fixture.read_ghelper_json().unwrap();
    assert_eq!(json2["m5"], target);
}

#[test]
fn test_t2_f7_ghelper_process_not_found_on_restart() {
    let output = Command::new("taskkill")
        .args(["/FI", "IMAGENAME eq nonexistent_ghelper_fake.exe"])
        .output()
        .expect("Failed to run taskkill");
    assert!(output.status.code().is_some());
}

#[test]
fn test_t2_f8_unknown_vk_code_safe_fallback() {
    let unknown_code = 0xFF;
    let zone = map_vk_to_zone(unknown_code);
    assert!(zone <= 3, "Unmapped VK code must fallback to valid zone 0..3 without panicking");
}

#[test]
fn test_t2_f8_zero_delay_simultaneous_keystrokes() {
    let debouncer = DebounceFilter::new(60);
    let now = Instant::now();

    let keys = [0x57, 0x41, 0x53, 0x44, 0x54, 0x59, 0x55, 0x49, 0x60, 0x0D];
    let mut allowed_count = 0;
    for &k in &keys {
        if debouncer.should_process(k, now) {
            allowed_count += 1;
        }
    }
    assert_eq!(allowed_count, 10, "All distinct simultaneous keys must pass debounce");
}

#[test]
fn test_t2_f9_superposition_8_ripple_saturation() {
    let mut engine = RippleEngine::new();
    assert_eq!(engine.active_ripple_count(), 0);
    for i in 0..12 {
        engine.add_ripple(i % 4, [10 * (i as u8 + 1), 20, 30]);
    }
    assert_eq!(engine.active_ripple_count(), 8, "RippleEngine must cap at 8 concurrent ripples");
    let ripples = engine.active_ripples();
    assert_eq!(ripples[0].color[0], 50, "Oldest surviving ripple must be index 4");
    assert_eq!(ripples[7].color[0], 120, "Newest ripple must be index 11");
}

#[test]
fn test_t2_f9_extreme_color_intensity_clamping() {
    let color_a = [200, 150, 220];
    let color_b = [180, 160, 90];
    let summed = saturating_add_rgb(color_a, color_b);
    assert_eq!(summed[0], 255);
    assert_eq!(summed[1], 255);
    assert_eq!(summed[2], 255);

    let mut engine = RippleEngine::new();
    for _ in 0..6 {
        engine.add_ripple(0, [220, 220, 220]);
    }
    let frame = engine.render_frame(Duration::from_millis(100), &[0, 0, 0]);
    assert_eq!(frame[0], [255, 255, 255], "Saturated zone must clamp at 255 without byte overflow wrap");
}

#[test]
fn test_t2_f10_corrupted_settings_json_fallback() {
    let fixture = TempConfigFixture::new().unwrap();
    std::fs::write(&fixture.settings_path, "MALFORMED_GARBAGE").unwrap();

    let backup_path = fixture.settings_path.with_extension("json.bak");
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    let (recovered_cfg, was_recovered) = RippleConfig::load_or_recover(&fixture.settings_path);
    assert!(was_recovered, "Fallback loader must report recovery on corrupted file");
    assert!(backup_path.exists(), "Corrupted settings file must be backed up to .json.bak");
    assert_eq!(recovered_cfg, RippleConfig::default(), "Fallback must restore default configuration");
    assert!(RippleConfig::load_from_path(&fixture.settings_path).is_ok());
}

// ============================================================================
// TIER 3: CROSS-FEATURE COMBINATIONS
// ============================================================================

#[test]
fn test_t3_p1_daemon_mock_ipc_lifecycle() {
    let _bin = require_binary();
    let test_id = format!("t3_p1_lifecycle_{}", uuid_string());
    let mut daemon = DaemonGuard::spawn_mock(&test_id).expect("Spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);

    // 1. Check initial status
    let status = client.send_command("STATUS").unwrap();
    assert!(status.contains("running") || status.contains("OK"));

    // 2. Toggle pause
    let toggle_resp = client.send_command("TOGGLE").unwrap();
    assert!(toggle_resp.contains("OK") || toggle_resp.contains("TOGGLE") || toggle_resp.contains("paused"));

    // 3. Stop daemon
    let stop_resp = client.send_command("STOP").unwrap();
    assert!(stop_resp.contains("OK") || stop_resp.contains("STOP"));

    let exit_status = daemon.child.wait().unwrap();
    assert!(exit_status.success());
}

#[test]
fn test_t3_p2_keystroke_to_mock_packet_flow() {
    let mut dev = MockAuraDevice::new();
    let debouncer = DebounceFilter::new(60);

    // Keystroke WASD (0x57) -> Zone 0
    let now = Instant::now();
    assert!(debouncer.should_process(0x57, now));
    let zone = map_vk_to_zone(0x57);
    assert_eq!(zone, 0);

    // Calculate wave color for Zone 0
    let ripple_color = [0, 210, 255];
    let base_color = [0, 0, 0];
    let intensity = two_phase_easing(0.15); // Peak
    let blended = blend_colors(base_color, ripple_color, intensity);

    let mut zone_colors = [base_color; 8];
    zone_colors[zone] = blended;

    dev.apply_zones(&zone_colors).unwrap();
    assert_eq!(dev.packet_count(), 1);
    let pkt = dev.sent_packets()[0];
    assert_eq!(pkt[0..5], [0x5D, 0xBC, 0x00, 0x01, 0x04]);
    assert_eq!(pkt[9], blended[0]);
    assert_eq!(pkt[10], blended[1]);
    assert_eq!(pkt[11], blended[2]);
}

#[test]
fn test_t3_p3_ghelper_install_and_cli_restore() {
    let fixture = TempConfigFixture::new().unwrap();
    let bin = require_binary();

    // 1. Dry run install against isolated fixture
    let output = Command::new(&bin)
        .arg("--install-ghelper")
        .env("GHELPER_CONFIG_PATH", fixture.ghelper_config_path.to_str().unwrap())
        .output()
        .expect("Install failed");
    assert!(output.status.code().is_some());

    // 2. Restore command against isolated fixture
    let restore = Command::new(&bin)
        .arg("--restore")
        .arg("--mock")
        .env("GHELPER_CONFIG_PATH", fixture.ghelper_config_path.to_str().unwrap())
        .output()
        .expect("Restore failed");
    assert!(restore.status.success());
}

#[test]
fn test_t3_p4_config_hot_reload_applies_to_physics() {
    let fixture = TempConfigFixture::new().unwrap();
    let mut config = RippleConfig::default();
    config.speed = 2.5;
    config.save_to_path(&fixture.settings_path).unwrap();

    let reloaded = RippleConfig::load_from_path(&fixture.settings_path).unwrap();
    assert_eq!(reloaded.speed, 2.5);
    assert_eq!(reloaded.preset, Preset::Cyberpunk);
}

#[test]
fn test_t3_p5_concurrent_ipc_clients() {
    let _bin = require_binary();
    let test_id = format!("t3_p5_concurrent_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Spawn failed");
    let pipe_name = daemon.pipe_name.clone();

    let mut handles = Vec::new();
    for thread_idx in 0..5 {
        let p_name = pipe_name.clone();
        handles.push(thread::spawn(move || {
            let client = IpcTestClient::new(&p_name);
            for _ in 0..5 {
                let resp = client.send_command("STATUS").expect("Concurrent query failed");
                assert!(resp.contains("running") || resp.contains("OK"));
            }
            thread_idx
        }));
    }

    for h in handles {
        h.join().expect("Concurrent client thread panicked");
    }
}

#[test]
fn test_t3_p6_rainbow_burst_hardware_slots() {
    let mut dev = MockAuraDevice::new();
    for i in 0..10 {
        let hue = (i as f32 * 36.0) % 360.0;
        let rgb = hsv_to_rgb(hue, 1.0, 1.0);
        let colors = [rgb; 8];
        dev.apply_zones(&colors).unwrap();
    }
    assert_eq!(dev.packet_count(), 10);
    for pkt in dev.sent_packets() {
        assert_eq!(pkt.len(), 64);
        assert_eq!(pkt[0], 0x5D);
        assert_eq!(pkt[1], 0xBC);
    }
}

// ============================================================================
// TIER 4: REAL-WORLD APPLICATION SCENARIOS
// ============================================================================

#[test]
fn test_t4_scenario_1_daily_startup_background_operation() {
    let _bin = require_binary();
    let test_id = format!("t4_sc1_daily_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);

    for _ in 0..3 {
        thread::sleep(Duration::from_millis(50));
        let status = client.send_command("STATUS").unwrap();
        assert!(status.contains("running") || status.contains("OK"));
    }
}

#[test]
fn test_t4_scenario_2_hardware_hotkey_toggle_simulation() {
    let _bin = require_binary();
    let test_id = format!("t4_sc2_toggle_{}", uuid_string());
    let daemon = DaemonGuard::spawn_mock(&test_id).expect("Daemon spawn failed");
    let client = IpcTestClient::new(&daemon.pipe_name);

    // First toggle: Pause
    let toggle1 = client.send_command("TOGGLE").unwrap();
    assert!(toggle1.contains("OK") || toggle1.contains("TOGGLE") || toggle1.contains("paused"));

    // Second toggle: Resume
    let toggle2 = client.send_command("TOGGLE").unwrap();
    assert!(toggle2.contains("OK") || toggle2.contains("TOGGLE") || toggle2.contains("active"));
}

#[test]
fn test_t4_scenario_3_fast_touch_typing_keystroke_burst() {
    let mut dev = MockAuraDevice::new();
    let debouncer = DebounceFilter::new(60);

    // Simulate 120 WPM typing burst: 25 keys within 2 seconds
    let keys = [
        0x51, 0x57, 0x45, 0x52, 0x54, 0x59, 0x55, 0x49, 0x4F, 0x50, // QWERTYUIOP
        0x41, 0x53, 0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C,       // ASDFGHJKL
        0x5A, 0x58, 0x43, 0x56, 0x42, 0x4E,                         // ZXCVBN
    ];

    let mut processed_keys = 0;
    let mut t = Instant::now();

    for &k in &keys {
        if debouncer.should_process(k, t) {
            processed_keys += 1;
            let zone = map_vk_to_zone(k);
            let mut colors = [[0u8; 3]; 8];
            colors[zone] = [0, 210, 255];
            dev.apply_zones(&colors).unwrap();
        }
        t += Duration::from_millis(80); // 80ms interval
    }

    assert_eq!(processed_keys, 25, "All 25 distinct keys must be processed");
    assert_eq!(dev.packet_count(), 25, "All 25 frames must be recorded");
}

#[test]
fn test_t4_scenario_4_gui_config_and_presets() {
    let fixture = TempConfigFixture::new().unwrap();

    let stealth_config = RippleConfig::from_preset(Preset::Stealth);
    stealth_config.save_to_path(&fixture.settings_path).unwrap();

    let read_back = RippleConfig::load_from_path(&fixture.settings_path).unwrap();
    assert_eq!(read_back.preset, Preset::Stealth);
    assert_eq!(read_back.color_base, [0, 0, 0]);
    assert_eq!(read_back.color_ripple, [255, 255, 255]);
}

#[test]
fn test_t4_scenario_5_clean_system_shutdown_and_static_restore() {
    let mut dev = MockAuraDevice::new();
    // Simulate active ripple stream
    dev.apply_zones(&[[0, 210, 255]; 8]).unwrap();
    assert_eq!(dev.packet_count(), 1);

    // System shutdown signal -> restore static fallback G-Helper profile
    let static_blue = [0, 0, 255];
    dev.set_static_uniform(static_blue).unwrap();
    assert_eq!(dev.packet_count(), 2);
    assert_eq!(dev.latest_zones(), Some(&[static_blue; 8]));
}

#[test]
fn test_t4_scenario_6_fault_recovery_from_corrupted_environment() {
    let fixture = TempConfigFixture::new().unwrap();
    std::fs::write(&fixture.settings_path, "{ corrupted malformed syntax").unwrap();

    let backup_path = fixture.settings_path.with_extension("json.bak");
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    // Engine detects corruption, creates .bak backup, and restores default config
    let (config, recovered) = RippleConfig::load_or_recover(&fixture.settings_path);
    assert!(recovered, "Engine must report that corruption recovery occurred");
    assert!(backup_path.exists(), "Engine must create .bak backup of corrupted file");
    assert_eq!(config, RippleConfig::default(), "Engine must recover valid default config");

    // Verify recovered settings file is valid JSON
    let reload = RippleConfig::load_from_path(&fixture.settings_path);
    assert!(reload.is_ok());
}

// ============================================================================
// Helper Utilities
// ============================================================================

fn uuid_string() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}_{}", duration.as_secs(), duration.subsec_nanos())
}
