//! Pure, zero-allocation protocol serialization for ASUS ROG Aura Direct Mode.

use super::AuraError;

/// Report ID for ASUS Aura HID feature reports.
pub const AURA_REPORT_ID: u8 = 0x5D;
pub const AURA_ID: u8 = AURA_REPORT_ID;

/// Direct mode command byte.
pub const AURA_DIRECT_CMD: u8 = 0xBC;
pub const AURA_DIRECT: u8 = AURA_DIRECT_CMD;

/// Direct 4-Zone configuration command subcode.
pub const AURA_DIRECT_4ZONE_CMD: u8 = 0x04;

/// Handshake initialization primer subcode.
pub const AURA_HANDSHAKE_INIT: u8 = 0x01;

/// Fixed buffer size mandated by Windows HID API for Aura feature reports.
pub const REPORT_SIZE: usize = 64;
pub const PACKET_SIZE: usize = REPORT_SIZE;

/// Number of hardware LED slots addressable by the controller.
pub const SLOT_COUNT: usize = 12;
pub const HARDWARE_SLOT_COUNT: usize = SLOT_COUNT;

/// Number of logical zones exposed to the physics engine and UI.
pub const LOGICAL_ZONE_COUNT: usize = 8;
pub const ZONE_COUNT_LOGICAL: usize = LOGICAL_ZONE_COUNT;

/// Byte offset where the 12-slot RGB payload begins.
pub const PAYLOAD_OFFSET: usize = 9;

/// Byte offset where payload ends and tail padding begins (9 + 12 * 3 = 45).
pub const TAIL_PADDING_OFFSET: usize = 45;

/// Hardware slot mapping table for ASUS ROG Strix G513:
/// Maps 12 physical hardware slots to 8 logical zones:
/// - Slots 0..3: Keyboard Zones 0..3 (Left, Mid-Left, Mid-Right, Right)
/// - Slots 4..5: Auxiliary left channels mirroring Zone 0
/// - Slots 6..11: Hardware bus wires ascending Right -> Left:
///   Slots 6..7: Front underglow right-wrap (Zone 7)
///   Slots 8..9: Front underglow center channels (Zones 6, 5)
///   Slots 10..11: Front underglow left-wrap (Zone 4)
pub const ZONE_MAP: [usize; SLOT_COUNT] = [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4];

pub type Packet = [u8; REPORT_SIZE];
pub type ColorRgb = [u8; 3];

/// Encodes the G-Helper initialization primer handshake packet.
///
/// Output packet structure (strictly 64 bytes):
/// - `[0]` = 0x5D (Report ID)
/// - `[1]` = 0xBC (Direct Command)
/// - `[2]` = 0x01 (Handshake Init)
/// - `[3..64]` = 0x00 (Zero-padded remainder)
#[inline]
pub fn encode_handshake() -> [u8; REPORT_SIZE] {
    let mut pkt = [0u8; REPORT_SIZE];
    pkt[0] = AURA_REPORT_ID;
    pkt[1] = AURA_DIRECT_CMD;
    pkt[2] = AURA_HANDSHAKE_INIT;
    pkt
}

/// Encodes 8 logical RGB zone colors into a 64-byte direct frame packet.
///
/// Layout:
/// - `pkt[0]` = 0x5D (Report ID)
/// - `pkt[1]` = 0xBC (Direct Command)
/// - `pkt[2]` = 0x00
/// - `pkt[3]` = 0x01
/// - `pkt[4]` = 0x04 (4-Zone Direct Mode)
/// - `pkt[5..9]` = 0x00 (Header padding)
/// - `pkt[9 + i*3 .. 9 + i*3 + 3]` = `zone_colors[ZONE_MAP[i]]` for i in 0..11
/// - `pkt[45..64]` = 0x00 (Strict tail zero padding)
#[inline]
pub fn encode_direct_frame(zone_colors: &[[u8; 3]; LOGICAL_ZONE_COUNT]) -> [u8; REPORT_SIZE] {
    let mut pkt = [0u8; REPORT_SIZE];
    pkt[0] = AURA_REPORT_ID;
    pkt[1] = AURA_DIRECT_CMD;
    pkt[2] = 0x00;
    pkt[3] = 0x01;
    pkt[4] = AURA_DIRECT_4ZONE_CMD;

    let mut offset = PAYLOAD_OFFSET;
    for &logical_zone in &ZONE_MAP {
        let rgb = zone_colors[logical_zone];
        pkt[offset] = rgb[0];
        pkt[offset + 1] = rgb[1];
        pkt[offset + 2] = rgb[2];
        offset += 3;
    }
    // offsets 45..64 are initialized to 0x00
    pkt
}

/// Validates that a buffer conforms strictly to Aura Direct protocol constraints.
pub fn validate_packet(packet: &[u8]) -> Result<(), AuraError> {
    if packet.len() != REPORT_SIZE {
        return Err(AuraError::InvalidBufferSize {
            expected: REPORT_SIZE,
            actual: packet.len(),
        });
    }
    if packet[0] != AURA_REPORT_ID {
        return Err(AuraError::InvalidReportId(packet[0]));
    }
    if packet[1] != AURA_DIRECT_CMD {
        return Err(AuraError::InvalidCommand(packet[1]));
    }
    // Verify tail padding is strictly zero
    if packet[TAIL_PADDING_OFFSET..REPORT_SIZE].iter().any(|&b| b != 0) {
        return Err(AuraError::InvalidBufferSize {
            expected: REPORT_SIZE,
            actual: packet.len(),
        });
    }
    Ok(())
}
