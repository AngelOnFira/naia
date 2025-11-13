/// Integration tests for Decoder error handling
///
/// This test file verifies that all panic points in decoder.rs have been
/// replaced with proper error handling via try_* methods.
///
/// SECURITY: The Decoder is a CRITICAL security boundary as it processes
/// untrusted network data. These tests focus on ensuring that malformed or
/// malicious payloads cannot crash the system.

#[cfg(feature = "zstd_support")]
mod zstd_tests {
    use naia_shared::{CompressionMode, Decoder, DecoderError, Encoder};

    // ========== Error Type Tests ==========

    #[test]
    fn test_decompressor_creation_failed_error() {
        let error = DecoderError::DecompressorCreationFailed;
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to create decompressor"));
    }

    #[test]
    fn test_decompressor_with_dictionary_failed_error() {
        let error = DecoderError::DecompressorWithDictionaryFailed;
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to create decompressor with dictionary"));
    }

    #[test]
    fn test_upper_bound_calculation_failed_error() {
        let error = DecoderError::UpperBoundCalculationFailed { payload_size: 512 };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to calculate upper bound"));
        assert!(msg.contains("512"));
    }

    #[test]
    fn test_decompression_failed_error() {
        let error = DecoderError::DecompressionFailed { payload_size: 2048 };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to decompress payload"));
        assert!(msg.contains("2048"));
        assert!(msg.contains("malformed") || msg.contains("malicious"));
    }

    // ========== Decoder::try_new Tests ==========

    #[test]
    fn test_decoder_try_new_with_training_mode() {
        let mode = CompressionMode::Training(1000);
        let result = Decoder::try_new(mode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decoder_try_new_with_default_mode() {
        let mode = CompressionMode::Default(3);
        let result = Decoder::try_new(mode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decoder_try_new_with_dictionary() {
        // Create a simple dictionary
        let dictionary = vec![0u8; 1024];
        let mode = CompressionMode::Dictionary(3, dictionary);
        let result = Decoder::try_new(mode);
        // Empty dictionary might fail or succeed depending on zstd
        let _ = result; // Just ensure try_new exists and compiles
    }

    // ========== Decoder::new backward compatibility ==========

    #[test]
    fn test_decoder_new_still_works_training_mode() {
        let mode = CompressionMode::Training(1000);
        let _decoder = Decoder::new(mode);
        // Should not panic with valid mode
    }

    #[test]
    fn test_decoder_new_still_works_default_mode() {
        let mode = CompressionMode::Default(3);
        let _decoder = Decoder::new(mode);
        // Should not panic with valid mode
    }

    // ========== Decoder::try_decode Tests - Valid Data ==========

    #[test]
    fn test_decoder_try_decode_with_training_mode() {
        let mode = CompressionMode::Training(1000);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        let payload = b"Hello, World! This is test data.";
        let result = decoder.try_decode(payload);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        // In training mode, data is not compressed, just copied
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_decoder_try_decode_with_compressed_data() {
        let mode = CompressionMode::Default(3);

        // First, compress some data
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let original = b"Hello, World! This is test data that should be compressed and then decompressed.";
        let compressed = encoder.try_encode(original).expect("Failed to encode");

        // Now decompress it
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");
        let result = decoder.try_decode(compressed);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decoder_try_decode_empty_payload() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        let payload = b"";
        let result = decoder.try_decode(payload);
        // Empty payload behavior depends on zstd - it might succeed or fail
        // We just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_large_payload() {
        let mode = CompressionMode::Default(3);

        // Compress a large payload
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let original = vec![b'A'; 10000];
        let compressed = encoder.try_encode(&original).expect("Failed to encode");

        // Decompress it
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");
        let result = decoder.try_decode(compressed);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded, &original[..]);
    }

    // ========== SECURITY TESTS - Malicious/Malformed Data ==========

    #[test]
    fn test_decoder_try_decode_random_garbage() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Random garbage that is NOT valid compressed data
        let malicious_payload = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09";
        let result = decoder.try_decode(malicious_payload);

        // This should return an error, not panic
        // The specific behavior depends on zstd, but it should not crash
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_all_zeros() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // All zeros - not valid compressed data
        let malicious_payload = vec![0u8; 1024];
        let result = decoder.try_decode(&malicious_payload);

        // Should not panic, even if decompression fails
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_all_ones() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // All ones - not valid compressed data
        let malicious_payload = vec![0xFFu8; 1024];
        let result = decoder.try_decode(&malicious_payload);

        // Should not panic
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_single_byte() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Single byte - likely invalid compressed data
        let malicious_payload = b"X";
        let result = decoder.try_decode(malicious_payload);

        // Should not panic
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_truncated_data() {
        let mode = CompressionMode::Default(3);

        // Create valid compressed data
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let original = b"Hello, World! This is test data.";
        let compressed = encoder.try_encode(original).expect("Failed to encode");

        // Truncate the compressed data (simulate network corruption)
        let truncated = if compressed.len() > 5 {
            &compressed[..compressed.len() - 5]
        } else {
            &compressed[..1]
        };

        // Try to decompress truncated data
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");
        let result = decoder.try_decode(truncated);

        // Should not panic, even though data is corrupted
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_oversized_claim() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Craft data that might claim to decompress to huge size
        // This is a basic test - real malicious payloads would be more sophisticated
        let malicious_payload = vec![0x28, 0xB5, 0x2F, 0xFD]; // zstd magic with garbage
        let result = decoder.try_decode(&malicious_payload);

        // Should not panic or consume excessive memory
        let _ = result;
    }

    #[test]
    fn test_decoder_try_decode_repeated_attempts() {
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Attempt to decode multiple bad payloads in sequence
        for _ in 0..10 {
            let malicious_payload = b"\x00\x01\x02\x03";
            let _ = decoder.try_decode(malicious_payload);
            // Should not panic or leak memory
        }
    }

    // ========== Decoder::decode backward compatibility ==========

    #[test]
    fn test_decoder_decode_still_works() {
        let mode = CompressionMode::Default(3);

        // Compress data
        let mut encoder = Encoder::new(mode.clone());
        let original = b"Hello, World!";
        let compressed = encoder.encode(original);

        // Decompress using the non-try method
        let mut decoder = Decoder::new(mode);
        let decoded = decoder.decode(compressed);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decoder_multiple_decodes() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Decode multiple payloads
        let payload1 = b"First payload";
        let compressed1 = encoder.try_encode(payload1).expect("Failed to encode");
        let result1 = decoder.try_decode(compressed1);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), payload1);

        let payload2 = b"Second payload with different data";
        let compressed2 = encoder.try_encode(payload2).expect("Failed to encode");
        let result2 = decoder.try_decode(compressed2);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), payload2);
    }

