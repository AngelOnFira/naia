use thiserror::Error;

/// Errors that can occur during message sender operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SenderError {
    /// Message queue is empty when trying to access next message
    #[error("Message queue is empty. Cannot access message from empty queue")]
    EmptyMessageQueue,

    /// Message index difference is negative (ordering violation)
    #[error("Message Index diff is negative in subsequent message. Previous: {previous}, Current: {current}, Diff: {diff}. This indicates an internal sequencing error")]
    NegativeIndexDiff {
        previous: u16,
        current: u16,
        diff: i16,
    },

    /// Message is too large to fit in packet (blocking overflow)
    #[error("Blocking overflow detected! Message requires {bits_needed} bits, but packet only has {bits_free} bits available. Large Messages should be Fragmented in the Reliable channel")]
    MessageTooLarge {
        bits_needed: u32,
        bits_free: u32,
    },

    /// Message is too large to fit in unreliable packet
    #[error("Blocking overflow detected! Message of type `{message_name}` requires {bits_needed} bits, but packet only has {bits_free} bits available. Recommended to slim down this Message, or send over a Reliable channel for Fragmentation")]
    UnreliableMessageTooLarge {
        message_name: String,
        bits_needed: u32,
        bits_free: u32,
    },

    /// count_bits method called on FragmentWriter (should only be used by BitCounter)
    #[error("count_bits() method should only be used by BitCounter, not FragmentWriter. This indicates incorrect usage of the BitWrite trait")]
    InvalidCountBitsUsage,

    /// LocalRequestOrResponseId is a response when request expected
    #[error("LocalRequestOrResponseId is a response, but a request was expected. This indicates a protocol mismatch")]
    ExpectedRequest,

    /// LocalRequestOrResponseId is a request when response expected
    #[error("LocalRequestOrResponseId is a request, but a response was expected. This indicates a protocol mismatch")]
    ExpectedResponse,

    /// Channel does not support request/response pattern
    #[error("{channel_type} channels do not support request/response pattern. Use a reliable channel for requests")]
    RequestsNotSupported {
        channel_type: &'static str,
    },

    /// Internal state inconsistency in sender
    #[error("Internal sender state inconsistency: {reason}. This indicates a bug in the sender implementation")]
    StateInconsistency {
        reason: &'static str,
    },
}
