use naia_shared::ComponentError;

/// Tests for ComponentKinds error handling
///
/// Note: ComponentKinds methods (try_net_id_to_kind, try_kind_to_net_id, try_kind_to_builder)
/// are private implementation details. The public API (read, read_create_update, split_update,
/// kind_to_name) handles these errors internally.
///
/// These tests verify that the error types are properly defined and usable.

#[test]
fn test_component_not_registered_error() {
    let error = ComponentError::ComponentNotRegistered {
        component_name: "TestComponent",
    };

    assert_eq!(
        format!("{}", error),
        "Component not registered with Protocol. Must call `add_component()` during protocol initialization. Component: TestComponent"
    );
}

#[test]
fn test_net_id_not_found_error() {
    let error = ComponentError::NetIdNotFound { net_id: 999 };

    assert_eq!(
        format!("{}", error),
        "Component net ID 999 not found in registry. Must properly initialize Component with Protocol via `add_component()` function"
    );
}

#[test]
fn test_kind_not_found_error() {
    let error = ComponentError::KindNotFound;

    assert_eq!(
        format!("{}", error),
        "Component kind not found in registry. Must properly initialize Component with Protocol via `add_component()` function"
    );
}

#[test]
fn test_error_equality() {
    let error1 = ComponentError::NetIdNotFound { net_id: 42 };
    let error2 = ComponentError::NetIdNotFound { net_id: 42 };
    let error3 = ComponentError::NetIdNotFound { net_id: 99 };

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}

#[test]
fn test_error_clone() {
    let error1 = ComponentError::KindNotFound;
    let error2 = error1.clone();

    assert_eq!(error1, error2);
}

// Note: Full integration tests would require:
// - Setting up a Protocol with registered components
// - Attempting to read/write unregistered component types
// - Testing the internal panic -> try_* -> Result flow
//
// These tests focus on verifying error types are properly defined
// and can be constructed, which validates the error handling infrastructure.
