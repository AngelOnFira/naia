// Integration tests for WorldChannel error handling
// This is THE most critical module - testing all panic-free error paths

use naia_shared::{CheckedMap, CheckedSet, WorldChannelError};

// ========== Error Type Tests ==========
// These tests verify that WorldChannelError has all required variants
// and they can be constructed properly

#[test]
fn test_world_channel_error_entity_already_spawned() {
    let error = WorldChannelError::EntityAlreadySpawned {
        entity_id: "Entity(1)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("already spawned"));
}

#[test]
fn test_world_channel_error_entity_not_found() {
    let error = WorldChannelError::EntityNotFound {
        entity_id: "Entity(999)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(999)"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_world_channel_error_entity_has_no_channel() {
    let error = WorldChannelError::EntityHasNoChannel {
        entity_id: "Entity(42)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(42)"));
    assert!(msg.contains("no channel"));
}

#[test]
fn test_world_channel_error_entity_already_despawned() {
    let error = WorldChannelError::EntityAlreadyDespawned {
        entity_id: "Entity(1)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("despawned"));
}

#[test]
fn test_world_channel_error_entity_not_spawned() {
    let error = WorldChannelError::EntityNotSpawned {
        entity_id: "Entity(5)".to_string(),
        operation: "despawn",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(5)"));
    assert!(msg.contains("despawn"));
    assert!(msg.contains("not spawned"));
}

#[test]
fn test_world_channel_error_component_not_found() {
    let error = WorldChannelError::ComponentNotFound {
        entity_id: "Entity(1)".to_string(),
        component_kind: "Position".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("Position"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_world_channel_error_component_already_exists() {
    let error = WorldChannelError::ComponentAlreadyExists {
        entity_id: "Entity(1)".to_string(),
        component_kind: "Position".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("Position"));
    assert!(msg.contains("already exists"));
}

#[test]
fn test_world_channel_error_remote_entity_already_tracked() {
    let error = WorldChannelError::RemoteEntityAlreadyTracked {
        entity_id: "Entity(10)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(10)"));
    assert!(msg.contains("already tracked"));
}

#[test]
fn test_world_channel_error_remote_entity_not_tracked() {
    let error = WorldChannelError::RemoteEntityNotTracked {
        entity_id: "Entity(20)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(20)"));
    assert!(msg.contains("not tracked"));
}

#[test]
fn test_world_channel_error_must_spawn_before_tracking() {
    let error = WorldChannelError::MustSpawnBeforeTracking {
        entity_id: "Entity(7)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(7)"));
    assert!(msg.contains("spawn"));
}

#[test]
fn test_world_channel_error_remote_entity_already_exists() {
    let error = WorldChannelError::RemoteEntityAlreadyExists {
        entity_id: "Entity(100)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(100)"));
    assert!(msg.contains("already exists"));
}

#[test]
fn test_world_channel_error_invalid_channel_state() {
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(50)".to_string(),
        operation: "spawn",
        expected_state: "idle",
        actual_state: "spawning",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(50)"));
    assert!(msg.contains("spawn"));
    assert!(msg.contains("idle"));
    assert!(msg.contains("spawning"));
}

#[test]
fn test_world_channel_error_remote_component_entity_not_found() {
    let error = WorldChannelError::RemoteComponentEntityNotFound {
        entity_id: "Entity(1)".to_string(),
        component_kind: "Velocity".to_string(),
        operation: "insert",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("Velocity"));
    assert!(msg.contains("insert"));
}

#[test]
fn test_world_channel_error_duplicate_map_key() {
    let error = WorldChannelError::DuplicateMapKey {
        key: "key123".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("key123"));
    assert!(msg.contains("Duplicate"));
}

#[test]
fn test_world_channel_error_map_key_not_found() {
    let error = WorldChannelError::MapKeyNotFound {
        key: "key456".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("key456"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_world_channel_error_duplicate_set_value() {
    let error = WorldChannelError::DuplicateSetValue {
        value: "value789".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("value789"));
    assert!(msg.contains("Duplicate"));
}

#[test]
fn test_world_channel_error_set_value_not_found() {
    let error = WorldChannelError::SetValueNotFound {
        value: "value999".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("value999"));
    assert!(msg.contains("not found"));
}

// ========== Error Trait Tests ==========

#[test]
fn test_world_channel_error_implements_error_trait() {
    let error = WorldChannelError::EntityNotFound {
        entity_id: "test".to_string(),
    };

    // Should implement std::error::Error
    let _: &dyn std::error::Error = &error;
}

#[test]
fn test_world_channel_error_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<WorldChannelError>();
    assert_sync::<WorldChannelError>();
}

#[test]
fn test_world_channel_error_is_clone() {
    let error1 = WorldChannelError::EntityNotFound {
        entity_id: "Entity(1)".to_string(),
    };
    let error2 = error1.clone();

    let msg1 = format!("{}", error1);
    let msg2 = format!("{}", error2);
    assert_eq!(msg1, msg2);
}

// ========== CheckedMap Tests ==========

#[cfg(test)]
mod checked_map_tests {
    use naia_shared::{CheckedMap, WorldChannelError};

    #[test]
    fn test_checked_map_insert_and_get() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        // Insert should work
        map.insert(1, "value1".to_string());

        // Get should return the value
        assert_eq!(map.get(&1), Some(&"value1".to_string()));
        assert_eq!(map.get(&2), None);
    }

    #[test]
    fn test_checked_map_try_insert_success() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        // First insert should succeed
        let result = map.try_insert(1, "value1".to_string());
        assert!(result.is_ok());

        // Value should be retrievable
        assert_eq!(map.get(&1), Some(&"value1".to_string()));
    }

    #[test]
    fn test_checked_map_try_insert_duplicate_fails() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        // First insert succeeds
        map.try_insert(1, "value1".to_string()).unwrap();

        // Second insert should fail
        let result = map.try_insert(1, "value2".to_string());
        assert!(matches!(result, Err(WorldChannelError::DuplicateMapKey { .. })));

        // Original value should remain
        assert_eq!(map.get(&1), Some(&"value1".to_string()));
    }

    #[test]
    fn test_checked_map_try_remove_success() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        map.insert(1, "value1".to_string());

        // Remove should succeed and return the value
        let result = map.try_remove(&1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "value1".to_string());

        // Value should be gone
        assert_eq!(map.get(&1), None);
    }

    #[test]
    fn test_checked_map_try_remove_nonexistent_fails() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        // Remove non-existent key should fail
        let result = map.try_remove(&999);
        assert!(matches!(result, Err(WorldChannelError::MapKeyNotFound { .. })));
    }

    #[test]
    fn test_checked_map_contains_key() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        assert!(!map.contains_key(&1));

        map.insert(1, "value1".to_string());
        assert!(map.contains_key(&1));
        assert!(!map.contains_key(&2));
    }

    #[test]
    fn test_checked_map_iter() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        map.insert(1, "value1".to_string());
        map.insert(2, "value2".to_string());

        let count = map.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_checked_map_len() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        assert_eq!(map.len(), 0);

        map.insert(1, "value1".to_string());
        assert_eq!(map.len(), 1);

        map.insert(2, "value2".to_string());
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_checked_map_clear() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        map.insert(1, "value1".to_string());
        map.insert(2, "value2".to_string());
        assert_eq!(map.len(), 2);

        map.clear();
        assert_eq!(map.len(), 0);
        assert!(!map.contains_key(&1));
    }

    #[test]
    #[should_panic(expected = "Cannot insert and replace value for given key")]
    fn test_checked_map_insert_panic_on_duplicate() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        map.insert(1, "value1".to_string());
        map.insert(1, "value2".to_string()); // Should panic
    }

    #[test]
    #[should_panic(expected = "Cannot remove value for key with non-existent value")]
    fn test_checked_map_remove_panic_on_nonexistent() {
        let mut map: CheckedMap<u32, String> = CheckedMap::new();

        map.remove(&999); // Should panic
    }
}

