/// Tests for Entity Action error handling
///
/// This test file verifies the EntityError type and its variants.
/// It ensures that error types are properly structured and can be
/// used by other agents (E2, E3, E4) for entity management operations.

use naia_shared::{EntityAuthError, EntityError};

#[test]
fn test_entity_not_found_error() {
    let error = EntityError::EntityNotFound {
        context: "entity lookup failed",
    };

    assert_eq!(
        error.to_string(),
        "Entity not found: entity lookup failed"
    );
}

#[test]
fn test_component_not_found_error() {
    let error = EntityError::ComponentNotFound {
        context: "component lookup failed",
    };

    assert_eq!(
        error.to_string(),
        "Component not found on entity: component lookup failed"
    );
}

#[test]
fn test_invalid_action_error() {
    let error = EntityError::InvalidAction {
        action: "spawn",
        reason: "entity already exists",
    };

    assert_eq!(
        error.to_string(),
        "Invalid entity action: spawn - entity already exists"
    );
}

#[test]
fn test_entity_already_exists_error() {
    let error = EntityError::EntityAlreadyExists {
        context: "tried to spawn duplicate entity",
    };

    assert_eq!(
        error.to_string(),
        "Entity already exists: tried to spawn duplicate entity"
    );
}

#[test]
fn test_component_already_exists_error() {
    let error = EntityError::ComponentAlreadyExists {
        context: "tried to insert duplicate component",
    };

    assert_eq!(
        error.to_string(),
        "Component already exists on entity: tried to insert duplicate component"
    );
}

#[test]
fn test_invalid_state_transition_error() {
    let error = EntityError::InvalidStateTransition {
        from_state: "Spawned",
        to_state: "Spawned",
        operation: "spawn",
    };

    assert_eq!(
        error.to_string(),
        "Invalid entity state transition: Spawned -> Spawned via spawn"
    );
}

#[test]
fn test_invalid_channel_operation_error() {
    let error = EntityError::InvalidChannelOperation {
        operation: "receive_action",
        reason: "entity not found in channel",
    };

    assert_eq!(
        error.to_string(),
        "Invalid entity channel operation: receive_action - entity not found in channel"
    );
}

#[test]
fn test_out_of_sequence_error() {
    let error = EntityError::OutOfSequence {
        expected: "action 5".to_string(),
        actual: "action 3".to_string(),
    };

    assert_eq!(
        error.to_string(),
        "Entity action out of sequence: expected action 5, got action 3"
    );
}

#[test]
fn test_internal_consistency_error() {
    let error = EntityError::InternalConsistency {
        context: "unexpected state in entity channel",
    };

    assert_eq!(
        error.to_string(),
        "Internal entity consistency error: unexpected state in entity channel"
    );
}

#[test]
fn test_entity_not_found_with_conversion_error() {
    let error = EntityError::EntityNotFound {
        context: "global entity conversion",
    };

    assert_eq!(
        error.to_string(),
        "Entity not found: global entity conversion"
    );
}

#[test]
fn test_conversion_failed_error() {
    let error = EntityError::ConversionFailed {
        from: "LocalEntity",
        to: "GlobalEntity",
    };

    assert_eq!(
        error.to_string(),
        "Entity conversion failed: cannot convert LocalEntity to GlobalEntity"
    );
}

#[test]
fn test_invalid_entity_type_error() {
    let error = EntityError::InvalidEntityType {
        expected: "RemoteEntity",
        actual: "HostEntity",
    };

    assert_eq!(
        error.to_string(),
        "Invalid entity type: expected RemoteEntity, got HostEntity"
    );
}

#[test]
fn test_serialization_not_supported_error() {
    let error = EntityError::SerializationNotSupported {
        entity_type: "LocalEntity",
        operation: "serialize",
    };

    assert_eq!(
        error.to_string(),
        "Serialization operation 'serialize' not supported for LocalEntity"
    );
}

#[test]
fn test_entity_auth_error_already_registered() {
    let error = EntityAuthError::EntityAlreadyRegistered {
        entity_id: "entity_123".to_string(),
    };

    assert_eq!(
        error.to_string(),
        "Entity entity_123 is already registered with the auth handler"
    );
}

#[test]
fn test_entity_auth_error_not_registered() {
    let error = EntityAuthError::EntityNotRegistered {
        entity_id: "entity_456".to_string(),
        operation: "request_authority",
    };

    assert_eq!(
        error.to_string(),
        "Entity entity_456 is not registered - operation 'request_authority' requires registration"
    );
}

#[test]
fn test_entity_auth_error_lock_poisoned() {
    let error = EntityAuthError::AuthLockPoisoned;

    assert_eq!(
        error.to_string(),
        "Auth status lock is poisoned - this indicates a panic occurred while holding the lock"
    );
}

#[test]
fn test_entity_auth_error_invalid_state_transition() {
    let error = EntityAuthError::InvalidAuthStateTransition {
        host_type: "Server",
        from_state: "Available",
        to_state: "Granted",
        operation: "release",
    };

    assert_eq!(
        error.to_string(),
        "Invalid authority state transition for Server: Available -> Granted via release"
    );
}

#[test]
fn test_entity_auth_error_operation_not_permitted() {
    let error = EntityAuthError::OperationNotPermitted {
        operation: "mutate",
        host_type: "Client",
        current_state: "Requested",
    };

    assert_eq!(
        error.to_string(),
        "mutate not permitted for Client in state Requested"
    );
}

#[test]
fn test_entity_error_is_clone() {
    let error = EntityError::EntityNotFound {
        context: "test clone",
    };
    let cloned = error.clone();

    assert_eq!(error, cloned);
}

#[test]
fn test_entity_error_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<EntityError>();
    assert_sync::<EntityError>();
}

#[test]
fn test_entity_error_display_formatting() {
    let errors = vec![
        EntityError::EntityNotFound {
            context: "lookup",
        },
        EntityError::ComponentNotFound {
            context: "component search",
        },
        EntityError::InvalidAction {
            action: "spawn",
            reason: "already exists",
        },
    ];

    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty());
        // Ensure error messages are descriptive (at least 20 chars)
        assert!(display.len() > 20, "Error message too short: {}", display);
    }
}

#[test]
fn test_entity_error_debug_formatting() {
    let error = EntityError::InvalidAction {
        action: "despawn",
        reason: "entity not found",
    };

    let debug = format!("{:?}", error);
    assert!(debug.contains("InvalidAction"));
    assert!(debug.contains("despawn"));
    assert!(debug.contains("entity not found"));
}

#[test]
fn test_entity_error_equality() {
    let error1 = EntityError::EntityNotFound {
        context: "test",
    };
    let error2 = EntityError::EntityNotFound {
        context: "test",
    };
    let error3 = EntityError::ComponentNotFound {
        context: "test",
    };

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}
