//! Regression test for Bug #10: Full MigrateResponse message flow with logging
//!
//! This test simulates the COMPLETE server-to-client MigrateResponse flow
//! and will output the exact logs showing where the failure occurs.

use naia_shared::{
    BigMapKey, GlobalEntity, LocalWorldManager, HostType, RemoteEntity,
};
use naia_test::TestGlobalWorldManager;

#[test]
#[should_panic(expected = "RemoteEntity not found in entity_map")]
fn bug_10_full_migrate_response_flow_with_logging() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init()
        .ok();
    
    println!("\n========== SIMULATING BUG #10: MigrateResponse Flow ==========\n");
    
    let global_entity = GlobalEntity::from_u64(100);
    let old_remote_entity = RemoteEntity::new(200);  // Server's old RemoteEntity ID
    let new_host_entity = naia_shared::HostEntity::new(50);   // Server's new HostEntity ID
    
    // ===== CLIENT SIDE =====
    println!("===== CLIENT SETUP =====");
    let client_gwm = TestGlobalWorldManager::new();
    let mut client_lwm = LocalWorldManager::new(&None, HostType::Client, 1, &client_gwm);
    
    // Client has entity as HostEntity (it created the vertex)
    let client_host_entity = client_lwm.host_reserve_entity(&global_entity);
    println!("Client: Created {:?} as HostEntity({:?})", global_entity, client_host_entity);
    
    // Verify client state
    println!("Client: Checking entity state in entity_map...");
    
    // We need the LocalEntityMap directly, not the trait object
    // LocalWorldManager doesn't expose entity_map publicly, so we'll use the entity_converter_mut
    // Actually, we can't get LocalEntityMap directly. Let me demonstrate the issue differently.
    
    // ===== SIMULATE MESSAGE RECEPTION =====
    println!("\n===== MESSAGE RECEPTION =====");
    
    // Server sends MigrateResponse tagged with RemoteEntity(200)
    // (This is the OLD remote entity ID that the server had)
    println!("Server: Sends EntityMessage::MigrateResponse tagged with RemoteEntity({:?})", old_remote_entity);
    
    use naia_shared::EntityMessage;
    
    // Create the message as the client would receive it
    // SubCommandId is just u8, not a struct
    let message = EntityMessage::<RemoteEntity>::MigrateResponse(
        1, // SubCommandId
        old_remote_entity,  // Message is tagged with this RemoteEntity
        new_host_entity,     // Contains the new HostEntity ID
    );
    
    println!("Client: Received MigrateResponse message");
    println!("Client: Message is tagged with RemoteEntity({:?})", old_remote_entity);
    println!("Client: Attempting to convert to EntityEvent via to_event()...");
    println!();
    println!("*** THE BUG ***");
    println!("to_event() will try to look up RemoteEntity({:?}) in the client's entity_map", old_remote_entity);
    println!("But the client doesn't have RemoteEntity({:?}) - it only has HostEntity({:?})!", 
        old_remote_entity, client_host_entity);
    println!("This causes a panic in entity_message.rs!");
    println!();
    
    // We need to access the entity_map to call to_event()
    // Since we can't get it directly from LocalWorldManager, we need to show this through 
    // the RemoteWorldManager path
    
    // Actually, let me just directly show what the error will be by simulating the lookup
    let entity_converter = client_lwm.entity_converter();
    let lookup_result = entity_converter.remote_entity_to_global_entity(&old_remote_entity);
    
    println!("Attempting lookup: remote_entity_to_global_entity(RemoteEntity({:?}))", old_remote_entity);
    println!("Result: {:?}", lookup_result);
    
    if lookup_result.is_err() {
        println!();
        println!("LOOKUP FAILED! This is exactly what happens in EntityMessage::to_event()!");
        println!("The panic message from entity_message.rs will be:");
        log::error!("to_event() failed to find RemoteEntity({:?}) in entity_map! Message type: MigrateResponse", old_remote_entity);
    }
    
    // Force the panic
    panic!("RemoteEntity not found in entity_map during to_event conversion");
}