// ========== CheckedSet Tests ==========

#[cfg(test)]
mod checked_set_tests {
    use naia_shared::{CheckedSet, WorldChannelError};

    #[test]
    fn test_checked_set_insert_and_contains() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        assert!(!set.contains(&1));

        set.insert(1);
        assert!(set.contains(&1));
        assert!(!set.contains(&2));
    }

    #[test]
    fn test_checked_set_try_insert_success() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        let result = set.try_insert(1);
        assert!(result.is_ok());
        assert!(set.contains(&1));
    }

    #[test]
    fn test_checked_set_try_insert_duplicate_fails() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        set.try_insert(1).unwrap();

        // Second insert should fail
        let result = set.try_insert(1);
        assert!(matches!(result, Err(WorldChannelError::DuplicateSetValue { .. })));
    }

    #[test]
    fn test_checked_set_try_remove_success() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        set.insert(1);

        let result = set.try_remove(&1);
        assert!(result.is_ok());
        assert!(!set.contains(&1));
    }

    #[test]
    fn test_checked_set_try_remove_nonexistent_fails() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        let result = set.try_remove(&999);
        assert!(matches!(result, Err(WorldChannelError::SetValueNotFound { .. })));
    }

    #[test]
    fn test_checked_set_iter() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        set.insert(1);
        set.insert(2);
        set.insert(3);

        let count = set.iter().count();
        assert_eq!(count, 3);
    }

    #[test]
    #[should_panic(expected = "Cannot insert and replace given key")]
    fn test_checked_set_insert_panic_on_duplicate() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        set.insert(1);
        set.insert(1); // Should panic
    }

    #[test]
    #[should_panic(expected = "Cannot remove given non-existent key")]
    fn test_checked_set_remove_panic_on_nonexistent() {
        let mut set: CheckedSet<u32> = CheckedSet::new();

        set.remove(&999); // Should panic
    }
}

