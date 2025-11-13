use naia_shared::{
    EntityAuthError, EntityAuthStatus, HostAuthHandler, HostEntityAuthStatus, HostType,
};

// Test helper to create a mock entity type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct TestEntity(u32);

#[cfg(test)]
mod host_auth_handler_tests {
    use super::*;

    #[test]
    fn test_try_register_entity_succeeds() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        let result = handler.try_register_entity(HostType::Server, &entity);

        assert!(result.is_ok(), "Expected successful registration");

        // Verify entity is registered by checking auth_status through handler
        let auth_status = handler.auth_status(&entity);
        assert!(auth_status.is_some());
        assert_eq!(auth_status.unwrap().status(), EntityAuthStatus::Available);
    }

    #[test]
    fn test_try_register_entity_twice_fails() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        // First registration should succeed
        handler.try_register_entity(HostType::Server, &entity).unwrap();

        // Second registration should fail
        let result = handler.try_register_entity(HostType::Server, &entity);

        assert!(result.is_err(), "Expected duplicate registration to fail");
        if let Err(EntityAuthError::EntityAlreadyRegistered { entity_id }) = result {
            assert!(entity_id.contains("TestEntity"));
        } else {
            panic!("Expected EntityAlreadyRegistered error");
        }
    }

    #[test]
    fn test_register_entity_twice_panics() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        handler.register_entity(HostType::Server, &entity);

        // This should panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler.register_entity(HostType::Server, &entity);
        }));

        assert!(result.is_err(), "Expected panic on duplicate registration");
    }

    #[test]
    fn test_try_get_accessor_for_unregistered_entity_fails() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(42);

        let result = handler.try_get_accessor(&entity);

        assert!(result.is_err(), "Expected failure for unregistered entity");
        if let Err(EntityAuthError::EntityNotRegistered { entity_id, operation }) = result {
            assert!(entity_id.contains("42"));
            assert_eq!(operation, "get_accessor");
        } else {
            panic!("Expected EntityNotRegistered error");
        }
    }

    #[test]
    fn test_try_get_accessor_for_registered_entity_succeeds() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        handler.try_register_entity(HostType::Client, &entity).unwrap();

        let result = handler.try_get_accessor(&entity);

        assert!(result.is_ok(), "Expected success for registered entity");
    }

    #[test]
    fn test_get_accessor_for_unregistered_entity_panics() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(99);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler.get_accessor(&entity);
        }));

        assert!(result.is_err(), "Expected panic for unregistered entity");
    }

    #[test]
    fn test_try_set_auth_status_for_unregistered_entity_fails() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(123);

        let result = handler.try_set_auth_status(&entity, EntityAuthStatus::Granted);

        assert!(result.is_err(), "Expected failure for unregistered entity");
        if let Err(EntityAuthError::EntityNotRegistered { entity_id, operation }) = result {
            assert!(entity_id.contains("123"));
            assert_eq!(operation, "set_auth_status");
        } else {
            panic!("Expected EntityNotRegistered error");
        }
    }

    #[test]
    fn test_try_set_auth_status_for_registered_entity_succeeds() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        handler.try_register_entity(HostType::Client, &entity).unwrap();

        let result = handler.try_set_auth_status(&entity, EntityAuthStatus::Granted);

        assert!(result.is_ok(), "Expected success for registered entity");

        // Verify the status was set
        let auth_status = handler.auth_status(&entity);
        assert!(auth_status.is_some());
        assert_eq!(auth_status.unwrap().status(), EntityAuthStatus::Granted);
    }

    #[test]
    fn test_set_auth_status_for_unregistered_entity_panics() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(456);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler.set_auth_status(&entity, EntityAuthStatus::Denied);
        }));

        assert!(result.is_err(), "Expected panic for unregistered entity");
    }

    #[test]
    fn test_deregister_entity_removes_from_handler() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        handler.try_register_entity(HostType::Server, &entity).unwrap();
        assert!(handler.auth_status(&entity).is_some());

        handler.deregister_entity(&entity);

        // After deregistration, entity should not be found
        assert!(handler.auth_status(&entity).is_none());
        assert!(handler.try_get_accessor(&entity).is_err());
    }

    #[test]
    fn test_auth_status_returns_none_for_unregistered_entity() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(789);

        let status = handler.auth_status(&entity);

        assert!(status.is_none(), "Expected None for unregistered entity");
    }
}

#[cfg(test)]
mod entity_auth_status_tests {
    use super::*;

    // Client authority state machine tests
    #[test]
    fn test_client_can_request_when_available() {
        let status = HostEntityAuthStatus::new(HostType::Client, EntityAuthStatus::Available);
        assert!(status.can_request(), "Client should be able to request when available");
        assert!(!status.can_release(), "Client should not be able to release when available");
        assert!(!status.can_write(), "Client should not be able to write when available");
        assert!(status.can_read(), "Client should be able to read when available");
    }

    #[test]
    fn test_client_cannot_request_when_already_requested() {
        let status = HostEntityAuthStatus::new(HostType::Client, EntityAuthStatus::Requested);
        assert!(!status.can_request(), "Client should not request when already requested");
        assert!(status.can_release(), "Client should be able to release when requested");
        assert!(!status.can_write(), "Client should not be able to write when only requested");
        assert!(status.can_mutate(), "Client should be able to mutate when requested");
    }

