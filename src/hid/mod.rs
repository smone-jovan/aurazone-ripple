//! HID Hardware Abstraction Layer for ASUS ROG Aura Direct Controllers.

pub mod mock_device;
pub mod protocol;
pub mod win32_device;

pub use mock_device::{MockAuraDevice, SharedMockAuraDevice};
pub use protocol::{
    encode_direct_frame, encode_handshake, validate_packet, ColorRgb, Packet, AURA_DIRECT,
    AURA_DIRECT_4ZONE_CMD, AURA_DIRECT_CMD, AURA_HANDSHAKE_INIT, AURA_ID, AURA_REPORT_ID,
    HARDWARE_SLOT_COUNT, LOGICAL_ZONE_COUNT, PACKET_SIZE, PAYLOAD_OFFSET, REPORT_SIZE,
    SLOT_COUNT, TAIL_PADDING_OFFSET, ZONE_COUNT_LOGICAL, ZONE_MAP,
};
pub use win32_device::Win32AuraDevice;

use thiserror::Error;

/// Strongly-typed error enum representing any failure within the HID subsystem.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum AuraError {
    #[error("Aura HID device Col05 (VID_0B05&PID_19B6) was not found on this system")]
    DeviceNotFound,

    #[error("Failed to open Aura HID device handle (Win32 Error: {0})")]
    OpenDeviceFailed(u32),

    #[error("Failed to dispatch HID feature report (Win32 Error: {0})")]
    SetFeatureFailed(u32),

    #[error("Invalid HID buffer size: expected {expected} bytes, got {actual}")]
    InvalidBufferSize { expected: usize, actual: usize },

    #[error("Invalid Aura Report ID: expected 0x5D, got 0x{0:02X}")]
    InvalidReportId(u8),

    #[error("Invalid Aura Direct Command: expected 0xBC, got 0x{0:02X}")]
    InvalidCommand(u8),

    #[error("Device handle access denied (Win32 Error 5). Ensure no conflicting exclusive lock exists.")]
    AccessDenied,

    #[error("Device sharing violation (Win32 Error 32). Another process has locked the device exclusively.")]
    SharingViolation,

    #[error("Device disconnected or USB suspended (Win32 Error 1167 / 0x48F).")]
    DeviceDisconnected,

    #[error("Invalid parameter sent to HID driver (Win32 Error 87). Ensure buffer is exactly 64 bytes.")]
    InvalidParameter,

    #[error("Simulated hardware error: {0}")]
    InjectedError(String),

    #[error("Device communication channel closed or unavailable")]
    DeviceClosed,

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("I/O error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for AuraError {
    fn from(err: std::io::Error) -> Self {
        AuraError::IoError(err.to_string())
    }
}

impl From<u32> for AuraError {
    fn from(code: u32) -> Self {
        match code {
            2 | 3 => AuraError::DeviceNotFound,
            5 => AuraError::AccessDenied,
            32 => AuraError::SharingViolation,
            87 => AuraError::InvalidParameter,
            1167 => AuraError::DeviceDisconnected,
            other => AuraError::SetFeatureFailed(other),
        }
    }
}

/// Core hardware abstraction trait for ASUS Aura lighting devices.
///
/// Encapsulates raw feature report transmission and high-level zone color application.
pub trait AuraDevice: Send + Sync {
    /// Dispatches a raw 64-byte HID feature report to the device.
    fn send_feature(&mut self, report: &[u8; 64]) -> Result<(), AuraError>;

    /// Encodes 8 logical RGB zones into the 12-slot hardware format and dispatches to the device.
    ///
    /// - `zone_colors[0..4]`: Keyboard Zones (0=Left, 1=Mid-Left, 2=Mid-Right, 3=Right)
    /// - `zone_colors[4..8]`: Front Lightbar (4=Left-Wrap, 5=Mid-Left, 6=Mid-Right, 7=Right-Wrap)
    fn apply_zones(&mut self, zone_colors: &[[u8; 3]; 8]) -> Result<(), AuraError> {
        let frame = protocol::encode_direct_frame(zone_colors);
        self.send_feature(&frame)
    }

    /// Sets all 8 logical zones to a uniform static RGB color.
    fn set_static_uniform(&mut self, color: [u8; 3]) -> Result<(), AuraError> {
        self.apply_zones(&[color; 8])
    }
}
