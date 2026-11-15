//! Unit and Contract Verification Tests for ASUS Aura HID Protocol & Mock Device.

use aura_zone_ripple::hid::{
    encode_direct_frame, encode_handshake, validate_packet, AuraDevice, AuraError,
    MockAuraDevice, SharedMockAuraDevice, ZONE_MAP,
};

// ==============================================================================
// 1. Handshake Packet Invariant Tests
// ==============================================================================

#[test]
fn test_handshake_exact_length_64_bytes() {
    let pkt = encode_handshake();
    assert_eq!(pkt.len(), 64, "Handshake report must be exactly 64 bytes");
}

#[test]
fn test_handshake_header_bytes() {
    let pkt = encode_handshake();
    assert_eq!(pkt[0], 0x5D, "Byte 0 must be Report ID 0x5D");
    assert_eq!(pkt[1], 0xBC, "Byte 1 must be Direct Command 0xBC");
    assert_eq!(pkt[2], 0x01, "Byte 2 must be Handshake Subcode 0x01");
}

#[test]
fn test_handshake_zero_padded_remainder() {
    let pkt = encode_handshake();
    for i in 3..64 {
        assert_eq!(
            pkt[i], 0x00,
            "Handshake remainder byte at offset {} must be 0x00, got 0x{:02X}",
            i, pkt[i]
        );
    }
}

// ==============================================================================
// 2. Direct Frame Packet Invariant Tests
// ==============================================================================

#[test]
fn test_direct_frame_exact_length_64_bytes() {
    let zones = [[0u8; 3]; 8];
    let pkt = encode_direct_frame(&zones);
    assert_eq!(pkt.len(), 64, "Direct frame report must be exactly 64 bytes");
}

#[test]
fn test_direct_frame_header_bytes() {
    let zones = [[10, 20, 30]; 8];
    let pkt = encode_direct_frame(&zones);
    assert_eq!(pkt[0], 0x5D, "Byte 0 must be Report ID 0x5D");
    assert_eq!(pkt[1], 0xBC, "Byte 1 must be Direct Command 0xBC");
    assert_eq!(pkt[2], 0x00, "Byte 2 must be 0x00");
    assert_eq!(pkt[3], 0x01, "Byte 3 must be 0x01");
    assert_eq!(pkt[4], 0x04, "Byte 4 must be 4-Zone Command Code 0x04");
}

#[test]
fn test_direct_frame_header_padding() {
    let zones = [[255, 255, 255]; 8];
    let pkt = encode_direct_frame(&zones);
    for i in 5..9 {
        assert_eq!(
            pkt[i], 0x00,
            "Header padding at offset {} must be 0x00, got 0x{:02X}",
            i, pkt[i]
        );
    }
}

#[test]
fn test_direct_frame_tail_padding_strictly_zero() {
    let zones = [[255, 255, 255]; 8];
    let pkt = encode_direct_frame(&zones);
    for i in 45..64 {
        assert_eq!(
            pkt[i], 0x00,
            "Tail padding at offset {} must be strictly 0x00, got 0x{:02X}",
            i, pkt[i]
        );
    }
}

// ==============================================================================
// 3. 12-to-8 Slot Mapping Verification
// ==============================================================================

#[test]
fn test_zone_map_constants() {
    assert_eq!(ZONE_MAP.len(), 12, "Hardware zone map must contain 12 slots");
    assert_eq!(
        ZONE_MAP,
        [0, 1, 2, 3, 0, 0, 7, 7, 6, 5, 4, 4],
        "ZONE_MAP must match ASUS ROG Strix G513 hardware layout"
    );
}

#[test]
fn test_12_to_8_distinct_color_mapping() {
    // Assign a unique, identifiable RGB triplet to each of the 8 logical zones
    let distinct_zones: [[u8; 3]; 8] = [
        [10, 20, 30],    // Zone 0: Left Keyboard
        [40, 50, 60],    // Zone 1: Mid-Left Keyboard
        [70, 80, 90],    // Zone 2: Mid-Right Keyboard
        [100, 110, 120], // Zone 3: Right Keyboard
        [130, 140, 150], // Zone 4: Lightbar Left-Wrap
        [160, 170, 180], // Zone 5: Lightbar Mid-Left
        [190, 200, 210], // Zone 6: Lightbar Mid-Right
        [220, 230, 240], // Zone 7: Lightbar Right-Wrap
    ];

    let pkt = encode_direct_frame(&distinct_zones);

    // Verify each of the 12 hardware slots against ZONE_MAP
    for (slot_idx, &zone_idx) in ZONE_MAP.iter().enumerate() {
        let expected_rgb = distinct_zones[zone_idx];
        let offset = 9 + slot_idx * 3;
        assert_eq!(
            [pkt[offset], pkt[offset + 1], pkt[offset + 2]],
            expected_rgb,
            "Slot {} (mapped to Zone {}) at offset {} expected RGB {:?}, got {:?}",
            slot_idx, zone_idx, offset, expected_rgb,
            [pkt[offset], pkt[offset + 1], pkt[offset + 2]]
        );
    }

    // Explicitly verify mirroring rules:
    // Slots 4 and 5 must duplicate Zone 0
    assert_eq!([pkt[21], pkt[22], pkt[23]], distinct_zones[0], "Slot 4 must mirror Zone 0");
    assert_eq!([pkt[24], pkt[25], pkt[26]], distinct_zones[0], "Slot 5 must mirror Zone 0");

    // Slot 7 must duplicate Zone 7 (Slot 6)
    assert_eq!([pkt[27], pkt[28], pkt[29]], distinct_zones[7], "Slot 6 is Zone 7");
    assert_eq!([pkt[30], pkt[31], pkt[32]], distinct_zones[7], "Slot 7 must mirror Zone 7");

    // Slot 11 must duplicate Zone 4 (Slot 10)
    assert_eq!([pkt[39], pkt[40], pkt[41]], distinct_zones[4], "Slot 10 is Zone 4");
    assert_eq!([pkt[42], pkt[43], pkt[44]], distinct_zones[4], "Slot 11 must mirror Zone 4");
}

