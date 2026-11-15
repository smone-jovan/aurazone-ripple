//! Common test fixtures and helpers for AuraZoneRipple_Rust E2E test suite.
//!
//! Provides:
//! - `DaemonGuard`: RAII child process wrapper for `aurazone_ripple.exe --daemon --mock`
//!   that guarantees process termination and cleanup on drop.
//! - `IpcTestClient`: Windows Named Pipe client connecting to `\\.\pipe\AuraZoneRipple_IPC`
//!   (or test override `AURAZONE_PIPE_NAME`) sending newline-terminated commands and reading responses.
//! - `TempConfigFixture`: Isolated temporary sandbox using `tempfile` for testing G-Helper
//!   `config.json` and `ripple_settings.json` without modifying live user configs.
//! - Protocol constants and verification helpers.

#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Standard protocol constants for ASUS ROG Strix G513RC Direct Mode
pub const REPORT_SIZE: usize = 64;
pub const AURA_REPORT_ID: u8 = 0x5D;
pub const AURA_DIRECT_CMD: u8 = 0xBC;
pub const AURA_DIRECT_4ZONE_CMD: u8 = 0x04;
pub const AURA_HANDSHAKE_INIT: u8 = 0x01;
pub const HARDWARE_SLOTS: usize = 12;
pub const LOGICAL_ZONES: usize = 8;
pub const ZONE_MAP: [usize; HARDWARE_SLOTS] = [0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7];

/// Returns the path to the compiled `aurazone_ripple.exe` binary.
pub fn get_binary_path() -> PathBuf {
    // 1. Cargo env var when executed via cargo test
    if let Some(path) = option_env!("CARGO_BIN_EXE_aurazone_ripple") {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    // 2. Relative check in target/debug or target/release
    let candidates = [
        PathBuf::from("target/debug/aurazone_ripple.exe"),
        PathBuf::from("target/release/aurazone_ripple.exe"),
        PathBuf::from("../target/debug/aurazone_ripple.exe"),
        PathBuf::from("../target/release/aurazone_ripple.exe"),
    ];
    for cand in &candidates {
        if cand.exists() {
            return cand.clone();
        }
    }
    // Fallback to target/debug default
    PathBuf::from("target/debug/aurazone_ripple.exe")
}

/// Enforces that the `aurazone_ripple.exe` binary exists, panicking with a clear,
/// actionable error message if missing. Strictly prevents silent test passes.
pub fn require_binary() -> PathBuf {
    let bin = get_binary_path();
    assert!(
        bin.exists(),
        "Required test binary 'aurazone_ripple.exe' not found at {:?}. Build the project with 'cargo build --bin aurazone_ripple' before running E2E tests.",
        bin
    );
    bin
}

/// RAII Guard for background mock daemon child processes.
/// Automatically terminates the child process on drop, preventing orphaned daemons.
pub struct DaemonGuard {
    pub child: Child,
    pub pipe_name: String,
    pub mutex_name: String,
    pub test_id: String,
}

impl DaemonGuard {
    /// Spawns `aurazone_ripple.exe --daemon --mock` with isolated pipe and mutex names,
    /// and sandbox configuration paths to prevent host environment pollution.
    pub fn spawn_mock(test_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pipe_name = format!(r"\\.\pipe\AuraZoneRipple_IPC_{}", test_id);
        let mutex_name = format!("Local\\AuraZoneRipple_Mutex_{}", test_id);
        let bin_path = require_binary();

        let temp_dir = tempfile::tempdir()?;
        let ghelper_config = temp_dir.path().join("config.json");
        let settings_path = temp_dir.path().join("ripple_settings.json");
        std::fs::write(&ghelper_config, "{}")?;
        std::fs::write(&settings_path, "{}")?;

        let mut cmd = Command::new(&bin_path);
        cmd.arg("--daemon")
            .arg("--mock")
            .env("AURAZONE_PIPE_NAME", &pipe_name)
            .env("AURAZONE_MUTEX_NAME", &mutex_name)
            .env("GHELPER_CONFIG_PATH", ghelper_config.to_str().unwrap())
            .env("AURAZONE_SETTINGS_PATH", settings_path.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Err(format!(
                    "Failed to spawn daemon at '{:?}': {}. Ensure binary is built.",
                    bin_path, e
                )
                .into());
            }
        };

        let guard = Self {
            child,
            pipe_name,
            mutex_name,
            test_id: test_id.to_string(),
        };

        // Poll until pipe server accepts connections or timeout
        guard.wait_for_pipe_ready(Duration::from_secs(3))?;
        Ok(guard)
    }

    /// Spawns daemon with default production pipe and mutex (used for singleton collision tests).
    pub fn spawn_mock_default() -> Result<Self, Box<dyn std::error::Error>> {
        let pipe_name = r"\\.\pipe\AuraZoneRipple_IPC".to_string();
        let mutex_name = "Local\\AuraZoneRipple_Daemon_Mutex".to_string();
        let bin_path = require_binary();

        let temp_dir = tempfile::tempdir()?;
        let ghelper_config = temp_dir.path().join("config.json");
        let settings_path = temp_dir.path().join("ripple_settings.json");
        std::fs::write(&ghelper_config, "{}")?;
        std::fs::write(&settings_path, "{}")?;

        let mut cmd = Command::new(&bin_path);
        cmd.arg("--daemon")
            .arg("--mock")
            .env("GHELPER_CONFIG_PATH", ghelper_config.to_str().unwrap())
            .env("AURAZONE_SETTINGS_PATH", settings_path.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;

        let guard = Self {
            child,
            pipe_name,
            mutex_name,
            test_id: "default".to_string(),
        };

        guard.wait_for_pipe_ready(Duration::from_secs(3))?;
        Ok(guard)
    }

    /// Polls named pipe until receptive, verifies round-trip ping-pong, and allows
    /// server loop to settle cleanly before returning.
    pub fn wait_for_pipe_ready(&self, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(pipe) = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.pipe_name)
            {
                let mut reader = BufReader::new(pipe);
                let pipe_writer = reader.get_mut();
                if pipe_writer.write_all(b"STATUS\n").is_ok() && pipe_writer.flush().is_ok() {
                    let mut response = String::new();
                    // Crucial: Read the full response before closing the pipe to prevent
                    // ERROR_BROKEN_PIPE on the server and ERROR_PIPE_BUSY on next client.
                    if reader.read_line(&mut response).is_ok() && !response.trim().is_empty() {
                        // Settle delay allows server loop to cycle DisconnectNamedPipe -> ConnectNamedPipe
                        thread::sleep(Duration::from_millis(50));
                        return Ok(());
                    }
                }
            }
            thread::sleep(Duration::from_millis(30));
        }
        Err(format!(
            "Timed out after {:?} waiting for named pipe '{}' to be ready",
            timeout, self.pipe_name
        )
        .into())
    }

    /// Checks if the daemon child process is still actively running.
    pub fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        // Kill child process and wait to prevent zombie/orphaned processes
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Helper client for interacting with the daemon's Windows Named Pipe
pub struct IpcTestClient {
    pub pipe_path: String,
}