    // ========== Round-trip Tests ==========

    #[test]
    fn test_encode_decode_roundtrip() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        let original = b"The quick brown fox jumps over the lazy dog.";
        let compressed = encoder.try_encode(original).expect("Failed to encode");
        let decompressed = decoder.try_decode(compressed).expect("Failed to decode");

        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_encode_decode_roundtrip_binary_data() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode.clone()).expect("Failed to create encoder");
        let mut decoder = Decoder::try_new(mode).expect("Failed to create decoder");

        // Test with binary data including null bytes
        let original = vec![0u8, 1, 2, 255, 254, 0, 127, 128];
        let compressed = encoder.try_encode(&original).expect("Failed to encode");
        let decompressed = decoder.try_decode(compressed).expect("Failed to decode");

        assert_eq!(decompressed, &original[..]);
    }

    // ========== Error Properties Tests ==========

    #[test]
    fn test_decoder_error_contains_security_context() {
        let error = DecoderError::DecompressionFailed { payload_size: 1024 };
        let error_string = error.to_string();

        // Error should mention potential security issues
        assert!(error_string.contains("malformed") || error_string.contains("malicious"));
        assert!(error_string.contains("1024"));
    }

    #[test]
    fn test_decoder_error_is_cloneable() {
        let error = DecoderError::DecompressionFailed { payload_size: 512 };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_decoder_error_is_debug() {
        let error = DecoderError::DecompressorCreationFailed;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("DecompressorCreationFailed"));
    }
}

// ========== Non-zstd Tests (fallback implementation) ==========

#[cfg(not(feature = "zstd_support"))]
mod no_zstd_tests {
    use naia_shared::{CompressionMode, Decoder};

    #[test]
    fn test_decoder_without_zstd_support() {
        // Without zstd_support, decoder should just pass through data
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::new(mode);

        let payload = b"Hello, World!";
        let decoded = decoder.decode(payload);

        // Without compression, output should match input
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_decoder_without_zstd_training_mode() {
        let mode = CompressionMode::Training(1000);
        let mut decoder = Decoder::new(mode);

        let payload = b"Training data";
        let decoded = decoder.decode(payload);

        // Without compression, output should match input
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_decoder_without_zstd_malicious_data() {
        // Even without zstd, we should handle any input safely
        let mode = CompressionMode::Default(3);
        let mut decoder = Decoder::new(mode);

        let malicious = b"\x00\x01\x02\xFF\xFE\xFD";
        let decoded = decoder.decode(malicious);

        // Should just pass through without panicking
        assert_eq!(decoded, malicious);
    }
}
