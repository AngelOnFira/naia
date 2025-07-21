#![cfg(test)]
//! Basic happy-path tests for `Engine` spawn/insert flow (step S4).

use crate::world::sync::Engine;
use crate::world::entity::entity_message::EntityMessage;
use crate::world::entity::local_entity::RemoteEntity;
use crate::world::component::component_kinds::ComponentKind;

#[test]
fn engine_basic_spawn_insert() {
    struct DummyRoot;
    let mut engine: Engine<DummyRoot> = Engine::new();

    // Feed spawn and insert events
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(EntityMessage::InsertComponent(entity, comp));

    // Drain and check order
    let out = engine.drain();
    assert_eq!(out.len(), 2);
    assert!(matches!(out[0], EntityMessage::SpawnEntity(e, _) if e == entity));
    assert!(matches!(out[1], EntityMessage::InsertComponent(e, k) if e == entity && k == comp));
} 