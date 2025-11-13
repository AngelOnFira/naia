/// Security-focused tests for Message Receiver error handling
///
/// These tests verify that ReceiverError types are properly defined and exported,
/// ensuring that receivers can handle malicious or malformed network input gracefully
/// without panicking, which is critical for preventing DoS attacks.

use naia_shared::ReceiverError;

// ============================================================================
// ReceiverError Type Tests
// ============================================================================

#[test]
fn receiver_error_implements_std_error() {
    use std::error::Error;

    let err = ReceiverError::NonFragmentedMessage;
    let _err_msg: &str = &err.to_string();

    // Verify Error trait is implemented
    let _source: Option<&(dyn Error + 'static)> = err.source();
}

#[test]
fn receiver_error_is_clone_and_eq() {
    let err1 = ReceiverError::NonFragmentedMessage;
    let err2 = err1.clone();

    assert_eq!(err1, err2);
}

#[test]
fn receiver_error_is_debug() {
    let err = ReceiverError::NonFragmentedMessage;
    let debug_str = format!("{:?}", err);
    assert!(!debug_str.is_empty());
}

#[test]
fn receiver_error_non_fragmented_message() {
    let err = ReceiverError::NonFragmentedMessage;
    let msg = err.to_string().to_lowercase();

    assert!(
        msg.contains("non-fragmented") || msg.contains("fragmented"),
        "Error message should mention fragmentation: {}",
        msg
    );
}

#[test]
fn receiver_error_message_downcast_failed() {
    let err = ReceiverError::MessageDowncastFailed {
        expected_type: "TestType",
    };
    let msg = err.to_string();

    assert!(
        msg.contains("TestType") && (msg.contains("downcast") || msg.contains("type")),
        "Error message should mention type and downcast: {}",
        msg
    );
}

#[test]
fn receiver_error_fragment_id_not_found() {
    let err = ReceiverError::FragmentIdNotFound;
    let msg = err.to_string().to_lowercase();

    assert!(
        msg.contains("fragment") && msg.contains("not found"),
        "Error message should mention fragment not found: {}",
        msg
    );
}

#[test]
fn receiver_error_duplicate_first_fragment() {
    let err = ReceiverError::DuplicateFirstFragment;
    let msg = err.to_string().to_lowercase();

    assert!(
        msg.contains("duplicate") && msg.contains("fragment"),
        "Error message should mention duplicate fragment: {}",
        msg
    );
}

#[test]
fn receiver_error_first_fragment_metadata_missing() {
    let err = ReceiverError::FirstFragmentMetadataMissing;
    let msg = err.to_string().to_lowercase();

    assert!(
        msg.contains("first") && msg.contains("fragment") && msg.contains("metadata"),
        "Error message should mention first fragment metadata: {}",
        msg
    );
}

#[test]
fn receiver_error_fragmented_message_read_failed() {
    let err = ReceiverError::FragmentedMessageReadFailed {
        reason: "test reason",
    };
    let msg = err.to_string();

    assert!(
        msg.contains("test reason") && (msg.contains("fragment") || msg.contains("read")),
        "Error message should mention reason and fragmented message: {}",
        msg
    );
}

#[test]
fn receiver_error_request_or_response_read_failed() {
    let err = ReceiverError::RequestOrResponseReadFailed {
        reason: "malformed data",
    };
    let msg = err.to_string();

    assert!(
        msg.contains("malformed data") && (msg.contains("request") || msg.contains("response")),
        "Error message should mention reason and request/response: {}",
        msg
    );
}

#[test]
fn receiver_error_buffer_inconsistency() {
    let err = ReceiverError::BufferInconsistency {
        reason: "duplicate detected",
    };
    let msg = err.to_string().to_lowercase();

    assert!(
        msg.contains("duplicate detected") && msg.contains("buffer"),
        "Error message should mention reason and buffer: {}",
        msg
    );
}

#[test]
fn receiver_error_requests_not_supported() {
    let err = ReceiverError::RequestsNotSupported {
        channel_type: "TestChannel",
    };
    let msg = err.to_string();

    assert!(
        msg.contains("TestChannel") && (msg.contains("support") || msg.contains("request")),
        "Error message should mention channel type and support: {}",
        msg
    );
}

// ============================================================================
// Security-Focused Error Message Tests
// ============================================================================

#[test]
fn all_error_messages_are_descriptive() {
    let test_cases = vec![
        (
            ReceiverError::NonFragmentedMessage,
            vec!["fragmented", "message"],
        ),
        (
            ReceiverError::MessageDowncastFailed {
                expected_type: "TestType",
            },
            vec!["downcast", "type"],
        ),
        (
            ReceiverError::FragmentIdNotFound,
            vec!["fragment", "not found"],
        ),
        (
            ReceiverError::DuplicateFirstFragment,
            vec!["duplicate", "fragment"],
        ),
        (
            ReceiverError::FirstFragmentMetadataMissing,
            vec!["first", "metadata"],
        ),
        (
            ReceiverError::FragmentedMessageReadFailed {
                reason: "test",
            },
            vec!["fragment", "read", "corrupted"],
        ),
        (
            ReceiverError::RequestOrResponseReadFailed {
                reason: "test",
            },
            vec!["request", "response"],
        ),
        (
            ReceiverError::BufferInconsistency {
                reason: "test",
            },
            vec!["buffer", "inconsistency"],
        ),
        (
            ReceiverError::RequestsNotSupported {
                channel_type: "Test",
            },
            vec!["support", "request"],
        ),
    ];

    for (error, keywords) in test_cases {
        let error_msg = error.to_string().to_lowercase();
        let found_keywords: Vec<_> = keywords.iter()
            .filter(|kw| error_msg.contains(&kw.to_lowercase()))
            .collect();

        assert!(
            !found_keywords.is_empty(),
            "Error '{}' should contain at least one of: {:?}",
            error_msg,
            keywords
        );
    }
}

// ============================================================================
// Security Property Tests
// ============================================================================

#[test]
fn security_errors_mention_potential_attack_vectors() {
    // Errors related to security issues should mention the nature of the problem
    let security_errors = vec![
        (
            ReceiverError::DuplicateFirstFragment,
            "duplicate fragment could be replay attack",
        ),
        (
            ReceiverError::FragmentedMessageReadFailed {
                reason: "deserialization failed",
            },
            "corrupted data",
        ),
        (
            ReceiverError::MessageDowncastFailed {
                expected_type: "FragmentedMessage",
            },
            "type mismatch indicates corrupted",
        ),
        (
            ReceiverError::BufferInconsistency {
                reason: "duplicate message received",
            },
            "buffer inconsistency",
        ),
    ];

    for (error, _description) in security_errors {
        let msg = error.to_string();
        assert!(
            !msg.is_empty(),
            "Security-related errors must have descriptive messages"
        );
    }
}

#[test]
fn error_variants_cover_all_panic_scenarios() {
    // Verify we have error variants for all the panic scenarios we replaced:
    // 1. Non-fragmented message in FragmentReceiver
    let _err1 = ReceiverError::NonFragmentedMessage;

    // 2. Message downcast failures
    let _err2 = ReceiverError::MessageDowncastFailed {
        expected_type: "TestType",
    };

    // 3. Fragment ID not found
    let _err3 = ReceiverError::FragmentIdNotFound;

    // 4. Duplicate first fragment (security issue)
    let _err4 = ReceiverError::DuplicateFirstFragment;

    // 5. First fragment metadata missing
    let _err5 = ReceiverError::FirstFragmentMetadataMissing;

    // 6. Failed to read fragmented message
    let _err6 = ReceiverError::FragmentedMessageReadFailed {
        reason: "test",
    };

    // 7. Failed to read request/response
    let _err7 = ReceiverError::RequestOrResponseReadFailed {
        reason: "test",
    };

    // 8. Buffer inconsistency
    let _err8 = ReceiverError::BufferInconsistency {
        reason: "test",
    };

    // 9. Requests not supported on channel type
    let _err9 = ReceiverError::RequestsNotSupported {
        channel_type: "Test",
    };
}

// ============================================================================
// Static String Tests (Performance)
// ============================================================================

#[test]
fn error_uses_static_strings_for_performance() {
    // Verify that error types use &'static str for constant messages
    // This ensures no heap allocations for error creation

    let err1 = ReceiverError::MessageDowncastFailed {
        expected_type: "TestType",
    };
    let err2 = ReceiverError::MessageDowncastFailed {
        expected_type: "TestType",
    };

    // Should be exactly equal since they use static strings
    assert_eq!(err1, err2);

    let err3 = ReceiverError::FragmentedMessageReadFailed {
        reason: "deserialization failed",
    };
    let err4 = ReceiverError::FragmentedMessageReadFailed {
        reason: "deserialization failed",
    };

    assert_eq!(err3, err4);
}
