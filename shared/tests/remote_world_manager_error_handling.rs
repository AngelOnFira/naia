/// Tests for RemoteWorldManager panic-free error handling
///
/// RemoteWorldManager handles client-side entity and component replication.
/// These tests ensure that malformed network data and internal inconsistencies
/// cannot crash the client.

use std::collections::HashSet;

use naia_shared::{
    RemoteEntity, RemoteWorldError, EntityWaitlist, WaitlistStore,
};

#[test]
fn test_entity_waitlist_try_remove_waiting_handle_missing() {
    let mut waitlist = EntityWaitlist::new();
    let invalid_handle = 999;

    // Try to remove a handle that doesn't exist
    let result = waitlist.try_remove_waiting_handle(&invalid_handle);

    assert!(result.is_err());
    match result.unwrap_err() {
        RemoteWorldError::WaitlistHandleMissing { handle } => {
            assert_eq!(handle, invalid_handle);
        }
        other => panic!("Expected WaitlistHandleMissing error, got {:?}", other),
    }
}

#[test]
fn test_entity_waitlist_try_remove_waiting_handle_success() {
    let mut waitlist = EntityWaitlist::new();
    let mut store = WaitlistStore::<()>::new();
    let remote_entity = RemoteEntity::new(1);

    let mut entities = HashSet::new();
    entities.insert(remote_entity);

    // Queue an item to get a handle
    let handle = waitlist.queue(&entities, &mut store, ());

    // Should be able to remove it successfully
    let result = waitlist.try_remove_waiting_handle(&handle);
    assert!(result.is_ok());
}

#[test]
fn test_entity_waitlist_remove_waiting_handle_panics_on_missing() {
    let mut waitlist = EntityWaitlist::new();
    let invalid_handle = 999;

    // This should panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        waitlist.remove_waiting_handle(&invalid_handle);
    }));

    assert!(result.is_err());
}

#[test]
fn test_waitlist_store_try_collect_ready_items_no_intersection() {
    let mut store = WaitlistStore::<()>::new();
    let mut ready_handles = HashSet::new();

    // Add a handle that doesn't exist in the store
    // This should return Ok(None) because there's no intersection
    ready_handles.insert(999);

    let result = store.try_collect_ready_items(&mut ready_handles);

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_waitlist_store_try_collect_ready_items_success() {
    let mut store = WaitlistStore::<u32>::new();
    let mut ready_handles = HashSet::new();

    let handle = 1;
    store.queue(handle, 42);
    ready_handles.insert(handle);

    // This should succeed
    let result = store.try_collect_ready_items(&mut ready_handles);

    assert!(result.is_ok());
    let items = result.unwrap();
    assert!(items.is_some());
    assert_eq!(items.unwrap(), vec![42]);
}


#[test]
fn test_waitlist_store_collect_ready_items_no_panic_on_no_intersection() {
    let mut store = WaitlistStore::<()>::new();
    let mut ready_handles = HashSet::new();

    // Add a handle that doesn't exist in the store
    // This should NOT panic, just return None
    ready_handles.insert(999);

    let result = store.collect_ready_items(&mut ready_handles);
    assert!(result.is_none());
}

#[test]
fn test_waitlist_store_multiple_items_success() {
    let mut store = WaitlistStore::<String>::new();
    let mut ready_handles = HashSet::new();

    // Queue multiple items
    store.queue(1, "first".to_string());
    store.queue(2, "second".to_string());
    store.queue(3, "third".to_string());

    // Mark first and third as ready
    ready_handles.insert(1);
    ready_handles.insert(3);

    // Collect ready items
    let result = store.try_collect_ready_items(&mut ready_handles);

    assert!(result.is_ok());
    let items = result.unwrap();
    assert!(items.is_some());

    let items_vec = items.unwrap();
    assert_eq!(items_vec.len(), 2);
    assert!(items_vec.contains(&"first".to_string()));
    assert!(items_vec.contains(&"third".to_string()));
}

#[test]
fn test_entity_waitlist_add_remove_entity() {
    let mut waitlist = EntityWaitlist::new();
    let remote_entity = RemoteEntity::new(42);

    // Add entity
    waitlist.add_entity(&remote_entity);

    // Remove entity (should not panic)
    waitlist.remove_entity(&remote_entity);
}

#[test]
fn test_entity_waitlist_queue_with_all_entities_in_scope() {
    let mut waitlist = EntityWaitlist::new();
    let mut store = WaitlistStore::<String>::new();
    let remote_entity = RemoteEntity::new(1);

    // Add entity to scope first
    waitlist.add_entity(&remote_entity);

    let mut entities = HashSet::new();
    entities.insert(remote_entity);

    // Queue an item - should be immediately ready since entity is in scope
    let _handle = waitlist.queue(&entities, &mut store, "test".to_string());

    // Collect ready items
    let result = waitlist.collect_ready_items(&naia_socket_shared::Instant::now(), &mut store);

    assert!(result.is_some());
    let items = result.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], "test".to_string());
}

#[test]
fn test_entity_waitlist_queue_then_add_entity_makes_ready() {
    let mut waitlist = EntityWaitlist::new();
    let mut store = WaitlistStore::<u32>::new();
    let remote_entity = RemoteEntity::new(1);

    let mut entities = HashSet::new();
    entities.insert(remote_entity);

    // Queue an item before entity is in scope
    let _handle = waitlist.queue(&entities, &mut store, 42);

    // Item should not be ready yet
    let result = waitlist.collect_ready_items(&naia_socket_shared::Instant::now(), &mut store);
    assert!(result.is_none());

    // Add entity to scope
    waitlist.add_entity(&remote_entity);

    // Now item should be ready
    let result = waitlist.collect_ready_items(&naia_socket_shared::Instant::now(), &mut store);
    assert!(result.is_some());
    let items = result.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], 42);
}

#[test]
fn test_remote_world_error_display() {
    // Test that error messages are formatted correctly
    let error = RemoteWorldError::ComponentDataMissingDuringSpawn {
        entity_id: "RemoteEntity(123)".to_string(),
        component_kind: "Position".to_string(),
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Component"));
    assert!(error_msg.contains("missing"));
    assert!(error_msg.contains("spawn"));

    let error = RemoteWorldError::WaitlistHandleMissing { handle: 42 };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Waitlist handle"));
    assert!(error_msg.contains("42"));

    let error = RemoteWorldError::MalformedComponentUpdate {
        entity_id: "Entity1".to_string(),
        component_kind: "Transform".to_string(),
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Malformed"));
    assert!(error_msg.contains("component update"));
}

#[test]
fn test_remote_world_error_clone() {
    // Test that errors can be cloned
    let error = RemoteWorldError::WaitlistItemMissing { handle: 100 };
    let cloned = error.clone();

    match cloned {
        RemoteWorldError::WaitlistItemMissing { handle } => {
            assert_eq!(handle, 100);
        }
        _ => panic!("Clone didn't preserve error variant"),
    }
}

#[test]
fn test_remote_world_error_debug() {
    // Test that errors implement Debug
    let error = RemoteWorldError::HandleTtlQueueEmpty;
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("HandleTtlQueueEmpty"));
}
