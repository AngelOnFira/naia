use thiserror::Error;

/// Errors that can occur during connection encoding operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncoderError {
    /// Failed to create compressor with the specified configuration
    #[error("Failed to create compressor with compression level {level}")]
    CompressorCreationFailed {
        level: i32,
    },

    /// Failed to create compressor with dictionary
    #[error("Failed to create compressor with dictionary (compression level {level})")]
    CompressorWithDictionaryFailed {
        level: i32,
    },

    /// Compression operation failed
    #[error("Failed to compress payload of {payload_size} bytes")]
    CompressionFailed {
        payload_size: usize,
    },

    /// Dictionary training failed
    #[error("Failed to train compression dictionary from {sample_count} samples ({total_bytes} bytes)")]
    DictionaryTrainingFailed {
        sample_count: usize,
        total_bytes: usize,
    },

    /// Failed to write dictionary to file
    #[error("Failed to write dictionary to file: {path}")]
    DictionaryWriteFailed {
        path: &'static str,
    },
}

/// Errors that can occur during connection decoding operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecoderError {
    /// Failed to create decompressor
    #[error("Failed to create decompressor")]
    DecompressorCreationFailed,

    /// Failed to create decompressor with dictionary
    #[error("Failed to create decompressor with dictionary")]
    DecompressorWithDictionaryFailed,

    /// Failed to calculate upper bound for decompression
    #[error("Failed to calculate upper bound for payload of {payload_size} bytes")]
    UpperBoundCalculationFailed {
        payload_size: usize,
    },

    /// Decompression operation failed (SECURITY: potentially malicious payload)
    #[error("Failed to decompress payload of {payload_size} bytes (possible malformed or malicious data)")]
    DecompressionFailed {
        payload_size: usize,
    },
}

/// Errors that can occur during packet type serialization/deserialization
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PacketTypeError {
    /// Invalid packet type index received (SECURITY: potentially malicious packet)
    #[error("Invalid packet type index {index} received (valid range: 0-3). This may indicate a malformed or malicious packet")]
    InvalidPacketTypeIndex {
        index: u8,
    },

    /// Internal serialization error (should never happen if code is correct)
    #[error("Internal error: attempted to serialize packet type that was already handled")]
    InternalSerializationError,
}

/// General connection-level errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConnectionError {
    /// Encoder error
    #[error("Encoder error: {0}")]
    Encoder(#[from] EncoderError),

    /// Decoder error
    #[error("Decoder error: {0}")]
    Decoder(#[from] DecoderError),

    /// Packet type error
    #[error("Packet type error: {0}")]
    PacketType(#[from] PacketTypeError),
}
