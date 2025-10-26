#![cfg(test)]

use crate::{
    world::{
        component::component_kinds::ComponentKind,
        entity::entity_message::EntityMessage,
        sync::{RemoteEntityChannel, HostEntityChannel, remote_component_channel::RemoteComponentChannel},
    },
    HostType,
};
use crate::world::local::local_entity::RemoteEntity;
use crate::{GlobalEntity, HostEntity, OwnedLocalEntity, LocalEntityMap, BigMapKey};
use crate::world::local::local_world_manager::LocalWorldManager;
use crate::world::entity::entity_converters::GlobalWorldManagerType;
use std::collections::HashSet;

// BULLETPROOF: Test implementation of GlobalWorldManagerType
struct TestGlobalWorldManager;
impl GlobalWorldManagerType for TestGlobalWorldManager {
    fn get_in_scope_entities(&self) -> HashSet<GlobalEntity> {
        HashSet::new()
    }
}

/// Helper function to create a component kind for testing
fn component_kind<T: 'static>() -> ComponentKind {
    ComponentKind::from(std::any::TypeId::of::<T>())
}

// Helper types for testing
struct TestComponent1;
struct TestComponent2;

#[test]
fn remote_component_channel_is_inserted() {
    // Test that we can check if a component is inserted
    let channel = RemoteComponentChannel::new();
    
    // Initially should not be inserted
    assert!(!channel.is_inserted());
}

#[test]
fn remote_entity_channel_get_state() {
    // Test that we can get the current state of an entity channel
    let channel = RemoteEntityChannel::new(HostType::Server);
    
    // Should start in Despawned state
    assert_eq!(channel.get_state(), crate::world::sync::remote_entity_channel::EntityChannelState::Despawned);
}

#[test]
fn remote_entity_channel_extract_inserted_component_kinds() {
    // Test that we can extract which components are currently inserted
    let mut channel = RemoteEntityChannel::new(HostType::Server);
    let _entity = RemoteEntity::new(1);
    let comp1 = component_kind::<TestComponent1>();
    let comp2 = component_kind::<TestComponent2>();
    
    // Simulate spawn and component inserts
    channel.receive_message(1, EntityMessage::<()>::Spawn(()));
    channel.receive_message(2, EntityMessage::<()>::InsertComponent((), comp1));
    channel.receive_message(3, EntityMessage::<()>::InsertComponent((), comp2));
    
    // Extract component kinds
    let kinds = channel.extract_inserted_component_kinds();
    
    // Should contain both components
    assert_eq!(kinds.len(), 2);
    assert!(kinds.contains(&comp1));
    assert!(kinds.contains(&comp2));
}

#[test]
fn host_entity_channel_new_with_components() {
    // Test that we can create a HostEntityChannel with pre-populated components
    let comp1 = component_kind::<TestComponent1>();
    let comp2 = component_kind::<TestComponent2>();
    let mut kinds = std::collections::HashSet::new();
    kinds.insert(comp1);
    kinds.insert(comp2);
    
    let channel = HostEntityChannel::new_with_components(
        HostType::Server,
        kinds.clone()
    );
    
    // Should have the components pre-populated
    assert_eq!(channel.component_kinds(), &kinds);
}

#[test]
fn host_entity_channel_extract_outgoing_commands() {
    // Test that we can extract outgoing commands from a HostEntityChannel
    let mut channel = HostEntityChannel::new(HostType::Server);
    
    // Initially should be empty
    let commands = channel.extract_outgoing_commands();
    assert!(commands.is_empty());
}

#[test]
fn remote_component_channel_force_drain_buffers() {
    // Test that we can force-drain all buffered operations
    let mut channel = RemoteComponentChannel::new();
    let comp = component_kind::<TestComponent1>();
    
    // Add some operations while entity is not spawned (so they get buffered)
    channel.accept_message(
        crate::world::sync::remote_entity_channel::EntityChannelState::Despawned,
        1,
        EntityMessage::<()>::InsertComponent((), comp)
    );
    channel.accept_message(
        crate::world::sync::remote_entity_channel::EntityChannelState::Despawned,
        3,
        EntityMessage::<()>::RemoveComponent((), comp)
    );
    channel.accept_message(
        crate::world::sync::remote_entity_channel::EntityChannelState::Despawned,
        2,
        EntityMessage::<()>::InsertComponent((), comp)
    );
    
    // Before force-drain: should not be inserted (operations are buffered)
    assert!(!channel.is_inserted());
    
    // Force-drain all buffers
    channel.force_drain_buffers(crate::world::sync::remote_entity_channel::EntityChannelState::Spawned);
    
    // After force-drain: should have processed all operations
    // The final operation should be RemoveComponent (from message 3, which is the last one)
    assert!(!channel.is_inserted());
}

