//! Regression test for Bug #10: MigrateResponse fails to convert because RemoteEntity doesn't exist
//!
//! ## The Bug
//! When the server sends MigrateResponse to the client, it includes:
//! - GlobalEntity
//! - old_remote_entity (the RemoteEntity ID the entity WILL have after migration)
//! - new_host_entity (the HostEntity ID on the server)
//!
//! The MigrateResponse is tagged with the old_remote_entity ID as the message's entity.
//! When the client receives this message, it tries to convert it to an EntityEvent by calling:
//! `local_entity_map.global_entity_from_remote(&remote_entity)`
//!
//! But the entity is still a HostEntity on the client! The RemoteEntity doesn't exist yet!
//! So the lookup fails, and the MigrateResponse is never processed.
//!
//! ## Root Cause
//! The MigrateResponse message is tagged with the FUTURE RemoteEntity ID, but the client
//! doesn't have that mapping yet. The entity is still registered as a HostEntity.
//!
//! ## Expected Behavior
//! MigrateResponse should use the GlobalEntity or HostEntity for lookup, not the future RemoteEntity.

use naia_shared::{
    BigMapKey, GlobalEntity, LocalWorldManager, HostType, RemoteEntity, HostEntity,
};
use naia_test::TestGlobalWorldManager;

#[test]
fn bug_10_migrate_response_remote_entity_lookup_fails() {
    // This test demonstrates the bug: when MigrateResponse arrives, 
    // the RemoteEntity doesn't exist in the entity_map yet
    
    let global_world_manager = TestGlobalWorldManager::new();
    let mut lwm = LocalWorldManager::new(&None, HostType::Client, 1, &global_world_manager);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client creates entity as HostEntity (this is what happens when vertex is spawned)
    let host_entity = lwm.host_reserve_entity(&global_entity);
    
    // Server would send MigrateResponse with RemoteEntity(200) as the future entity ID
    // But that RemoteEntity doesn't exist in the client's entity_map yet!
    
    let entity_converter = lwm.entity_converter();
    
    // Entity exists as HostEntity
    let found_host = entity_converter.global_entity_to_host_entity(&global_entity);
    assert!(found_host.is_ok(), "Entity exists as HostEntity");
    assert_eq!(found_host.unwrap(), host_entity);
    
    // But RemoteEntity doesn't exist
    let found_remote = entity_converter.global_entity_to_remote_entity(&global_entity);
    assert!(found_remote.is_err(), "Entity does NOT exist as RemoteEntity yet");
    
    // When MigrateResponse tries to look up RemoteEntity(200), it fails
    let future_remote_entity = RemoteEntity::new(200);
    let found_global = entity_converter.remote_entity_to_global_entity(&future_remote_entity);
    assert!(found_global.is_err(), 
        "BUG: MigrateResponse tries to look up RemoteEntity that doesn't exist!");
}

#[test]
fn bug_10_entity_exists_as_host_not_remote() {
    // This test verifies that the entity exists as HostEntity but not RemoteEntity
    
    let global_world_manager = TestGlobalWorldManager::new();
    let mut lwm = LocalWorldManager::new(&None, HostType::Client, 1, &global_world_manager);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client creates entity as HostEntity
    let host_entity = lwm.host_reserve_entity(&global_entity);
    
    let entity_converter = lwm.entity_converter();
    
    // Verify entity exists as HostEntity
    let found_host = entity_converter.global_entity_to_host_entity(&global_entity);
    assert!(found_host.is_ok(), "Entity should exist as HostEntity");
    assert_eq!(found_host.unwrap(), host_entity);
    
    // Verify entity DOES NOT exist as RemoteEntity
    let found_remote = entity_converter.global_entity_to_remote_entity(&global_entity);
    assert!(found_remote.is_err(), "Entity should NOT exist as RemoteEntity yet");
    
    // Try to look up a RemoteEntity that doesn't exist
    let fake_remote_entity = RemoteEntity::new(200);
    let found_global = entity_converter.remote_entity_to_global_entity(&fake_remote_entity);
    assert!(found_global.is_err(), "RemoteEntity(200) should not exist in entity_map");
}

#[test]
fn bug_10_migrate_response_should_use_global_entity() {
    // This test shows what the fix should be:
    // MigrateResponse should be tagged with GlobalEntity or use a different lookup method
    
    let global_world_manager = TestGlobalWorldManager::new();
    let mut lwm = LocalWorldManager::new(&None, HostType::Client, 1, &global_world_manager);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client creates entity as HostEntity
    let _host_entity = lwm.host_reserve_entity(&global_entity);
    
    // The fix: MigrateResponse should include GlobalEntity in a way that allows
    // lookup even when the RemoteEntity doesn't exist yet
    
    // Current structure: EntityMessage::MigrateResponse(SubCommandId, E, HostEntity)
    // The E is the entity the message is tagged with (RemoteEntity in this case)
    
    // Proposed fix: Either:
    // 1. Tag message with HostEntity instead of RemoteEntity
    // 2. Include GlobalEntity explicitly in the message payload
    // 3. Have a special case for MigrateResponse that doesn't require entity_map lookup
    
    // For now, this test documents the issue
    let entity_converter = lwm.entity_converter();
    
    // GlobalEntity lookup works
    let found_host = entity_converter.global_entity_to_host_entity(&global_entity);
    assert!(found_host.is_ok(), "Can look up by GlobalEntity");
    
    // RemoteEntity lookup fails
    let fake_remote = RemoteEntity::new(200);
    let found_global = entity_converter.remote_entity_to_global_entity(&fake_remote);
    assert!(found_global.is_err(), "Cannot look up by RemoteEntity that doesn't exist");
}

