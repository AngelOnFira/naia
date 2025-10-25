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