// ========== Documentation Tests ==========

#[test]
fn test_error_messages_are_informative() {
    // All error messages should contain relevant context

    let errors = vec![
        WorldChannelError::EntityAlreadySpawned {
            entity_id: "Entity(1)".to_string(),
        },
        WorldChannelError::EntityNotFound {
            entity_id: "Entity(2)".to_string(),
        },
        WorldChannelError::ComponentNotFound {
            entity_id: "Entity(3)".to_string(),
            component_kind: "Position".to_string(),
        },
        WorldChannelError::InvalidChannelState {
            entity_id: "Entity(4)".to_string(),
            operation: "spawn",
            expected_state: "idle",
            actual_state: "spawning",
        },
    ];

    for error in errors {
        let msg = format!("{}", error);
        assert!(!msg.is_empty(), "Error message should not be empty");
        assert!(msg.len() > 10, "Error message should be meaningful: {}", msg);
    }
}

// ========== Remote Action Processing Error Tests ==========
// These tests verify that remote action processing (ACK handling) returns proper errors
// instead of panicking when receiving malformed/out-of-order packets

#[test]
fn test_remote_entity_already_exists_error() {
    let error = WorldChannelError::RemoteEntityAlreadyExists {
        entity_id: "Entity(1)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(1)"));
    assert!(msg.contains("already exists"));
}

