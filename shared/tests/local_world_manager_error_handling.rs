/// Tests for LocalWorldManager panic-free error handling
///
/// This test file demonstrates the new try_* methods that provide
/// graceful error handling instead of panicking. These tests ensure
/// that malicious clients cannot crash the server through entity
/// management operations.

use naia_shared::{
    EntityError,
    LocalWorldManager,
    RemoteEntity,
};

// Use u32 as a simple world entity type for tests
type WorldEntity = u32;

#[test]
fn test_try_host_reserve_entity_twice_fails() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;

    // First reservation should succeed
    let result1 = manager.try_host_reserve_entity(&world_entity);
    assert!(result1.is_ok());

    // Second reservation should fail
    let result2 = manager.try_host_reserve_entity(&world_entity);
    assert!(result2.is_err());

    match result2.unwrap_err() {
        EntityError::EntityAlreadyReserved { .. } => {
            // Expected error variant
        }
        _ => panic!("Expected EntityAlreadyReserved error"),
    }
}

#[test]
fn test_try_insert_host_entity_collision_fails() {
    let mut manager1 = LocalWorldManager::<WorldEntity>::new(123);
    let mut manager2 = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity1 = 1u32;
    let world_entity2 = 2u32;

    // Reserve a host entity in first manager
    let host_entity = manager1.try_host_reserve_entity(&world_entity1).unwrap();

    // Create the same host entity ID in second manager by reserving
    let _host_entity2 = manager2.try_host_reserve_entity(&world_entity2).unwrap();

    // Now try to insert host_entity into manager2 where it already exists
    let result2 = manager2.try_insert_host_entity(world_entity2, host_entity);
    assert!(result2.is_err());

    match result2.unwrap_err() {
        EntityError::EntityAlreadyRegisteredAs { existing_type, .. } => {
            assert_eq!(existing_type, "HostEntity");
        }
        _ => panic!("Expected EntityAlreadyRegisteredAs error"),
    }
}

#[test]
fn test_try_insert_remote_entity_collision_fails() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity1 = 1u32;
    let world_entity2 = 2u32;
    let remote_entity = RemoteEntity::new(456);

    // First insertion should succeed
    let result1 = manager.try_insert_remote_entity(&world_entity1, remote_entity);
    assert!(result1.is_ok());

    // Second insertion with same remote entity should fail
    let result2 = manager.try_insert_remote_entity(&world_entity2, remote_entity);
    assert!(result2.is_err());

    match result2.unwrap_err() {
        EntityError::EntityAlreadyRegisteredAs { entity_id, existing_type } => {
            assert_eq!(existing_type, "RemoteEntity");
            assert!(entity_id.contains("RemoteEntity"));
        }
        _ => panic!("Expected EntityAlreadyRegisteredAs error"),
    }
}

#[test]
fn test_try_remove_by_world_entity_missing_entity_fails() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 999u32;

    // Try to remove an entity that doesn't exist
    let result = manager.try_remove_by_world_entity(&world_entity);
    assert!(result.is_err());

    match result.unwrap_err() {
        EntityError::EntityNotFound { context } => {
            assert!(context.contains("despawn"));
        }
        _ => panic!("Expected EntityNotFound error"),
    }
}

#[test]
fn test_try_remove_by_world_entity_succeeds() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;

    // Reserve entity (which also inserts into entity_map)
    let _host_entity = manager.try_host_reserve_entity(&world_entity).unwrap();

    // Removal should succeed
    let result = manager.try_remove_by_world_entity(&world_entity);
    assert!(result.is_ok());
}

#[test]
fn test_try_remove_by_remote_entity_missing_entity_fails() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let remote_entity = RemoteEntity::new(999);

    // Try to remove a remote entity that doesn't exist
    let result = manager.try_remove_by_remote_entity(&remote_entity);
    assert!(result.is_err());

    match result.unwrap_err() {
        EntityError::EntityNotFound { context } => {
            assert!(context.contains("remote entity"));
        }
        _ => panic!("Expected EntityNotFound error"),
    }
}

#[test]
fn test_try_remove_by_remote_entity_succeeds() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;
    let remote_entity = RemoteEntity::new(456);

    // Insert remote entity properly
    manager.try_insert_remote_entity(&world_entity, remote_entity).unwrap();

    // Removal should succeed and return the world entity
    let result = manager.try_remove_by_remote_entity(&remote_entity);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), world_entity);
}

#[test]
fn test_try_world_entity_from_remote_missing_fails() {
    let manager = LocalWorldManager::<WorldEntity>::new(123);
    let remote_entity = RemoteEntity::new(999);

    // Try to lookup a remote entity that doesn't exist
    let result = manager.try_world_entity_from_remote(&remote_entity);
    assert!(result.is_err());

    match result.unwrap_err() {
        EntityError::EntityNotFound { context } => {
            assert!(context.contains("remote entity"));
        }
        _ => panic!("Expected EntityNotFound error"),
    }
}

#[test]
fn test_try_world_entity_from_remote_succeeds() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;
    let remote_entity = RemoteEntity::new(456);

    // Insert remote entity
    manager.try_insert_remote_entity(&world_entity, remote_entity).unwrap();

    // Lookup should succeed
    let result = manager.try_world_entity_from_remote(&remote_entity);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), world_entity);
}