impl IpcTestClient {
    pub fn new(pipe_path: &str) -> Self {
        Self {
            pipe_path: pipe_path.to_string(),
        }
    }

    /// Opens named pipe with retry on transient Win32 ERROR_PIPE_BUSY (error code 231).
    fn open_pipe_with_retry(&self, timeout: Duration) -> Result<std::fs::File, std::io::Error> {
        let start = Instant::now();
        loop {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.pipe_path)
            {
                Ok(file) => return Ok(file),
                Err(e) if e.raw_os_error() == Some(231) => {
                    // Win32 ERROR_PIPE_BUSY: Server is cycling connections
                    if start.elapsed() >= timeout {
                        return Err(e);
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Sends a newline-terminated command string and reads the single-line response.
    pub fn send_command(&self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut pipe = self.open_pipe_with_retry(Duration::from_secs(2))?;

        let formatted = if command.ends_with('\n') {
            command.to_string()
        } else {
            format!("{}\n", command.trim_end())
        };

        pipe.write_all(formatted.as_bytes())?;
        pipe.flush()?;

        let mut reader = BufReader::new(pipe);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response.trim().to_string())
    }

    /// Sends raw bytes without automatic newline for boundary/fuzz testing.
    pub fn send_raw(&self, raw: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut pipe = self.open_pipe_with_retry(Duration::from_secs(2))?;

        pipe.write_all(raw)?;
        pipe.flush()?;

        let mut buffer = Vec::new();
        let mut temp = [0u8; 512];
        let bytes_read = pipe.read(&mut temp)?;
        buffer.extend_from_slice(&temp[..bytes_read]);
        Ok(buffer)
    }
}

/// Fixture providing an isolated temporary directory for G-Helper and settings JSON configs.
pub struct TempConfigFixture {
    pub temp_dir: TempDir,
    pub ghelper_config_path: PathBuf,
    pub settings_path: PathBuf,
}

impl TempConfigFixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let ghelper_config_path = temp_dir.path().join("config.json");
        let settings_path = temp_dir.path().join("ripple_settings.json");

        // Seed realistic G-Helper config matching ROG Strix G513RC
        let ghelper_initial = r#"{
  "keyboard_brightness": 3,
  "aura_mode": 0,
  "aura_color": -16776961,
  "aura_color2": -49152,
  "m5": "performance",
  "battery_limit": 80,
  "fans": {
    "balanced": 0
  }
}"#;
        std::fs::write(&ghelper_config_path, ghelper_initial)?;

        // Seed default ripple settings
        let settings_initial = r#"{
  "preset": "Cyberpunk",
  "speed": 1.0,
  "decay": 0.85,
  "attack": 0.15,
  "lightbar_delay_ms": 35,
  "rainbow": false,
  "color_base": [0, 0, 0],
  "color_ripple": [0, 210, 255]
}"#;
        std::fs::write(&settings_path, settings_initial)?;

