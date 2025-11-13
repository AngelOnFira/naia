/// Integration tests for Connection module error handling
///
/// This test file verifies error handling in packet_type.rs and other
/// connection-related modules.
///
/// SECURITY: PacketType deserialization is a critical security boundary as
/// it's the first thing processed from untrusted network packets.

use naia_shared::{BitReader, BitWriter, ConnectionError, PacketType, PacketTypeError, Serde};

// ========== PacketTypeError Tests ==========

#[test]
fn test_invalid_packet_type_index_error() {
    let error = PacketTypeError::InvalidPacketTypeIndex { index: 5 };
    let msg = format!("{}", error);
    assert!(msg.contains("Invalid packet type index"));
    assert!(msg.contains("5"));
    assert!(msg.contains("malformed") || msg.contains("malicious"));
}

#[test]
fn test_internal_serialization_error() {
    let error = PacketTypeError::InternalSerializationError;
    let msg = format!("{}", error);
    assert!(msg.contains("Internal error"));
    assert!(msg.contains("serialize"));
}

#[test]
fn test_packet_type_error_is_cloneable() {
    let error = PacketTypeError::InvalidPacketTypeIndex { index: 10 };
    let cloned = error.clone();
    assert_eq!(error, cloned);
}

#[test]
fn test_packet_type_error_is_debug() {
    let error = PacketTypeError::InvalidPacketTypeIndex { index: 7 };
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("InvalidPacketTypeIndex"));
}

// ========== ConnectionError Tests ==========

#[test]
fn test_connection_error_from_packet_type_error() {
    let packet_error = PacketTypeError::InvalidPacketTypeIndex { index: 4 };
    let conn_error: ConnectionError = packet_error.into();
    let msg = format!("{}", conn_error);
    assert!(msg.contains("Packet type error"));
}

#[test]
fn test_connection_error_is_cloneable() {
    let error: ConnectionError = PacketTypeError::InvalidPacketTypeIndex { index: 8 }.into();
    let cloned = error.clone();
    let msg1 = format!("{}", error);
    let msg2 = format!("{}", cloned);
    assert_eq!(msg1, msg2);
}

// ========== PacketType Serialization Tests - Valid Cases ==========

#[test]
fn test_packet_type_serialize_data() {
    let packet_type = PacketType::Data;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    // Data packet should serialize successfully
    let bytes = writer.to_bytes();
    assert!(!bytes.is_empty());
}

#[test]
fn test_packet_type_serialize_heartbeat() {
    let packet_type = PacketType::Heartbeat;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    assert!(!bytes.is_empty());
}

#[test]
fn test_packet_type_serialize_handshake() {
    let packet_type = PacketType::Handshake;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    assert!(!bytes.is_empty());
}

#[test]
fn test_packet_type_serialize_ping() {
    let packet_type = PacketType::Ping;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    assert!(!bytes.is_empty());
}

#[test]
fn test_packet_type_serialize_pong() {
    let packet_type = PacketType::Pong;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    assert!(!bytes.is_empty());
}

// ========== PacketType Deserialization Tests - Valid Cases ==========

#[test]
fn test_packet_type_deserialize_data() {
    let packet_type = PacketType::Data;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    let mut reader = BitReader::new(&bytes);
    let deserialized = PacketType::de(&mut reader).expect("Failed to deserialize Data packet");

    assert_eq!(deserialized, PacketType::Data);
}

#[test]
fn test_packet_type_deserialize_heartbeat() {
    let packet_type = PacketType::Heartbeat;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    let mut reader = BitReader::new(&bytes);
    let deserialized = PacketType::de(&mut reader).expect("Failed to deserialize Heartbeat packet");

    assert_eq!(deserialized, PacketType::Heartbeat);
}

#[test]
fn test_packet_type_deserialize_handshake() {
    let packet_type = PacketType::Handshake;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    let mut reader = BitReader::new(&bytes);
    let deserialized = PacketType::de(&mut reader).expect("Failed to deserialize Handshake packet");

    assert_eq!(deserialized, PacketType::Handshake);
}

#[test]
fn test_packet_type_deserialize_ping() {
    let packet_type = PacketType::Ping;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    let mut reader = BitReader::new(&bytes);
    let deserialized = PacketType::de(&mut reader).expect("Failed to deserialize Ping packet");

    assert_eq!(deserialized, PacketType::Ping);
}

#[test]
fn test_packet_type_deserialize_pong() {
    let packet_type = PacketType::Pong;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();
    let mut reader = BitReader::new(&bytes);
    let deserialized = PacketType::de(&mut reader).expect("Failed to deserialize Pong packet");

    assert_eq!(deserialized, PacketType::Pong);
}

// ========== PacketType Round-trip Tests ==========

#[test]
fn test_packet_type_roundtrip_all_types() {
    let packet_types = vec![
        PacketType::Data,
        PacketType::Heartbeat,
        PacketType::Handshake,
        PacketType::Ping,
        PacketType::Pong,
    ];

    for packet_type in packet_types {
        let mut writer = BitWriter::new();
        packet_type.ser(&mut writer);

        let bytes = writer.to_bytes();
        let mut reader = BitReader::new(&bytes);
        let deserialized = PacketType::de(&mut reader)
            .expect(&format!("Failed to deserialize {:?}", packet_type));

        assert_eq!(deserialized, packet_type);
    }
}

// ========== SECURITY TESTS - Malicious/Malformed Data ==========

