// Tests for the new sync module
#![cfg(test)]

use crate::world::sync::Engine;
use crate::world::entity::entity_message::EntityMessage;
use crate::world::entity::local_entity::RemoteEntity;
use crate::world::component::component_kinds::ComponentKind;

#[test]
fn engine_basic() {
    struct DummyRoot;
    let mut engine: Engine<DummyRoot> = Engine::new();

    // Feed two simple events
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(EntityMessage::InsertComponent(entity, comp));

    // Drain context – stub engine emits nothing for now
    let out = engine.context().drain();
    assert!(out.is_empty(), "Stub engine should not output events yet");
} 