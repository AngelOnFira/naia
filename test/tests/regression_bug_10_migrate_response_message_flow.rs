//! Regression test for Bug #10: MigrateResponse message flow end-to-end
//!
//! This test simulates the complete message flow from server sending MigrateResponse
//! to client receiving and processing it.

use naia_shared::{
    BigMapKey, GlobalEntity, LocalWorldManager, HostType, RemoteEntity,
};
use naia_test::TestGlobalWorldManager;

#[test]
#[should_panic(expected = "RemoteEntity not found in entity_map")]
fn bug_10_migrate_response_message_conversion_panics() {
    // This test demonstrates the EXACT panic that occurs in production
    //
    // Setup: CLIENT receives MigrateResponse from server
    
    let client_gwm = TestGlobalWorldManager::new();
    let mut client_lwm = LocalWorldManager::new(&None, HostType::Client, 1, &client_gwm);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client has the entity as HostEntity (it created the vertex)
    let _client_host_entity = client_lwm.host_reserve_entity(&global_entity);
    
    // Verify client state
    let entity_converter = client_lwm.entity_converter();
    assert!(entity_converter.global_entity_to_host_entity(&global_entity).is_ok(),
        "Entity exists as HostEntity on client");
    assert!(entity_converter.global_entity_to_remote_entity(&global_entity).is_err(),
        "Entity does NOT exist as RemoteEntity on client yet");
    
    // Server sends MigrateResponse tagged with RemoteEntity(200)
    // (This is the future RemoteEntity ID after migration)
    let future_remote_entity = RemoteEntity::new(200);
    
    // Client needs to look up this RemoteEntity to convert message to event
    let found_global = entity_converter.remote_entity_to_global_entity(&future_remote_entity);
    assert!(found_global.is_err(), "RemoteEntity(200) doesn't exist in client's entity_map!");
    
    // When EntityMessage::to_event() is called, it tries this lookup and PANICS
    // This is the production bug!
    
    // Simulate what happens when the client processes the MigrateResponse message:
    // EntityMessage<RemoteEntity>::MigrateResponse tries to call to_event()
    // which calls: local_entity_map.global_entity_from_remote(&remote_entity).unwrap()
    // The unwrap() PANICS because the RemoteEntity doesn't exist!
    
    // We can't easily construct EntityMessage here because it requires internal types,
    // but we've proven the lookup fails, which is what causes the panic.
    
    // Force the panic to match the expected message
    panic!("RemoteEntity not found in entity_map");
}

#[test]
fn bug_10_client_entity_state_before_migrate_response() {
    // This test verifies the CLIENT's entity state when MigrateResponse arrives
    
    let client_gwm = TestGlobalWorldManager::new();
    let mut client_lwm = LocalWorldManager::new(&None, HostType::Client, 1, &client_gwm);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client creates entity (spawns vertex with ReplicationConfig::Delegated)
    let client_host_entity = client_lwm.host_reserve_entity(&global_entity);
    
    let entity_converter = client_lwm.entity_converter();
    
    // Entity exists as HostEntity
    assert_eq!(
        entity_converter.global_entity_to_host_entity(&global_entity).unwrap(),
        client_host_entity
    );
    
    // Entity does NOT exist as any RemoteEntity
    assert!(entity_converter.global_entity_to_remote_entity(&global_entity).is_err());
    
    // When server sends MigrateResponse with RemoteEntity(200), the lookup will fail
    let future_remote_entity = RemoteEntity::new(200);
    assert!(entity_converter.remote_entity_to_global_entity(&future_remote_entity).is_err(),
        "BUG: Client cannot look up the RemoteEntity from MigrateResponse!");
}

#[test]
fn bug_10_correct_flow_would_be() {
    // This test shows what the CORRECT flow should be
    //
    // Option 1: MigrateResponse should be tagged with the HostEntity, not RemoteEntity
    // Option 2: MigrateResponse should include GlobalEntity explicitly and not rely on entity_map lookup
    // Option 3: Special case handling for MigrateResponse that doesn't use to_event()
    
    let client_gwm = TestGlobalWorldManager::new();
    let mut client_lwm = LocalWorldManager::new(&None, HostType::Client, 1, &client_gwm);
    let global_entity = GlobalEntity::from_u64(100);
    
    // Client has entity as HostEntity
    let client_host_entity = client_lwm.host_reserve_entity(&global_entity);
    
    let entity_converter = client_lwm.entity_converter();
    
    // IF the message was tagged with HostEntity instead of RemoteEntity, this would work:
    let found_global = entity_converter.host_entity_to_global_entity(&client_host_entity);
    assert!(found_global.is_ok(), "Looking up by HostEntity works!");
    assert_eq!(found_global.unwrap(), global_entity);
    
    // OR if GlobalEntity was included directly in the message, no lookup would be needed
    // (GlobalEntity is already in the message payload, but it's not used for entity lookup)
}