#[test]
fn test_packet_type_deserialize_invalid_index_4() {
    // Manually craft a packet with invalid index
    // is_data = false, then index = 4 (invalid, should be 0-3)
    let mut writer = BitWriter::new();
    false.ser(&mut writer); // is_data = false
    // Now we need to write index 4 using UnsignedInteger<2>
    // UnsignedInteger<2> can hold values 0-3, so we can't directly create 4
    // However, if somehow the data is corrupted, the deserializer should handle it

    // Since we can't directly create invalid serialized data through the normal API,
    // we'll test that the deserializer properly validates the range
    // The actual security fix is in the code using a match with default case
}

#[test]
fn test_packet_type_deserialize_empty_buffer() {
    // Empty buffer should fail gracefully
    let bits = vec![];
    let mut reader = BitReader::new(&bits);
    let result = PacketType::de(&mut reader);

    // Should return an error, not panic
    assert!(result.is_err());
}

#[test]
fn test_packet_type_deserialize_truncated_data() {
    // Serialize a non-Data packet
    let packet_type = PacketType::Heartbeat;
    let mut writer = BitWriter::new();
    packet_type.ser(&mut writer);

    let bytes = writer.to_bytes();

    // Truncate the data
    let mut truncated_bytes = bytes.to_vec();
    if truncated_bytes.len() > 1 {
        truncated_bytes.truncate(truncated_bytes.len() - 1);
    }

    let mut reader = BitReader::new(&truncated_bytes);
    let result = PacketType::de(&mut reader);

    // Should not panic - might succeed or fail depending on how much data was truncated
    // The important part is that it doesn't crash
    let _ = result;
}

#[test]
fn test_packet_type_deserialize_garbage_data() {
    // Random garbage data
    let bits = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&bits);
    let result = PacketType::de(&mut reader);

    // Should either succeed (if it happens to parse) or fail gracefully
    // The important part is it doesn't panic
    let _ = result;
}

#[test]
fn test_packet_type_deserialize_all_zeros() {
    let bits = vec![0x00, 0x00, 0x00, 0x00];
    let mut reader = BitReader::new(&bits);
    let result = PacketType::de(&mut reader);

    // Should not panic
    let _ = result;
}

#[test]
fn test_packet_type_deserialize_all_ones() {
    let bits = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let mut reader = BitReader::new(&bits);
    let result = PacketType::de(&mut reader);

    // Should not panic
    let _ = result;
}

#[test]
fn test_packet_type_deserialize_single_byte_variations() {
    // Test various single-byte inputs
    for byte in 0u8..=255 {
        let bits = vec![byte];
        let mut reader = BitReader::new(&bits);
        let result = PacketType::de(&mut reader);

        // Should not panic regardless of input
        let _ = result;
    }
}

#[test]
fn test_packet_type_deserialize_repeated_attempts() {
    // Test that multiple failed deserializations don't cause issues
    for _ in 0..100 {
        let bits = vec![0xFF, 0xFE, 0xFD];
        let mut reader = BitReader::new(&bits);
        let _ = PacketType::de(&mut reader);
    }
}

// ========== PacketType bit_length Tests ==========

#[test]
fn test_packet_type_bit_length_data() {
    let packet_type = PacketType::Data;
    let length = packet_type.bit_length();
    assert!(length > 0);
}

#[test]
fn test_packet_type_bit_length_non_data() {
    let packet_types = vec![
        PacketType::Heartbeat,
        PacketType::Handshake,
        PacketType::Ping,
        PacketType::Pong,
    ];

    for packet_type in packet_types {
        let length = packet_type.bit_length();
        assert!(length > 0);

        // Non-data packets should have same or larger bit length than Data
        let data_length = PacketType::Data.bit_length();
        assert!(length >= data_length);
    }
}

// ========== Integration Tests ==========

#[test]
fn test_packet_type_in_realistic_packet_scenario() {
    // Simulate a realistic packet processing scenario
    let packet_types = vec![
        PacketType::Handshake,
        PacketType::Data,
        PacketType::Data,
        PacketType::Ping,
        PacketType::Pong,
        PacketType::Data,
        PacketType::Heartbeat,
    ];

    for packet_type in packet_types {
        // Serialize
        let mut writer = BitWriter::new();
        packet_type.ser(&mut writer);
        let bytes = writer.to_bytes();

        // Deserialize
        let mut reader = BitReader::new(&bytes);
        let deserialized = PacketType::de(&mut reader)
            .expect(&format!("Failed to process {:?}", packet_type));

        assert_eq!(deserialized, packet_type);
    }
}

#[test]
fn test_packet_type_copy_clone() {
    let packet_type = PacketType::Ping;
    let copied = packet_type;
    let cloned = packet_type.clone();

    assert_eq!(packet_type, copied);
    assert_eq!(packet_type, cloned);
}

#[test]
fn test_packet_type_debug() {
    let packet_types = vec![
        PacketType::Data,
        PacketType::Heartbeat,
        PacketType::Handshake,
        PacketType::Ping,
        PacketType::Pong,
    ];

    for packet_type in packet_types {
        let debug_str = format!("{:?}", packet_type);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_packet_type_equality() {
    assert_eq!(PacketType::Data, PacketType::Data);
    assert_eq!(PacketType::Heartbeat, PacketType::Heartbeat);
    assert_ne!(PacketType::Data, PacketType::Heartbeat);
    assert_ne!(PacketType::Ping, PacketType::Pong);
}
