use naia_shared::WorldChannelError;

// Note: GlobalDiffHandler is not widely used in public API tests due to component complexity
// The internal implementation uses the try_register_component method we created

#[test]
fn test_component_already_registered_error_type() {
    let error = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "Entity(42)".to_string(),
        component_kind: "Position".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("already registered"));
    assert!(message.contains("Entity(42)"));
    assert!(message.contains("Position"));
}

#[test]
fn test_component_already_registered_error_different_components() {
    let error1 = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "Entity(100)".to_string(),
        component_kind: "Position".to_string(),
    };

    let error2 = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "Entity(100)".to_string(),
        component_kind: "Velocity".to_string(),
    };

    // Same entity, different components - errors should be different
    assert_ne!(error1.to_string(), error2.to_string());
}

#[test]
fn test_component_already_registered_error_different_entities() {
    let error1 = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "Entity(100)".to_string(),
        component_kind: "Position".to_string(),
    };

    let error2 = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "Entity(200)".to_string(),
        component_kind: "Position".to_string(),
    };

    // Different entities, same component - errors should be different
    assert_ne!(error1.to_string(), error2.to_string());
}

#[test]
fn test_error_variants_are_clonable() {
    let error1 = WorldChannelError::ComponentAlreadyRegistered {
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

#[test]
fn test_error_display_format() {
    let error = WorldChannelError::ComponentAlreadyRegistered {
        entity_id: "<entity>".to_string(),
        component_kind: "TestComponent".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("Component"));
    assert!(message.contains("TestComponent"));
    assert!(message.contains("<entity>"));
}
