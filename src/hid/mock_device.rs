//! In-memory inspectable mock Aura HID device for testing, simulation, and dry-run execution.

use std::sync::Arc;
use parking_lot::Mutex;

use super::{protocol, AuraDevice, AuraError};

/// Inspectable, thread-safe mock device simulating the ASUS Aura HID controller.
#[derive(Debug, Clone)]
pub struct MockAuraDevice {
    packets: Vec<[u8; 64]>,
    latest_zones: Option<[[u8; 3]; 8]>,
    injected_error: Option<String>,
    connected: bool,
    max_history: usize,
}

impl Default for MockAuraDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAuraDevice {
    /// Creates a new MockAuraDevice in connected state with default history limit.
    pub fn new() -> Self {
        Self {
            packets: Vec::new(),
            latest_zones: None,
            injected_error: None,
            connected: true,
            max_history: 10_000,
        }
    }

    /// Creates a mock device with simulated immediate hardware failure.
    pub fn with_injected_error(error_msg: impl Into<String>) -> Self {
        let mut dev = Self::new();
        dev.inject_error(error_msg);
        dev
    }

    /// Injects a simulated hardware failure for testing error handling and recovery.
    pub fn inject_error(&mut self, error_msg: impl Into<String>) {
        self.injected_error = Some(error_msg.into());
    }

    /// Clears any previously injected hardware error.
    pub fn clear_error(&mut self) {
        self.injected_error = None;
    }

    /// Simulates device connection or disconnection.
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Returns whether the device is currently simulated as connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Returns a slice of all recorded packets dispatched to this mock device.
    pub fn sent_packets(&self) -> &[[u8; 64]] {
        &self.packets
    }

    /// Returns the most recently dispatched 64-byte packet, if any.
    pub fn latest_packet(&self) -> Option<&[u8; 64]> {
        self.packets.last()
    }

    /// Returns the total count of feature reports dispatched.
    pub fn packet_count(&self) -> usize {
        self.packets.len()
    }

    /// Returns the most recent 8-zone RGB color matrix applied.
    pub fn latest_zones(&self) -> Option<&[[u8; 3]; 8]> {
        self.latest_zones.as_ref()
    }

    /// Clears the packet history buffer.
    pub fn clear_packets(&mut self) {
        self.packets.clear();
    }
}

impl AuraDevice for MockAuraDevice {
    fn send_feature(&mut self, report: &[u8; 64]) -> Result<(), AuraError> {
        if let Some(ref err) = self.injected_error {
            return Err(AuraError::InjectedError(err.clone()));
        }
        if !self.connected {
            return Err(AuraError::DeviceClosed);
        }
        protocol::validate_packet(report)?;

        if self.packets.len() >= self.max_history {
            self.packets.remove(0);
        }
        self.packets.push(*report);
        Ok(())
    }

    fn apply_zones(&mut self, zone_colors: &[[u8; 3]; 8]) -> Result<(), AuraError> {
        self.latest_zones = Some(*zone_colors);
        let frame = protocol::encode_direct_frame(zone_colors);
        self.send_feature(&frame)
    }

    fn set_static_uniform(&mut self, color: [u8; 3]) -> Result<(), AuraError> {
        self.apply_zones(&[color; 8])
    }
}

/// Thread-safe wrapper enabling concurrent access and packet inspection.
#[derive(Clone, Debug, Default)]
pub struct SharedMockAuraDevice {
    inner: Arc<Mutex<MockAuraDevice>>,
}

impl SharedMockAuraDevice {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockAuraDevice::new())),
        }
    }

    pub fn packet_count(&self) -> usize {
        self.inner.lock().packet_count()
    }

    pub fn latest_packet(&self) -> Option<[u8; 64]> {
        self.inner.lock().latest_packet().copied()
    }

    pub fn latest_zones(&self) -> Option<[[u8; 3]; 8]> {
        self.inner.lock().latest_zones().copied()
    }

    pub fn inject_error(&self, msg: impl Into<String>) {
        self.inner.lock().inject_error(msg);
    }

    pub fn clear_error(&self) {
        self.inner.lock().clear_error();
    }

    pub fn clear_packets(&self) {
        self.inner.lock().clear_packets();
    }
}

impl AuraDevice for SharedMockAuraDevice {
    fn send_feature(&mut self, report: &[u8; 64]) -> Result<(), AuraError> {
        self.inner.lock().send_feature(report)
    }

    fn apply_zones(&mut self, zone_colors: &[[u8; 3]; 8]) -> Result<(), AuraError> {
        self.inner.lock().apply_zones(zone_colors)
    }

    fn set_static_uniform(&mut self, color: [u8; 3]) -> Result<(), AuraError> {
        self.inner.lock().set_static_uniform(color)
    }
}
