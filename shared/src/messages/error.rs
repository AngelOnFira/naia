use thiserror::Error;

/// Errors that can occur during message kind operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageKindsError {
    /// Network ID not found in registry
    #[error("Network ID {net_id} not found in message registry. Message type must be registered with Protocol via add_message()")]
    NetIdNotFound {
        net_id: u16,
    },

    /// Message kind not found in registry
    #[error("Message kind not found in registry. Message type must be registered with Protocol via add_message()")]
    MessageKindNotFound,
}

/// Errors that can occur during message manager operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageManagerError {
    /// Channel not configured for sending
    #[error("Channel {channel:?} not configured for sending. Check Protocol configuration and HostType permissions")]
    ChannelNotConfiguredForSending {
        channel: String,
    },

    /// Channel not configured for receiving
    #[error("Channel {channel:?} not configured for receiving. Check Protocol configuration and HostType permissions")]
    ChannelNotConfiguredForReceiving {
        channel: String,
    },

    /// Channel settings not found
    #[error("Channel settings not found for channel {channel:?}. This indicates an internal configuration error")]
    ChannelSettingsNotFound {
        channel: String,
    },

    /// Message exceeds fragmentation limit on unreliable channel
    #[error("Message of {bit_length} bits exceeds fragmentation limit of {limit} bits on unreliable channel {channel:?}. Use a reliable channel or reduce message size")]
    FragmentationLimitExceeded {
        bit_length: u32,
        limit: u32,
        channel: String,
    },

    /// Packet index not found in message map
    #[error("Packet index {packet_index} not found in message map. This indicates an internal state error")]
    PacketIndexNotFound {
        packet_index: u16,
    },

    /// Failed to process incoming response
    #[error("Failed to process incoming response for local request ID. Request may have expired or was never sent")]
    ResponseProcessingFailed,
}

/// Errors that can occur during message container operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageContainerError {
    /// Attempted to get bit_length on a MessageContainer created from read
    #[error("Cannot get bit_length on MessageContainer created from read operation. bit_length is only available for MessageContainers created for writing")]
    BitLengthNotAvailable,
}

/// Errors that can occur during channel operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChannelError {
    /// Channel kind not found in registry
    #[error("Channel kind not found in registry. Channel type must be registered with Protocol via add_channel()")]
    ChannelKindNotFound,

    /// Network ID not found in channel registry
    #[error("Network ID {net_id} not found in channel registry. Channel type must be registered with Protocol via add_channel()")]
    NetIdNotFound {
        net_id: u16,
    },

    /// Invalid channel configuration
    #[error("TickBuffered channels are only allowed to be sent from Client to Server")]
    InvalidTickBufferedDirection,
}

/// Errors that can occur during message fragmentation
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FragmentationError {
    /// Fragment index limit exceeded
    #[error("Fragment index limit of {limit} exceeded. Attempting to transmit approximately {estimated_mb} MB, which exceeds practical transmission limits. Consider breaking the data into smaller messages")]
    FragmentLimitExceeded {
        limit: u32,
        estimated_mb: usize,
    },
}

/// General message-level errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageError {
    /// Message kinds error
    #[error("Message kinds error: {0}")]
    MessageKinds(#[from] MessageKindsError),

    /// Message manager error
    #[error("Message manager error: {0}")]
    MessageManager(#[from] MessageManagerError),

    /// Message container error
    #[error("Message container error: {0}")]
    MessageContainer(#[from] MessageContainerError),

    /// Channel error
    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    /// Fragmentation error
    #[error("Fragmentation error: {0}")]
    Fragmentation(#[from] FragmentationError),
}
