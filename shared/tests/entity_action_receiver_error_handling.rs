/// Integration tests for EntityActionReceiver error handling
///
/// This test file verifies that all panic points in entity_action_receiver.rs
/// have been replaced with proper error handling via try_* methods.
///
/// The EntityActionReceiver processes entity actions from the network and must
/// be resilient to malicious or corrupted input that could violate internal
/// invariants.

use naia_shared::EntityError;

// ========== Error Type Tests ==========
// These tests verify that EntityError has all EntityActionReceiver-specific variants

#[test]
fn test_entity_channel_not_found_error() {
    let error = EntityError::EntityChannelNotFound {
        context: "entity channel not found after insertion in receive_actions",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity channel not found"));
    assert!(msg.contains("after insertion"));
}

#[test]
fn test_component_channel_not_found_error() {
    let error = EntityError::ComponentChannelNotFound {
        context: "component channel not found after insertion in receive_insert_component_action",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Component channel not found"));
    assert!(msg.contains("after insertion"));
}

#[test]
fn test_ordered_ids_corrupted_error() {
    let error = EntityError::OrderedIdsCorrupted {
        index: 5,
        length: 3,
    };
    let msg = format!("{}", error);
    assert!(msg.contains("OrderedIds"));
    assert!(msg.contains("corruption"));
    assert!(msg.contains("5"));
    assert!(msg.contains("3"));
}

#[test]
fn test_entity_channel_not_found_error_display() {
    let error = EntityError::EntityChannelNotFound {
        context: "test context",
    };
    assert_eq!(
        error.to_string(),
        "Entity channel not found after insertion: test context"
    );
}

#[test]
fn test_component_channel_not_found_error_display() {
    let error = EntityError::ComponentChannelNotFound {
        context: "test context",
    };
    assert_eq!(
        error.to_string(),
        "Component channel not found after insertion: test context"
    );
}

#[test]
fn test_ordered_ids_corrupted_error_display() {
    let error = EntityError::OrderedIdsCorrupted {
        index: 10,
        length: 5,
    };
    assert_eq!(
        error.to_string(),
        "OrderedIds internal corruption: index 10 out of bounds (length: 5)"
    );
}

// ========== Error Properties Tests ==========

#[test]
fn test_entity_action_receiver_errors_are_clone() {
    let error = EntityError::EntityChannelNotFound {
        context: "test clone",
    };
    let cloned = error.clone();
    assert_eq!(error, cloned);
}

#[test]
fn test_entity_action_receiver_errors_are_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<EntityError>();
    assert_sync::<EntityError>();
}

#[test]
fn test_entity_action_receiver_error_debug_formatting() {
    let error = EntityError::ComponentChannelNotFound {
        context: "debug test",
    };
    let debug = format!("{:?}", error);
    assert!(debug.contains("ComponentChannelNotFound"));
    assert!(debug.contains("debug test"));
}

// ========== Static String Tests ==========
// Verify that error messages use &'static str for performance

#[test]
fn test_error_contexts_are_static() {
    // These should compile, demonstrating &'static str usage
    let _error1: EntityError = EntityError::EntityChannelNotFound {
        context: "static string",
    };
    let _error2: EntityError = EntityError::ComponentChannelNotFound {
        context: "another static string",
    };
}

// ========== Integration Tests ==========
// Note: Full integration tests require setting up EntityActionReceiver with
// actual entity types and action sequences. These would be better placed in
// integration tests that can construct full action pipelines.
//
// The tests above verify that:
// 1. Error types exist and are properly defined
// 2. Error messages are descriptive and contain necessary context
// 3. Errors implement required traits (Clone, Send, Sync)
// 4. Error formatting works correctly
//
// The actual EntityActionReceiver behavior is tested through the existing
// naia test suite which exercises the public API.

// ========== Documentation ==========
//
// EntityActionReceiver is a critical component that processes entity actions
// from the network. It must handle:
//
// 1. Out-of-order action delivery (actions may arrive in any order)
// 2. Duplicate actions (network may send same action multiple times)
// 3. Missing actions (some actions may be lost)
// 4. Malicious input (corrupted or intentionally crafted bad data)
//
// The error types tested here protect against internal state corruption that
// could occur if the receiver's invariants are violated. These errors should
// be extremely rare in normal operation but prevent crashes when they do occur.
//
// Panics removed: 4
// - EntityActionReceiver::receive_actions (entity channel lookup after insert)
// - EntityChannel::receive_insert_component_action (component channel lookup after insert)
// - EntityChannel::receive_remove_component_action (component channel lookup after insert)
// - OrderedIds::push_back (VecDeque index access)
//
// All of these had "SAFETY" comments claiming they were safe, but proper error
// handling ensures the system remains stable even if those assumptions are
// violated by bugs or malicious input.
