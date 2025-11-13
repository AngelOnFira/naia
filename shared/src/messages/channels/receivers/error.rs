use thiserror::Error;

/// Errors that can occur during message receiver operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiverError {
    /// Non-fragmented message received by FragmentReceiver
    #[error("Received non-fragmented message in FragmentReceiver. Only fragmented messages should be processed by this receiver")]
    NonFragmentedMessage,

    /// Failed to downcast message to expected type
    #[error("Failed to downcast message to expected type {expected_type}. Message type mismatch indicates corrupted or malicious data")]
    MessageDowncastFailed {
        expected_type: &'static str,
    },

    /// Fragment ID not found in receiver map
    #[error("Fragment ID not found in receiver map. This indicates an internal state error")]
    FragmentIdNotFound,

    /// Duplicate first fragment received
    #[error("Received duplicate first fragment (index 0) for fragment ID. Fragment reassembly protocol violation")]
    DuplicateFirstFragment,

    /// First fragment metadata missing when reassembling
    #[error("First fragment metadata missing during reassembly. All fragments received but first fragment metadata was never set")]
    FirstFragmentMetadataMissing,

    /// Failed to read reassembled fragmented message
    #[error("Failed to read reassembled fragmented message: {reason}. Message data may be corrupted or malicious")]
    FragmentedMessageReadFailed {
        reason: &'static str,
    },

    /// Failed to read request or response message
    #[error("Failed to read request or response message: {reason}. Message data may be corrupted or malicious")]
    RequestOrResponseReadFailed {
        reason: &'static str,
    },

    /// Buffer inconsistency detected in ordered receiver
    #[error("Buffer inconsistency detected: {reason}. This indicates an internal ordering error")]
    BufferInconsistency {
        reason: &'static str,
    },

    /// Channel does not support request/response pattern
    #[error("{channel_type} channels do not support request/response pattern. Use a reliable channel for requests")]
    RequestsNotSupported {
        channel_type: &'static str,
    },
}