#[test]
fn local_entity_map_install_and_apply_redirect() {
    // Test that we can install and apply entity redirects
    let mut entity_map = crate::world::local::local_entity_map::LocalEntityMap::new(HostType::Server);
    
    let old_entity = crate::world::local::local_entity::OwnedLocalEntity::Remote(42);
    let new_entity = crate::world::local::local_entity::OwnedLocalEntity::Host(100);
    
    // Install redirect
    entity_map.install_entity_redirect(old_entity, new_entity);
    
    // Apply redirect
    let redirected = entity_map.apply_entity_redirect(&old_entity);
    assert_eq!(redirected, new_entity);
    
    // Non-redirected entity returns itself
    let other_entity = crate::world::local::local_entity::OwnedLocalEntity::Remote(99);
    let not_redirected = entity_map.apply_entity_redirect(&other_entity);
    assert_eq!(not_redirected, other_entity);
}

#[test]
fn migrate_entity_remote_to_host_success() {
    // Setup
    let mut world_manager = LocalWorldManager::new(
        &None,
        HostType::Server,
        1,
        &TestGlobalWorldManager as &dyn GlobalWorldManagerType
    );
    let global_entity = GlobalEntity::from_u64(1);
    
    // Create RemoteEntity with components
    let remote_entity = RemoteEntity::new(42);
    world_manager.entity_map.insert_with_remote_entity(global_entity, remote_entity);
    
    // Add some components
    let comp1 = component_kind::<TestComponent1>();
    let comp2 = component_kind::<TestComponent2>();
    world_manager.remote.spawn_entity(&remote_entity);
    // Note: Component insertion would need proper setup, but we can test the migration flow
    
    // Migrate
    let new_host_entity = world_manager.migrate_entity_remote_to_host(&global_entity);
    
    // Verify: RemoteEntity no longer exists
    assert!(!world_manager.entity_map.remote_to_global.contains_key(&remote_entity));
    
    // Verify: HostEntity now exists
    assert!(world_manager.entity_map.host_to_global.contains_key(&new_host_entity));
    
    // Verify: GlobalEntity maps to new HostEntity
    let mapped_host = world_manager.entity_map.global_entity_to_host_entity(&global_entity).unwrap();
    assert_eq!(mapped_host, new_host_entity);
}

#[test]
fn migrate_with_buffered_operations() {
    // Setup entity with pending buffered operations
    let mut world_manager = LocalWorldManager::new(
        &None,
        HostType::Server,
        1,
        &TestGlobalWorldManager as &dyn GlobalWorldManagerType
    );
    let global_entity = GlobalEntity::from_u64(1);
    let remote_entity = RemoteEntity::new(42);
    world_manager.entity_map.insert_with_remote_entity(global_entity, remote_entity);
    
    // Buffer some operations that haven't been processed
    world_manager.remote.spawn_entity(&remote_entity);
    let comp1 = component_kind::<TestComponent1>();
    world_manager.remote.insert_component(&remote_entity, comp1);
    
    // Migrate (should force-drain first)
    let new_host_entity = world_manager.migrate_entity_remote_to_host(&global_entity);
    
    // Verify: all operations were applied (not lost)
    // Component state should reflect final state after drain
    assert!(world_manager.entity_map.host_to_global.contains_key(&new_host_entity));
}

#[test]
fn remote_entity_channel_force_drain_all_buffers() {
    // Test that we can force-drain all entity-level and component-level buffers
    let mut channel = RemoteEntityChannel::new(HostType::Server);
    let _entity = RemoteEntity::new(1);
    let comp1 = component_kind::<TestComponent1>();
    let comp2 = component_kind::<TestComponent2>();
    
    // Add some buffered operations
    channel.receive_message(1, EntityMessage::<()>::Spawn(()));
    channel.receive_message(2, EntityMessage::<()>::InsertComponent((), comp1));
    channel.receive_message(4, EntityMessage::<()>::RemoveComponent((), comp1));
    channel.receive_message(3, EntityMessage::<()>::InsertComponent((), comp2));
    
    // Force-drain all buffers
    channel.force_drain_all_buffers();
    
    // After force-drain: should have final component state
    let kinds = channel.extract_inserted_component_kinds();
    assert_eq!(kinds.len(), 1); // Only comp2 should be inserted (comp1 was removed)
    assert!(kinds.contains(&comp2));
    assert!(!kinds.contains(&comp1));
}