// ==============================================================================
// 4. Mock Device Lifecycle, Recording & Replay Tests
// ==============================================================================

#[test]
fn test_mock_device_initial_state() {
    let mock = MockAuraDevice::new();
    assert_eq!(mock.packet_count(), 0);
    assert!(mock.sent_packets().is_empty());
    assert!(mock.latest_packet().is_none());
    assert!(mock.latest_zones().is_none());
    assert!(mock.is_connected());
}

#[test]
fn test_mock_device_records_handshake() {
    let mut mock = MockAuraDevice::new();
    let handshake = encode_handshake();
    mock.send_feature(&handshake).expect("Handshake dispatch should succeed");

    assert_eq!(mock.packet_count(), 1);
    assert_eq!(mock.latest_packet(), Some(&handshake));
    assert_eq!(mock.sent_packets()[0], handshake);
}

#[test]
fn test_mock_device_apply_zones_recording() {
    let mut mock = MockAuraDevice::new();
    let test_colors = [[50, 100, 150]; 8];

    mock.apply_zones(&test_colors).expect("apply_zones should succeed");

    assert_eq!(mock.packet_count(), 1);
    assert_eq!(mock.latest_zones(), Some(&test_colors));

    let recorded = mock.latest_packet().unwrap();
    let expected = encode_direct_frame(&test_colors);
    assert_eq!(recorded, &expected);
}

#[test]
fn test_mock_device_set_static_uniform() {
    let mut mock = MockAuraDevice::new();
    let red = [255, 0, 0];

    mock.set_static_uniform(red).expect("set_static_uniform should succeed");

    assert_eq!(mock.packet_count(), 1);
    assert_eq!(mock.latest_zones(), Some(&[red; 8]));

    let recorded = mock.latest_packet().unwrap();
    let expected = encode_direct_frame(&[red; 8]);
    assert_eq!(recorded, &expected);
}

#[test]
fn test_mock_device_error_injection() {
    let mut mock = MockAuraDevice::new();
    mock.inject_error("Simulated I/O Bus Timeout");

    let pkt = encode_handshake();
    let result = mock.send_feature(&pkt);
    assert!(result.is_err());
    match result {
        Err(AuraError::InjectedError(msg)) => assert_eq!(msg, "Simulated I/O Bus Timeout"),
        other => panic!("Expected InjectedError, got {:?}", other),
    }

    // Packets should not be added when error occurs
    assert_eq!(mock.packet_count(), 0);

    // Clear error and verify recovery
    mock.clear_error();
    assert!(mock.send_feature(&pkt).is_ok());
    assert_eq!(mock.packet_count(), 1);
}

#[test]
fn test_mock_device_disconnected_behavior() {
    let mut mock = MockAuraDevice::new();
    mock.set_connected(false);

    let pkt = encode_handshake();
    let result = mock.send_feature(&pkt);
    assert_eq!(result, Err(AuraError::DeviceClosed));
}

#[test]
fn test_mock_device_clear_history() {
    let mut mock = MockAuraDevice::new();
    mock.send_feature(&encode_handshake()).unwrap();
    mock.set_static_uniform([0, 255, 0]).unwrap();
    assert_eq!(mock.packet_count(), 2);

    mock.clear_packets();
    assert_eq!(mock.packet_count(), 0);
    assert!(mock.sent_packets().is_empty());
}

// ==============================================================================
// 5. Packet Validation & Boundary Rejection Tests
// ==============================================================================

#[test]
fn test_validate_valid_packets() {
    assert!(validate_packet(&encode_handshake()).is_ok());
    assert!(validate_packet(&encode_direct_frame(&[[0; 3]; 8])).is_ok());
}

#[test]
fn test_validate_rejects_invalid_report_id() {
    let mut pkt = encode_handshake();
    pkt[0] = 0x5C; // Invalid Report ID
    assert_eq!(validate_packet(&pkt), Err(AuraError::InvalidReportId(0x5C)));
}

#[test]
fn test_validate_rejects_invalid_command() {
    let mut pkt = encode_handshake();
    pkt[1] = 0xBD; // Invalid Command
    assert_eq!(validate_packet(&pkt), Err(AuraError::InvalidCommand(0xBD)));
}

#[test]
fn test_validate_rejects_dirty_tail_padding() {
    let mut pkt = encode_direct_frame(&[[0; 3]; 8]);
    pkt[50] = 0xFF; // Corrupt tail padding
    assert!(matches!(
        validate_packet(&pkt),
        Err(AuraError::InvalidBufferSize { .. })
    ));
}

#[test]
fn test_shared_mock_device_thread_safety() {
    let shared = SharedMockAuraDevice::new();
    let mut shared_clone = shared.clone();

    let handle = std::thread::spawn(move || {
        for _ in 0..10 {
            shared_clone.set_static_uniform([0, 0, 255]).unwrap();
        }
    });

    handle.join().unwrap();
    assert_eq!(shared.packet_count(), 10);
    assert_eq!(shared.latest_zones(), Some([[0, 0, 255]; 8]));
}
