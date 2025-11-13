use thiserror::Error;

/// Errors that can occur during protocol operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    /// Protocol is locked and cannot be modified
    #[error("Protocol is already locked and cannot be modified. Protocol.lock() has been called and no further changes are allowed")]
    AlreadyLocked,
}
