// Note: FragmentIndex is pub(crate), so we can't directly test it from outside the crate.
// This test file documents the fragmentation error handling that exists internally.
// The error is tested indirectly through the message manager's fragmentation logic.

use naia_shared::{FragmentationError, MessageError};

#[test]
fn test_fragmentation_error_display() {
    let error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };

    let error_str = format!("{}", error);
    assert!(error_str.contains("Fragment index limit"));
    assert!(error_str.contains("1048576"));
    assert!(error_str.contains("500 MB"));
}

#[test]
fn test_fragmentation_error_converts_to_message_error() {
    let frag_error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };

    let message_error: MessageError = frag_error.into();

    let error_str = format!("{}", message_error);
    assert!(error_str.contains("Fragmentation error"));
}

#[test]
fn test_fragmentation_error_properties() {
    let error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };

    // Test that error can be cloned
    let error_clone = error.clone();
    assert_eq!(error, error_clone);

    // Test that error can be compared
    let same_error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };
    assert_eq!(error, same_error);

    let different_error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 600,
    };
    assert_ne!(error, different_error);
}

#[test]
fn test_fragmentation_error_debug() {
    let error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };

    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("FragmentLimitExceeded"));
    assert!(debug_str.contains("limit"));
    assert!(debug_str.contains("estimated_mb"));
}

// Note: The actual FragmentIndex::try_increment() method is tested internally within the crate
// since FragmentIndex is not public. The method is used by the message fragmentation system
// to safely handle large messages without panicking.

#[test]
fn test_fragmentation_error_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<FragmentationError>();
    assert_sync::<FragmentationError>();
}

#[test]
fn test_message_error_with_fragmentation() {
    let frag_error = FragmentationError::FragmentLimitExceeded {
        limit: 1048576,
        estimated_mb: 500,
    };

    let message_error = MessageError::from(frag_error);

    // Test that we can pattern match on the error
    match message_error {
        MessageError::Fragmentation(FragmentationError::FragmentLimitExceeded { limit, estimated_mb }) => {
            assert_eq!(limit, 1048576);
            assert_eq!(estimated_mb, 500);
        }
        _ => panic!("Expected Fragmentation error variant"),
    }
}