        Ok(Self {
            temp_dir,
            ghelper_config_path,
            settings_path,
        })
    }

    /// Reads G-Helper config as a parsed JSON value.
    pub fn read_ghelper_json(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(&self.ghelper_config_path)?;
        let val = serde_json::from_str(&content)?;
        Ok(val)
    }

    /// Writes modified G-Helper config.
    pub fn write_ghelper_json(
        &self,
        val: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(val)?;
        std::fs::write(&self.ghelper_config_path, content)?;
        Ok(())
    }

    /// Reads ripple settings as a parsed JSON value.
    pub fn read_settings_json(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(&self.settings_path)?;
        let val = serde_json::from_str(&content)?;
        Ok(val)
    }

    /// Writes modified ripple settings.
    pub fn write_settings_json(
        &self,
        val: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(val)?;
        std::fs::write(&self.settings_path, content)?;
        Ok(())
    }
}

// ============================================================================
// Opaque-Box Reference Implementations for Testing Protocol Invariants
// ============================================================================

/// Encodes the authoritative 64-byte handshake primer packet:
/// `[0x5D, 0xBC, 0x01] + [0x00]*61`.
pub fn ref_encode_handshake() -> [u8; REPORT_SIZE] {
    let mut pkt = [0u8; REPORT_SIZE];
    pkt[0] = AURA_REPORT_ID;
    pkt[1] = AURA_DIRECT_CMD;
    pkt[2] = AURA_HANDSHAKE_INIT;
    pkt
}

/// Encodes the authoritative 64-byte direct RGB frame packet:
/// Header: `[0x5D, 0xBC, 0x00, 0x01, 0x04]`, 4-byte padding `[0x00; 4]`,
/// Payload: 12 slots mapped via `[0, 1, 2, 3, 0, 0, 4, 4, 5, 6, 7, 7]`,
/// Tail padding: 19 bytes of zeroes (`pkt[45..64]`).
pub fn ref_encode_direct_frame(zone_colors: &[[u8; 3]; LOGICAL_ZONES]) -> [u8; REPORT_SIZE] {
    let mut pkt = [0u8; REPORT_SIZE];
    pkt[0] = AURA_REPORT_ID;
    pkt[1] = AURA_DIRECT_CMD;
    pkt[2] = 0x00;
    pkt[3] = 0x01;
    pkt[4] = AURA_DIRECT_4ZONE_CMD;

    let mut offset = 9;
    for &zone in &ZONE_MAP {
        let rgb = zone_colors[zone];
        pkt[offset] = rgb[0];
        pkt[offset + 1] = rgb[1];
        pkt[offset + 2] = rgb[2];
        offset += 3;
    }
    pkt
}

/// Reference two-phase wave easing calculation:
/// - 15% smooth attack (sine curve: `sin(progress / 0.15 * PI / 2)`)
/// - 85% decay (cosine-squared ease-out: `cos((progress - 0.15) / 0.85 * PI / 2)^2`)
pub fn ref_two_phase_easing(progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);
    if p < 0.15 {
        let t = p / 0.15;
        (t * std::f32::consts::FRAC_PI_2).sin()
    } else {
        let t = (p - 0.15) / 0.85;
        let c = (t * std::f32::consts::FRAC_PI_2).cos();
        c * c
    }
}
