use thiserror::Error;

/// Errors that can occur during component operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ComponentError {
    /// Component kind not registered in the protocol
    #[error("Component not registered with Protocol. Must call `add_component()` during protocol initialization. Component: {component_name}")]
    ComponentNotRegistered {
        component_name: &'static str,
    },

    /// Net ID lookup failed (component not found in registry)
    #[error("Component net ID {net_id} not found in registry. Must properly initialize Component with Protocol via `add_component()` function")]
    NetIdNotFound {
        net_id: u16,
    },

    /// Component kind lookup failed (component type not found in registry)
    #[error("Component kind not found in registry. Must properly initialize Component with Protocol via `add_component()` function")]
    KindNotFound,
}

/// Errors that can occur during EntityProperty operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EntityPropertyError {
    /// Attempted to write an entity property type that should not be written
    #[error("EntityProperty of type '{property_type}' should never be written")]
    InvalidWriteOperation {
        property_type: &'static str,
    },

    /// Attempted to call set_mutator on wrong entity property type
    #[error("EntityProperty of type '{property_type}' cannot call set_mutator()")]
    InvalidMutatorOperation {
        property_type: &'static str,
    },

    /// Attempted to get bit_length for property type that should not be written
    #[error("EntityProperty of type '{property_type}' should never be written, so no need for their bit length")]
    BitLengthNotSupported {
        property_type: &'static str,
    },

    /// Attempted to manually set a remote entity property
    #[error("Remote EntityProperty should never be set manually")]
    RemotePropertyManualSet,

    /// Attempted to manually set an invalid entity property
    #[error("Invalid EntityProperty should never be set manually")]
    InvalidPropertyManualSet,

    /// Attempted to mirror an invalid entity property
    #[error("Invalid EntityProperty should never be mirrored")]
    InvalidPropertyMirror,

    /// Unknown read case encountered (should be unreachable in normal operation)
    #[error("Unknown read case for EntityProperty - this indicates corrupted state")]
    UnknownReadCase,

    /// Failed to convert RemoteEntity to GlobalEntity during waiting completion
    #[error("Error completing waiting EntityProperty! Could not convert RemoteEntity to GlobalEntity")]
    WaitingConversionFailed,

    /// Attempted to complete waiting on wrong entity property type
    #[error("Cannot complete EntityProperty of type '{property_type}'")]
    InvalidWaitingComplete {
        property_type: &'static str,
    },

    /// Invalid state transition (e.g., trying to publish already public property)
    #[error("EntityProperty of type '{property_type}' should never {operation}")]
    InvalidStateTransition {
        property_type: &'static str,
        operation: &'static str,
    },

    /// Attempted to enable delegation on invalid property type
    #[error("EntityProperty of type '{property_type}' should never enable delegation")]
    InvalidDelegationEnable {
        property_type: &'static str,
    },

    /// Attempted to disable delegation on invalid property type
    #[error("EntityProperty of type '{property_type}' should never disable delegation")]
    InvalidDelegationDisable {
        property_type: &'static str,
    },

    /// Attempted to localize invalid property type
    #[error("EntityProperty of type '{property_type}' should never be made local")]
    InvalidLocalization {
        property_type: &'static str,
    },

    /// Attempted operation requiring authority without having it
    #[error("Must have Authority over Entity before performing this operation")]
    InsufficientAuthority,

    /// Attempted to mutate a delegated property without requesting authority
    #[error("Must request authority to mutate a Delegated EntityProperty")]
    MutationNotAuthorized,

    /// Attempted to write_local_entity on unsupported property type
    #[error("EntityProperty type '{property_type}' cannot use write_local_entity method")]
    WriteLocalEntityNotSupported {
        property_type: &'static str,
    },

    /// Expected mutator to be set but it wasn't
    #[error("EntityProperty mutator should be initialized by now")]
    MutatorNotInitialized,
}
