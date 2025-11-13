use naia_shared::{
    EntityError, EntityDoesNotExistError,
};

#[test]
fn test_entity_error_from_does_not_exist() {
    let legacy_error = EntityDoesNotExistError;
    let entity_error: EntityError = legacy_error.into();

    // Should convert to EntityNotFound variant
    match entity_error {
        EntityError::EntityNotFound { context } => {
            assert_eq!(context, "entity lookup failed during conversion");
        }
        _ => panic!("Expected EntityNotFound variant"),
    }
}

#[test]
fn test_entity_error_display_messages() {
    // Test that error messages are clear and helpful
    let error1 = EntityError::EntityNotFound {
        context: "global entity lookup",
    };
    assert_eq!(
        error1.to_string(),
        "Entity not found: global entity lookup"
    );

    let error2 = EntityError::ConversionFailed {
        from: "GlobalEntity",
        to: "LocalEntity",
    };
    assert_eq!(
        error2.to_string(),
        "Entity conversion failed: cannot convert GlobalEntity to LocalEntity"
    );

    let error3 = EntityError::InvalidEntityType {
        expected: "RemoteEntity",
        actual: "HostEntity",
    };
    assert_eq!(
        error3.to_string(),
        "Invalid entity type: expected RemoteEntity, got HostEntity"
    );

    let error4 = EntityError::SerializationNotSupported {
        entity_type: "GlobalEntity",
        operation: "serialize",
    };
    assert_eq!(
        error4.to_string(),
        "Serialization operation 'serialize' not supported for GlobalEntity"
    );
}

#[test]
fn test_entity_error_clone() {
    let error = EntityError::InvalidEntityType {
        expected: "RemoteEntity",
        actual: "HostEntity",
    };

    let cloned = error.clone();
    assert_eq!(error, cloned);
}

#[test]
fn test_entity_error_debug() {
    let error = EntityError::EntityNotFound {
        context: "test context",
    };

    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("EntityNotFound"));
    assert!(debug_str.contains("test context"));
}

#[test]
fn test_entity_error_is_send_sync() {
    // Verify EntityError can be sent across threads (important for async/server code)
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<EntityError>();
    assert_sync::<EntityError>();
}

#[test]
fn test_entity_error_is_std_error() {
    let error = EntityError::EntityNotFound {
        context: "test",
    };

    // Verify it implements std::error::Error
    let _: &dyn std::error::Error = &error;
}

#[test]
fn test_entity_error_equality() {
    let error1 = EntityError::EntityNotFound {
        context: "same context",
    };
    let error2 = EntityError::EntityNotFound {
        context: "same context",
    };
    let error3 = EntityError::EntityNotFound {
        context: "different context",
    };

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}