    #[test]
    fn test_client_granted_authority() {
        let status = HostEntityAuthStatus::new(HostType::Client, EntityAuthStatus::Granted);
        assert!(!status.can_request(), "Client should not request when already granted");
        assert!(status.can_release(), "Client should be able to release when granted");
        assert!(status.can_write(), "Client should be able to write when granted");
        assert!(status.can_mutate(), "Client should be able to mutate when granted");
        assert!(!status.can_read(), "Client should not read (server replicates) when granted");
    }

    #[test]
    fn test_client_releasing_authority() {
        let status = HostEntityAuthStatus::new(HostType::Client, EntityAuthStatus::Releasing);
        assert!(!status.can_request(), "Client should not request when releasing");
        assert!(!status.can_release(), "Client should not release when already releasing");
        assert!(status.can_write(), "Client should still be able to write when releasing");
        assert!(status.can_read(), "Client should be able to read when releasing");
    }

    #[test]
    fn test_client_denied_authority() {
        let status = HostEntityAuthStatus::new(HostType::Client, EntityAuthStatus::Denied);
        assert!(!status.can_request(), "Client should not request when denied");
        assert!(!status.can_release(), "Client should not release when denied");
        assert!(!status.can_write(), "Client should not be able to write when denied");
        assert!(status.can_read(), "Client should be able to read when denied");
    }

    // Server authority tests - server always has implicit authority
    #[test]
    fn test_server_does_not_need_to_request_authority() {
        let statuses = [
            EntityAuthStatus::Available,
            EntityAuthStatus::Requested,
            EntityAuthStatus::Granted,
            EntityAuthStatus::Releasing,
            EntityAuthStatus::Denied,
        ];

        for auth_status in &statuses {
            let status = HostEntityAuthStatus::new(HostType::Server, *auth_status);
            assert!(!status.can_request(),
                "Server should not need to request authority in state {:?}", auth_status);
        }
    }

    #[test]
    fn test_server_does_not_release_authority() {
        let statuses = [
            EntityAuthStatus::Available,
            EntityAuthStatus::Requested,
            EntityAuthStatus::Granted,
            EntityAuthStatus::Releasing,
            EntityAuthStatus::Denied,
        ];

        for auth_status in &statuses {
            let status = HostEntityAuthStatus::new(HostType::Server, *auth_status);
            assert!(!status.can_release(),
                "Server should not release authority in state {:?}", auth_status);
        }
    }

    #[test]
    fn test_server_can_always_mutate_read_and_write() {
        let statuses = [
            EntityAuthStatus::Available,
            EntityAuthStatus::Requested,
            EntityAuthStatus::Granted,
            EntityAuthStatus::Releasing,
            EntityAuthStatus::Denied,
        ];

        for auth_status in &statuses {
            let status = HostEntityAuthStatus::new(HostType::Server, *auth_status);
            assert!(status.can_mutate(),
                "Server should always be able to mutate in state {:?}", auth_status);
            assert!(status.can_read(),
                "Server should always be able to read in state {:?}", auth_status);
            assert!(status.can_write(),
                "Server should always be able to write in state {:?}", auth_status);
        }
    }

    #[test]
    fn test_status_accessor() {
        let auth_status = EntityAuthStatus::Granted;
        let status = HostEntityAuthStatus::new(HostType::Client, auth_status);
        assert_eq!(status.status(), auth_status, "Status accessor should return correct status");
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_malicious_client_cannot_crash_server_with_duplicate_registration() {
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        // First registration succeeds
        handler.try_register_entity(HostType::Client, &entity).unwrap();

        // Malicious client tries to register again - should fail gracefully
        for _ in 0..100 {
            let result = handler.try_register_entity(HostType::Client, &entity);
            assert!(result.is_err(), "Duplicate registration should always fail");
        }

        // Handler should still be in valid state
        assert!(handler.auth_status(&entity).is_some());
    }

    #[test]
    fn test_malicious_client_cannot_crash_server_with_invalid_accessor_requests() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let unregistered_entities = vec![
            TestEntity(1),
            TestEntity(999),
            TestEntity(0xDEADBEEF),
            TestEntity(u32::MAX),
        ];

        // Malicious client tries to access non-existent entities - should fail gracefully
        for entity in &unregistered_entities {
            for _ in 0..10 {
                let result = handler.try_get_accessor(entity);
                assert!(result.is_err(), "Accessing unregistered entity should fail");
            }
        }
    }

    #[test]
    fn test_malicious_client_cannot_crash_server_with_invalid_status_updates() {
        let handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let unregistered_entity = TestEntity(0xBADBAD);

        // Malicious client tries to set status on non-existent entity - should fail gracefully
        let invalid_statuses = vec![
            EntityAuthStatus::Available,
            EntityAuthStatus::Requested,
            EntityAuthStatus::Granted,
            EntityAuthStatus::Releasing,
            EntityAuthStatus::Denied,
        ];

        for status in &invalid_statuses {
            for _ in 0..10 {
                let result = handler.try_set_auth_status(&unregistered_entity, *status);
                assert!(result.is_err(), "Setting status on unregistered entity should fail");
            }
        }
    }

    #[test]
    fn test_concurrent_registration_attempts_handled_safely() {
        // This tests that multiple registration attempts are handled in a predictable way
        let mut handler: HostAuthHandler<TestEntity> = HostAuthHandler::new();
        let entity = TestEntity(1);

        let results: Vec<_> = (0..10)
            .map(|_| handler.try_register_entity(HostType::Server, &entity))
            .collect();

        // Exactly one should succeed, rest should fail
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let error_count = results.iter().filter(|r| r.is_err()).count();

        assert_eq!(success_count, 1, "Exactly one registration should succeed");
        assert_eq!(error_count, 9, "All other registrations should fail");
    }
}
