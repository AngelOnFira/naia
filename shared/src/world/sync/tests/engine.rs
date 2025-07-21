#![cfg(test)]

use crate::world::sync::Engine;
use crate::world::entity::entity_message::EntityMessage;
use crate::world::entity::local_entity::RemoteEntity;
use crate::world::component::component_kinds::ComponentKind;

#[test]
fn engine_basic() {

    let mut engine: Engine<RemoteEntity> = Engine::new();
    
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.push(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(2, EntityMessage::InsertComponent(entity, comp));
    engine.push(3, EntityMessage::RemoveComponent(entity, comp));
    engine.push(4, EntityMessage::DespawnEntity(entity));

    // Drain and check order
    let out = engine.drain();
    assert_eq!(out.len(), 4);
    assert!(matches!(out[0], EntityMessage::SpawnEntity(e, _) if e == entity));
    assert!(matches!(out[1], EntityMessage::InsertComponent(e, k) if e == entity && k == comp));
    assert!(matches!(out[2], EntityMessage::RemoveComponent(e, k) if e == entity && k == comp));
    assert!(matches!(out[3], EntityMessage::DespawnEntity(e) if e == entity));
} 