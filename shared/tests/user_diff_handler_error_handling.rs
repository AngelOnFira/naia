use naia_shared::WorldChannelError;

// Note: UserDiffHandler is not publicly exported, so we can only test error types
// The internal implementation uses the try_* methods we created

#[test]
fn test_receiver_not_found_error_type() {
    let error = WorldChannelError::ReceiverNotFound {
        entity_id: "Entity(123)".to_string(),
        component_kind: "Position".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("Receiver not found"));
    assert!(message.contains("Entity(123)"));
    assert!(message.contains("Position"));
}

#[test]
fn test_component_not_registered_error_type() {
    let error = WorldChannelError::ComponentNotRegistered {
        entity_id: "Entity(999)".to_string(),
        component_kind: "Velocity".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("not registered"));
    assert!(message.contains("Entity(999)"));
    assert!(message.contains("Velocity"));
}

#[test]
fn test_rwlock_reentrant_error_type() {
    let error = WorldChannelError::RwLockReentrant;

    let message = error.to_string();
    assert_eq!(message, "RwLock is already held on current thread");
}

#[test]
fn test_error_variants_are_clonable() {
    let error1 = WorldChannelError::ReceiverNotFound {
        entity_id: "E1".to_string(),
        component_kind: "C1".to_string(),
    };
    let error2 = error1.clone();

    assert_eq!(error1.to_string(), error2.to_string());
}

#[test]
fn test_error_variants_are_sendable() {
    fn assert_send<T: Send>() {}
    assert_send::<WorldChannelError>();
}
