use std::error::Error;
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct EntityDoesNotExistError;
impl Error for EntityDoesNotExistError {}
impl std::fmt::Display for EntityDoesNotExistError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Error while attempting to look-up an Entity value for conversion: Entity was not found!")
    }
}

/// Errors that can occur during entity operations and conversions
#[derive(Debug, Clone, ThisError, PartialEq, Eq)]
pub enum EntityError {
    /// Entity was not found during lookup or conversion
    #[error("Entity not found: {context}")]
    EntityNotFound {
        context: &'static str,
    },

    /// Entity conversion failed between different entity types
    #[error("Entity conversion failed: cannot convert {from} to {to}")]
    ConversionFailed {
        from: &'static str,
        to: &'static str,
    },

    /// Invalid entity type for operation (e.g., expected RemoteEntity but got HostEntity)
    #[error("Invalid entity type: expected {expected}, got {actual}")]
    InvalidEntityType {
        expected: &'static str,
        actual: &'static str,
    },

    /// Serialization/deserialization not supported for this entity type
    #[error("Serialization operation '{operation}' not supported for {entity_type}")]
    SerializationNotSupported {
        entity_type: &'static str,
        operation: &'static str,
    },

    /// Component not found on entity
    #[error("Component not found on entity: {context}")]
    ComponentNotFound {
        context: &'static str,
    },

    /// Invalid entity action attempted
    #[error("Invalid entity action: {action} - {reason}")]
    InvalidAction {
        action: &'static str,
        reason: &'static str,
    },

    /// Entity already exists when trying to spawn
    #[error("Entity already exists: {context}")]
    EntityAlreadyExists {
        context: &'static str,
    },

    /// Component already exists on entity when trying to insert
    #[error("Component already exists on entity: {context}")]
    ComponentAlreadyExists {
        context: &'static str,
    },

    /// Invalid state transition for entity
    #[error("Invalid entity state transition: {from_state} -> {to_state} via {operation}")]
    InvalidStateTransition {
        from_state: &'static str,
        to_state: &'static str,
        operation: &'static str,
    },

    /// Invalid entity channel operation
    #[error("Invalid entity channel operation: {operation} - {reason}")]
    InvalidChannelOperation {
        operation: &'static str,
        reason: &'static str,
    },

    /// Entity action out of sequence
    #[error("Entity action out of sequence: expected {expected}, got {actual}")]
    OutOfSequence {
        expected: String,
        actual: String,
    },

    /// Internal consistency error (should never happen in normal operation)
    #[error("Internal entity consistency error: {context}")]
    InternalConsistency {
        context: &'static str,
    },

    /// Entity already reserved when attempting to reserve again
    #[error("Entity already reserved: {entity_id}")]
    EntityAlreadyReserved {
        entity_id: String,
    },

    /// Entity reservation expired or was corrupted
    #[error("Entity reservation expired or corrupted: {entity_id}")]
    ReservationExpired {
        entity_id: String,
    },

    /// Entity reservation timeout queue inconsistency
    #[error("Reservation timeout queue corrupted: entity in timeout queue but not in reservation map")]
    ReservationQueueCorrupted,

    /// Entity already registered as a different type
    #[error("Entity already registered: {entity_id} exists as {existing_type}")]
    EntityAlreadyRegisteredAs {
        entity_id: String,
        existing_type: &'static str,
    },

    /// Entity expected to have host but doesn't
    #[error("Entity missing expected host: {entity_id} should have host entity")]
    MissingHostEntity {
        entity_id: String,
    },

    // LocalEntityMap Errors

    /// Entity is neither host nor remote
    #[error("Entity {entity_id} is neither host nor remote")]
    EntityNeitherHostNorRemote {
        entity_id: String,
    },

    /// Entity record corruption detected
    #[error("Entity record corruption: {message}")]
    EntityRecordCorruption {
        message: String,
    },

    /// Entity mapping inconsistency detected
    #[error("Entity {entity_id} mapping inconsistency: {details}")]
    EntityMappingInconsistency {
        entity_id: String,
        details: &'static str,
    },

    // EntityActionReceiver Errors

    /// Entity channel not found when expected (internal consistency error)
    #[error("Entity channel not found after insertion: {context}")]
    EntityChannelNotFound {
        context: &'static str,
    },

    /// Component channel not found when expected (internal consistency error)
    #[error("Component channel not found after insertion: {context}")]
    ComponentChannelNotFound {
        context: &'static str,
    },

    /// OrderedIds internal state corrupted
    #[error("OrderedIds internal corruption: index {index} out of bounds (length: {length})")]
    OrderedIdsCorrupted {
        index: usize,
        length: usize,
    },
}

// Implement From to allow migration from EntityDoesNotExistError to EntityError
impl From<EntityDoesNotExistError> for EntityError {
    fn from(_: EntityDoesNotExistError) -> Self {
        EntityError::EntityNotFound {
            context: "entity lookup failed during conversion",
        }
    }
}

/// Errors that can occur during entity authorization operations
#[derive(Debug, Clone, ThisError)]
pub enum EntityAuthError {
    /// Entity is already registered and cannot be registered again
    #[error("Entity {entity_id} is already registered with the auth handler")]
    EntityAlreadyRegistered {
        entity_id: String,
    },

    /// Entity is not registered when required
    #[error("Entity {entity_id} is not registered - operation '{operation}' requires registration")]
    EntityNotRegistered {
        entity_id: String,
        operation: &'static str,
    },

    /// Authority status lock is poisoned (internal consistency error)
    #[error("Auth status lock is poisoned - this indicates a panic occurred while holding the lock")]
    AuthLockPoisoned,

    /// Invalid authority state transition
    #[error("Invalid authority state transition for {host_type}: {from_state} -> {to_state} via {operation}")]
    InvalidAuthStateTransition {
        host_type: &'static str,
        from_state: &'static str,
        to_state: &'static str,
        operation: &'static str,
    },

    /// Operation not permitted in current authority state
    #[error("{operation} not permitted for {host_type} in state {current_state}")]
    OperationNotPermitted {
        operation: &'static str,
        host_type: &'static str,
        current_state: &'static str,
    },
}