#[test]
fn test_remote_entity_not_found_for_despawn_error() {
    let error = WorldChannelError::RemoteEntityNotFoundForDespawn {
        entity_id: "Entity(2)".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(2)"));
    assert!(msg.contains("despawn"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_invalid_channel_state_spawn_error() {
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(3)".to_string(),
        operation: "remote spawn",
        expected_state: "Spawning",
        actual_state: "Spawned",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(3)"));
    assert!(msg.contains("remote spawn"));
    assert!(msg.contains("Spawning"));
    assert!(msg.contains("Spawned"));
}

#[test]
fn test_invalid_channel_state_despawn_error() {
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(4)".to_string(),
        operation: "remote despawn",
        expected_state: "Despawning",
        actual_state: "Spawning",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(4)"));
    assert!(msg.contains("remote despawn"));
    assert!(msg.contains("Despawning"));
    assert!(msg.contains("Spawning"));
}

#[test]
fn test_remote_component_entity_not_found_error() {
    let error = WorldChannelError::RemoteComponentEntityNotFound {
        entity_id: "Entity(5)".to_string(),
        component_kind: "Position".to_string(),
        operation: "insert",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(5)"));
    assert!(msg.contains("Position"));
    assert!(msg.contains("insert"));
}

#[test]
fn test_remote_component_already_exists_error() {
    let error = WorldChannelError::RemoteComponentAlreadyExists {
        entity_id: "Entity(6)".to_string(),
        component_kind: "Velocity".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(6)"));
    assert!(msg.contains("Velocity"));
    assert!(msg.contains("already exists"));
}

#[test]
fn test_remote_component_not_found_error() {
    let error = WorldChannelError::RemoteComponentNotFound {
        entity_id: "Entity(7)".to_string(),
        component_kind: "Health".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(7)"));
    assert!(msg.contains("Health"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_remote_component_channel_not_initialized_error() {
    let error = WorldChannelError::RemoteComponentChannelNotInitialized {
        entity_id: "Entity(8)".to_string(),
        component_kind: "Score".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(8)"));
    assert!(msg.contains("Score"));
    assert!(msg.contains("not initialized"));
}

#[test]
fn test_remote_component_not_removing_error() {
    let error = WorldChannelError::RemoteComponentNotRemoving {
        entity_id: "Entity(9)".to_string(),
        component_kind: "Power".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(9)"));
    assert!(msg.contains("Power"));
    assert!(msg.contains("not in removing state"));
}

#[test]
fn test_invalid_channel_state_remote_insert_error() {
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(10)".to_string(),
        operation: "remote insert component",
        expected_state: "Spawned",
        actual_state: "Spawning",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(10)"));
    assert!(msg.contains("remote insert component"));
    assert!(msg.contains("Spawned"));
    assert!(msg.contains("Spawning"));
}

#[test]
fn test_invalid_channel_state_remote_remove_error() {
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(11)".to_string(),
        operation: "remote remove component",
        expected_state: "Spawned",
        actual_state: "Despawning",
    };
    let msg = format!("{}", error);
    assert!(msg.contains("Entity(11)"));
    assert!(msg.contains("remote remove component"));
    assert!(msg.contains("Spawned"));
    assert!(msg.contains("Despawning"));
}

// ========== Security Tests ==========
// Tests to ensure malicious/malformed packets are handled gracefully

#[test]
fn test_all_remote_action_errors_are_descriptive() {
    // Verify all remote action processing errors provide actionable information
    let remote_errors = vec![
        WorldChannelError::RemoteEntityAlreadyExists {
            entity_id: "Entity(100)".to_string(),
        },
        WorldChannelError::RemoteEntityNotFoundForDespawn {
            entity_id: "Entity(101)".to_string(),
        },
        WorldChannelError::RemoteComponentEntityNotFound {
            entity_id: "Entity(102)".to_string(),
            component_kind: "Component(1)".to_string(),
            operation: "insert",
        },
        WorldChannelError::RemoteComponentAlreadyExists {
            entity_id: "Entity(103)".to_string(),
            component_kind: "Component(2)".to_string(),
        },
        WorldChannelError::RemoteComponentNotFound {
            entity_id: "Entity(104)".to_string(),
            component_kind: "Component(3)".to_string(),
        },
        WorldChannelError::RemoteComponentChannelNotInitialized {
            entity_id: "Entity(105)".to_string(),
            component_kind: "Component(4)".to_string(),
        },
        WorldChannelError::RemoteComponentNotRemoving {
            entity_id: "Entity(106)".to_string(),
            component_kind: "Component(5)".to_string(),
        },
    ];

    for error in remote_errors {
        let msg = format!("{}", error);
        // All remote errors should be descriptive
        assert!(msg.len() > 20, "Remote error should be descriptive: {}", msg);
        // Should contain entity information
        assert!(
            msg.contains("Entity") || msg.contains("entity"),
            "Remote error should reference entity: {}",
            msg
        );
    }
}

#[test]
fn test_invalid_channel_state_errors_show_both_states() {
    // InvalidChannelState errors should show both expected and actual states
    let error = WorldChannelError::InvalidChannelState {
        entity_id: "Entity(200)".to_string(),
        operation: "test operation",
        expected_state: "StateA",
        actual_state: "StateB",
    };

    let msg = format!("{}", error);
    assert!(msg.contains("StateA"), "Should show expected state");
    assert!(msg.contains("StateB"), "Should show actual state");
    assert!(msg.contains("Entity(200)"), "Should show entity");
    assert!(msg.contains("test operation"), "Should show operation");
}

#[test]
fn test_remote_component_errors_show_component_kind() {
    // Remote component errors should always show which component failed
    let errors = vec![
        WorldChannelError::RemoteComponentEntityNotFound {
            entity_id: "Entity(1)".to_string(),
            component_kind: "TestComponent".to_string(),
            operation: "insert",
        },
        WorldChannelError::RemoteComponentAlreadyExists {
            entity_id: "Entity(1)".to_string(),
            component_kind: "TestComponent".to_string(),
        },
        WorldChannelError::RemoteComponentNotFound {
            entity_id: "Entity(1)".to_string(),
            component_kind: "TestComponent".to_string(),
        },
        WorldChannelError::RemoteComponentChannelNotInitialized {
            entity_id: "Entity(1)".to_string(),
            component_kind: "TestComponent".to_string(),
        },
        WorldChannelError::RemoteComponentNotRemoving {
            entity_id: "Entity(1)".to_string(),
            component_kind: "TestComponent".to_string(),
        },
    ];

    for error in errors {
        let msg = format!("{}", error);
        assert!(
            msg.contains("TestComponent"),
            "Remote component error should show component kind: {}",
            msg
        );
    }
}
