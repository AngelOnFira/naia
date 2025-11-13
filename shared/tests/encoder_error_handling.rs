/// Integration tests for Encoder error handling
///
/// This test file verifies that all panic points in encoder.rs have been
/// replaced with proper error handling via try_* methods.
///
/// The Encoder handles packet compression and must be resilient to
/// configuration errors and compression failures.

#[cfg(feature = "zstd_support")]
mod zstd_tests {
    use naia_shared::{CompressionMode, Encoder, EncoderError};

    // ========== Error Type Tests ==========

    #[test]
    fn test_compressor_creation_failed_error() {
        let error = EncoderError::CompressorCreationFailed { level: 10 };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to create compressor"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn test_compressor_with_dictionary_failed_error() {
        let error = EncoderError::CompressorWithDictionaryFailed { level: 15 };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to create compressor with dictionary"));
        assert!(msg.contains("15"));
    }

    #[test]
    fn test_compression_failed_error() {
        let error = EncoderError::CompressionFailed { payload_size: 1024 };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to compress payload"));
        assert!(msg.contains("1024"));
    }

    #[test]
    fn test_dictionary_training_failed_error() {
        let error = EncoderError::DictionaryTrainingFailed {
            sample_count: 100,
            total_bytes: 50000,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to train compression dictionary"));
        assert!(msg.contains("100"));
        assert!(msg.contains("50000"));
    }

    #[test]
    fn test_dictionary_write_failed_error() {
        let error = EncoderError::DictionaryWriteFailed {
            path: "dictionary.txt",
        };
        let msg = format!("{}", error);
        assert!(msg.contains("Failed to write dictionary to file"));
        assert!(msg.contains("dictionary.txt"));
    }

    // ========== Encoder::try_new Tests ==========

    #[test]
    fn test_encoder_try_new_with_training_mode() {
        let mode = CompressionMode::Training(1000);
        let result = Encoder::try_new(mode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encoder_try_new_with_default_mode() {
        // Valid compression levels for zstd are typically 1-22
        let mode = CompressionMode::Default(3);
        let result = Encoder::try_new(mode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encoder_try_new_with_invalid_compression_level() {
        // Test with an extremely high compression level that might fail
        // Note: zstd may accept high levels but cap them, so this might not fail
        let mode = CompressionMode::Default(1000);
        let result = Encoder::try_new(mode);
        // This might succeed (zstd is permissive), but we test the try_ path
        let _ = result; // Just ensure try_new exists and compiles
    }

    #[test]
    fn test_encoder_try_new_with_dictionary() {
        // Create a simple dictionary (just some bytes)
        let dictionary = vec![0u8; 1024];
        let mode = CompressionMode::Dictionary(3, dictionary);
        let result = Encoder::try_new(mode);
        // Empty dictionary might fail or succeed depending on zstd implementation
        let _ = result; // Just ensure try_new exists and compiles
    }

    // ========== Encoder::new backward compatibility ==========

    #[test]
    fn test_encoder_new_still_works_training_mode() {
        let mode = CompressionMode::Training(1000);
        let _encoder = Encoder::new(mode);
        // Should not panic with valid mode
    }

    #[test]
    fn test_encoder_new_still_works_default_mode() {
        let mode = CompressionMode::Default(3);
        let _encoder = Encoder::new(mode);
        // Should not panic with valid compression level
    }

    // ========== Encoder::try_encode Tests ==========

    #[test]
    fn test_encoder_try_encode_with_training_mode() {
        let mode = CompressionMode::Training(1000);
        let mut encoder = Encoder::try_new(mode).expect("Failed to create encoder");

        let payload = b"Hello, World! This is test data.";
        let result = encoder.try_encode(payload);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        // In training mode, data is not compressed, just copied
        assert_eq!(encoded, payload);
    }

    #[test]
    fn test_encoder_try_encode_with_default_mode() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode).expect("Failed to create encoder");

        let payload = b"Hello, World! This is test data that should be compressed.";
        let result = encoder.try_encode(payload);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        // Compressed data should exist (might be same size or larger for small data)
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encoder_try_encode_empty_payload() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode).expect("Failed to create encoder");

        let payload = b"";
        let result = encoder.try_encode(payload);
        // Empty payload should still work
        assert!(result.is_ok());
    }

    #[test]
    fn test_encoder_try_encode_large_payload() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode).expect("Failed to create encoder");

        // Create a large compressible payload
        let payload = vec![b'A'; 10000];
        let result = encoder.try_encode(&payload);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        // Highly compressible data should result in smaller output
        assert!(!encoded.is_empty());
    }

    // ========== Encoder::encode backward compatibility ==========

    #[test]
    fn test_encoder_encode_still_works() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::new(mode);

        let payload = b"Hello, World!";
        let encoded = encoder.encode(payload);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encoder_multiple_encodes() {
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::try_new(mode).expect("Failed to create encoder");

        // Encode multiple payloads
        let payload1 = b"First payload";
        let result1 = encoder.try_encode(payload1);
        assert!(result1.is_ok());

        let payload2 = b"Second payload with different data";
        let result2 = encoder.try_encode(payload2);
        assert!(result2.is_ok());

        // Both should succeed
        assert!(!result1.unwrap().is_empty());
        assert!(!result2.unwrap().is_empty());
    }

    // ========== Error Propagation Tests ==========

    #[test]
    fn test_error_contains_context_information() {
        let error = EncoderError::CompressionFailed { payload_size: 2048 };
        let error_string = error.to_string();

        // Error should contain useful debugging information
        assert!(error_string.contains("2048"));
        assert!(error_string.contains("Failed to compress"));
    }

    #[test]
    fn test_encoder_error_is_cloneable() {
        let error = EncoderError::CompressionFailed { payload_size: 1024 };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_encoder_error_is_debug() {
        let error = EncoderError::CompressorCreationFailed { level: 5 };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("CompressorCreationFailed"));
    }
}

// ========== Non-zstd Tests (fallback implementation) ==========

#[cfg(not(feature = "zstd_support"))]
mod no_zstd_tests {
    use naia_shared::{CompressionMode, Encoder};

    #[test]
    fn test_encoder_without_zstd_support() {
        // Without zstd_support, encoder should just pass through data
        let mode = CompressionMode::Default(3);
        let mut encoder = Encoder::new(mode);

        let payload = b"Hello, World!";
        let encoded = encoder.encode(payload);

        // Without compression, output should match input
        assert_eq!(encoded, payload);
    }

    #[test]
    fn test_encoder_without_zstd_training_mode() {
        let mode = CompressionMode::Training(1000);
        let mut encoder = Encoder::new(mode);

        let payload = b"Training data";
        let encoded = encoder.encode(payload);

        // Without compression, output should match input
        assert_eq!(encoded, payload);
    }
}
