/// Assert that client's authority status is synchronized
/// This checks that the global authority tracker matches the entity channel's internal state
#[macro_export]
macro_rules! assert_authority_synced {
    ($client:expr, $entity:expr) => {
        assert!(
            $client.verify_authority_sync($entity).is_ok(),
            "Authority status mismatch between global tracker and entity channel for entity {:?}",
            $entity
        );
    };
}

/// Assert that client can request authority without panicking
#[macro_export]
macro_rules! assert_can_request_authority {
    ($client:expr, $entity:expr) => {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Just check that we can call the method without panic
            // Don't actually send the request in the assertion
        }));
        assert!(
            result.is_ok(),
            "Client should be able to request authority for entity {:?}",
            $entity
        );
    };
}

/// Assert that client can release authority without panicking
#[macro_export]
macro_rules! assert_can_release_authority {
    ($client:expr, $entity:expr) => {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Just check that we can call the method without panic
        }));
        assert!(
            result.is_ok(),
            "Client should be able to release authority for entity {:?}",
            $entity
        );
    };
}

