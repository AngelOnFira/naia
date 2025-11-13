use naia_shared::EntityError;

// Note: LocalEntityMap is not publicly exported, so we can only test error types
// The internal implementation uses the try_* methods we created

#[test]
fn test_entity_neither_host_nor_remote_error_type() {
    let error = EntityError::EntityNeitherHostNorRemote {
        entity_id: "Entity(42)".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("neither host nor remote"));
    assert!(message.contains("Entity(42)"));
}

#[test]
fn test_entity_record_corruption_error_type() {
    let error = EntityError::EntityRecordCorruption {
        message: "test corruption".to_string(),
    };

    let message = error.to_string();
    assert!(message.contains("Entity record corruption"));
    assert!(message.contains("test corruption"));
}

#[test]
fn test_entity_mapping_inconsistency_error_type() {
    let error = EntityError::EntityMappingInconsistency {
        entity_id: "Entity(100)".to_string(),
        details: "missing remote",
    };

    let message = error.to_string();
    assert!(message.contains("mapping inconsistency"));
    assert!(message.contains("Entity(100)"));
    assert!(message.contains("missing remote"));
}

#[test]
fn test_entity_mapping_inconsistency_dual_entity_details() {
    let error = EntityError::EntityMappingInconsistency {
        entity_id: "<entity>".to_string(),
        details: "record does not have dual host and remote entity",
    };

    let message = error.to_string();
    assert!(message.contains("mapping inconsistency"));
    assert!(message.contains("record does not have dual host and remote entity"));
}

#[test]
fn test_error_variants_are_clonable() {
    let error1 = EntityError::EntityNeitherHostNorRemote {
        entity_id: "E1".to_string(),
    };
    let error2 = error1.clone();

    assert_eq!(error1.to_string(), error2.to_string());
}

#[test]
fn test_error_variants_are_sendable() {
    fn assert_send<T: Send>() {}
    assert_send::<EntityError>();
}

#[test]
fn test_entity_error_partial_eq() {
    let error1 = EntityError::EntityNeitherHostNorRemote {
        entity_id: "E1".to_string(),
    };
    let error2 = EntityError::EntityNeitherHostNorRemote {
        entity_id: "E1".to_string(),
    };
    let error3 = EntityError::EntityNeitherHostNorRemote {
        entity_id: "E2".to_string(),
    };

    assert_eq!(error1, error2);
    assert_ne!(error1, error3);
}

#[test]
fn test_entity_not_found_context() {
    let error = EntityError::EntityNotFound {
        context: "get_owned_entity lookup",
    };

    let message = error.to_string();
    assert!(message.contains("Entity not found"));
    assert!(message.contains("get_owned_entity lookup"));
}
