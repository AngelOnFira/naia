// Integration tests for EntityChannel error handling
// Tests all panic-free error paths for the critical entity state machine module
//
// Note: EntityChannel is an internal module, so these tests focus on the error types
// that protect the entity state machine. The actual EntityChannel state transitions
// are tested through WorldChannel integration tests.

use naia_shared::WorldChannelError;

// ========== Error Type Tests ==========
// These tests verify that WorldChannelError has all EntityChannel-specific variants

#[test]
fn test_entity_not_spawning_state_error() {
    let error = WorldChannelError::EntityNotSpawningState {
        entity_id: "Entity(1)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("not in spawning state"));
}

#[test]
fn test_entity_has_pending_components_error() {
    let error = WorldChannelError::EntityHasPendingComponents {
        entity_id: "Entity(2)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(2)"));
    assert!(msg.contains("pending components"));
}

#[test]
fn test_entity_not_yet_spawned_error() {
    let error = WorldChannelError::EntityNotYetSpawned {
        entity_id: "Entity(3)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(3)"));
    assert!(msg.contains("not in spawned state"));
}

#[test]
fn test_component_insert_entity_not_spawned_error() {
    let error = WorldChannelError::ComponentInsertEntityNotSpawned {
        entity_id: "Entity(4)".to_string(),
        component_kind: "Position".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(4)"));
    assert!(msg.contains("Position"));
    assert!(msg.contains("not spawned"));
}

#[test]
fn test_component_remove_while_inserting_error() {
    let error = WorldChannelError::ComponentRemoveWhileInserting {
        entity_id: "Entity(5)".to_string(),
        component_kind: "Velocity".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(5)"));
    assert!(msg.contains("Velocity"));
    assert!(msg.contains("still being inserted"));
}

#[test]
fn test_component_operation_wrong_state_error() {
    let error = WorldChannelError::ComponentOperationWrongState {
        operation: "insertion_complete",
        expected: "Inserting",
        actual: "Inserted",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("insertion_complete"));
    assert!(msg.contains("Inserting"));
    assert!(msg.contains("Inserted"));
}

#[test]
fn test_authority_released_error() {
    let error = WorldChannelError::AuthorityReleased {
        entity_id: "Entity(6)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(6)"));
    assert!(msg.contains("authority was released"));
}

#[test]
fn test_auth_release_message_failed_error() {
    let error = WorldChannelError::AuthReleaseMessageFailed {
        entity_id: "Entity(7)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(7)"));
    assert!(msg.contains("channel dropped while waiting"));
}

// ========== Documentation ==========
//
// EntityChannel is an internal module that manages per-entity state machines.
// It tracks entity spawning, component insertion/removal, and authority delegation.
//
// The error types tested here protect against:
// 1. State machine violations (completing spawn when not spawning)
// 2. Component operation race conditions (removing while inserting)
// 3. Authority conflicts (sending messages after authority released)
// 4. Resource leaks (dropping channel with pending auth release)
//
// These errors prevent panics that would crash the server and are critical
// for production stability. The actual state transitions are tested through
// WorldChannel integration tests since EntityChannel is private.