// Security test: Malicious client spam reserves
#[test]
fn test_malicious_client_spam_reserves() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;

    // First reservation succeeds
    assert!(manager.try_host_reserve_entity(&world_entity).is_ok());

    // Attempt 100 duplicate reservations - all should fail gracefully
    for _ in 0..100 {
        let result = manager.try_host_reserve_entity(&world_entity);
        assert!(result.is_err());
        match result.unwrap_err() {
            EntityError::EntityAlreadyReserved { .. } => {
                // Expected - no panic, no crash
            }
            _ => panic!("Expected EntityAlreadyReserved error"),
        }
    }
}

// Security test: Malicious client spam duplicate insertions
#[test]
fn test_malicious_client_spam_insertions() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let remote_entity = RemoteEntity::new(456);

    // First insertion succeeds
    assert!(manager.try_insert_remote_entity(&1u32, remote_entity).is_ok());

    // Attempt 100 duplicate insertions - all should fail gracefully
    for i in 2..102 {
        let result = manager.try_insert_remote_entity(&i, remote_entity);
        assert!(result.is_err());
        match result.unwrap_err() {
            EntityError::EntityAlreadyRegisteredAs { .. } => {
                // Expected - no panic, no crash
            }
            _ => panic!("Expected EntityAlreadyRegisteredAs error"),
        }
    }
}

// Security test: Malicious client spam despawn non-existent entities
#[test]
fn test_malicious_client_spam_despawn() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);

    // Attempt 100 despawns of non-existent entities - all should fail gracefully
    for i in 0..100 {
        let result = manager.try_remove_by_world_entity(&i);
        assert!(result.is_err());
        match result.unwrap_err() {
            EntityError::EntityNotFound { .. } => {
                // Expected - no panic, no crash
            }
            _ => panic!("Expected EntityNotFound error"),
        }
    }
}

// Security test: Malicious client spam invalid lookups
#[test]
fn test_malicious_client_spam_lookups() {
    let manager = LocalWorldManager::<WorldEntity>::new(123);

    // Attempt 100 lookups of non-existent entities - all should fail gracefully
    for i in 0..100 {
        let remote_entity = RemoteEntity::new(i);
        let result = manager.try_world_entity_from_remote(&remote_entity);
        assert!(result.is_err());
        match result.unwrap_err() {
            EntityError::EntityNotFound { .. } => {
                // Expected - no panic, no crash
            }
            _ => panic!("Expected EntityNotFound error"),
        }
    }
}

#[test]
fn test_successful_entity_lifecycle() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 42u32;

    // Reserve entity (this also inserts into entity_map)
    let _host_entity = manager.try_host_reserve_entity(&world_entity).unwrap();
    assert!(manager.has_world_entity(&world_entity));

    // Add remote entity
    let remote_entity = RemoteEntity::new(999);
    manager.try_insert_remote_entity(&world_entity, remote_entity).unwrap();
    assert!(manager.has_remote_entity(&remote_entity));
    assert!(manager.has_both_host_and_remote_entity(&world_entity));

    // Lookup works
    let looked_up = manager.try_world_entity_from_remote(&remote_entity).unwrap();
    assert_eq!(looked_up, world_entity);

    // Remove by remote entity
    let removed_entity = manager.try_remove_by_remote_entity(&remote_entity).unwrap();
    assert_eq!(removed_entity, world_entity);
    assert!(!manager.has_world_entity(&world_entity));
}

#[test]
fn test_error_messages_are_clear() {
    // Test that error messages contain helpful context
    let error1 = EntityError::EntityAlreadyReserved {
        entity_id: "42".to_string(),
    };
    assert!(error1.to_string().contains("already reserved"));
    assert!(error1.to_string().contains("42"));

    let error2 = EntityError::EntityAlreadyRegisteredAs {
        entity_id: "123".to_string(),
        existing_type: "HostEntity",
    };
    assert!(error2.to_string().contains("already registered"));
    assert!(error2.to_string().contains("123"));
    assert!(error2.to_string().contains("HostEntity"));

    let error3 = EntityError::MissingHostEntity {
        entity_id: "999".to_string(),
    };
    assert!(error3.to_string().contains("missing"));
    assert!(error3.to_string().contains("host"));
    assert!(error3.to_string().contains("999"));

    let error4 = EntityError::ReservationExpired {
        entity_id: "456".to_string(),
    };
    assert!(error4.to_string().contains("reservation"));
    assert!(error4.to_string().contains("456"));

    let error5 = EntityError::ReservationQueueCorrupted;
    assert!(error5.to_string().contains("corrupted"));
}

#[test]
fn test_backward_compatibility_panics_preserved() {
    let mut manager = LocalWorldManager::<WorldEntity>::new(123);
    let world_entity = 1u32;

    // First call succeeds
    manager.host_reserve_entity(&world_entity);

    // Second call should panic (backward compatible behavior)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        manager.host_reserve_entity(&world_entity);
    }));

    assert!(result.is_err());
}

#[test]
fn test_error_is_send_sync() {
    // Verify EntityError can be sent across threads (important for async/server code)
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<EntityError>();
    assert_sync::<EntityError>();
}