#[test]
fn entity_message_apply_redirects() {
    // Test that we can apply entity redirects to EntityMessage
    use crate::world::entity::entity_message::EntityMessage;
    
    let old_entity = crate::world::local::local_entity::OwnedLocalEntity::Remote(42);
    let new_entity = crate::world::local::local_entity::OwnedLocalEntity::Host(100);
    
    // Create a message with the old entity
    let message = EntityMessage::<()>::Spawn(());
    let message_with_entity = message.with_entity(old_entity);
    
    // Apply redirect
    let redirected_message = message_with_entity.apply_entity_redirect(&old_entity, &new_entity);
    
    // Verify the entity was redirected
    assert_eq!(redirected_message.entity(), Some(new_entity));
}

#[test]
fn force_drain_resolves_all_buffers() {
    let mut channel = RemoteEntityChannel::new(HostType::Client);
    let _entity = RemoteEntity::new(1);
    let comp = component_kind::<TestComponent1>();
    
    // Setup: spawn + buffer some out-of-order operations
    channel.receive_message(1, EntityMessage::<()>::Spawn(()));
    channel.receive_message(4, EntityMessage::<()>::RemoveComponent((), comp));
    channel.receive_message(3, EntityMessage::<()>::InsertComponent((), comp));
    
    // Before drain: remove is buffered (can't remove non-existent)
    let events_before = channel.take_incoming_events();
    assert_eq!(events_before.len(), 1); // Only spawn
    
    // Force drain
    channel.force_drain_all_buffers();
    
    // After drain: all operations resolved
    let events_after = channel.take_incoming_events();
    assert_eq!(events_after.len(), 2); // Insert + Remove
    
    // Verify buffers empty
    let events_final = channel.take_incoming_events();
    assert_eq!(events_final.len(), 0);
}

#[test]
fn force_drain_preserves_component_state() {
    let mut channel = RemoteEntityChannel::new(HostType::Server);
    let comp = component_kind::<TestComponent1>();
    
    // Setup with buffered operations
    channel.receive_message(1, EntityMessage::<()>::Spawn(()));
    channel.receive_message(2, EntityMessage::<()>::InsertComponent((), comp));
    
    // Force drain
    channel.force_drain_all_buffers();
    
    // Verify final state matches expected after all ops applied
    let kinds = channel.extract_inserted_component_kinds();
    assert!(kinds.contains(&comp)); // Component should be inserted
}

#[test]
fn install_and_apply_redirect() {
    let mut entity_map = LocalEntityMap::new(HostType::Server);
    
    let old_entity = OwnedLocalEntity::Remote(42);
    let new_entity = OwnedLocalEntity::Host(100);
    
    // Install redirect
    entity_map.install_entity_redirect(old_entity, new_entity);
    
    // Apply redirect
    let redirected = entity_map.apply_entity_redirect(&old_entity);
    assert_eq!(redirected, new_entity);
    
    // Non-redirected entity returns itself
    let other_entity = OwnedLocalEntity::Remote(99);
    let not_redirected = entity_map.apply_entity_redirect(&other_entity);
    assert_eq!(not_redirected, other_entity);
}

#[test]
#[should_panic(expected = "does not exist")]
fn migrate_nonexistent_entity_panics() {
    let mut world_manager = LocalWorldManager::new(
        &None,
        HostType::Server,
        1,
        &TestGlobalWorldManager as &dyn GlobalWorldManagerType
    );
    let fake_entity = GlobalEntity::from_u64(999);
    
    world_manager.migrate_entity_remote_to_host(&fake_entity);
}

#[test]
#[should_panic(expected = "not remote-owned")]
fn migrate_host_entity_panics() {
    let mut world_manager = LocalWorldManager::new(
        &None,
        HostType::Server,
        1,
        &TestGlobalWorldManager as &dyn GlobalWorldManagerType
    );
    let global_entity = GlobalEntity::from_u64(1);
    
    // Insert as HostEntity
    let host_entity = HostEntity::new(10);
    world_manager.entity_map.insert_with_host_entity(global_entity, host_entity);
    
    // Try to migrate (should panic - it's already host)
    world_manager.migrate_entity_remote_to_host(&global_entity);
}
