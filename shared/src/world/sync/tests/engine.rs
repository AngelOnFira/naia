#![cfg(test)]

use crate::world::{sync::Engine, component::component_kinds::ComponentKind, entity::{entity_message::EntityMessage, local_entity::RemoteEntity}};

struct AssertList {
    asserts: Vec<EntityMessage<RemoteEntity>>,
}

impl AssertList {
    fn new() -> Self {
        Self { asserts: Vec::new() }
    }

    fn push(&mut self, msg: EntityMessage<RemoteEntity>) {
        self.asserts.push(msg);
    }

    fn check(&self, engine: &mut Engine<RemoteEntity>) {

        let out = engine.drain();

        assert_eq!(self.asserts.len(), out.len(), "Expected {} messages, got {}", self.asserts.len(), out.len());

        for (i, assert_msg) in self.asserts.iter().enumerate() {
            assert_eq!(assert_msg, &out[i], "At index {}, output message: {:?} not equal to expected message: {:?}", i, &out[i], assert_msg);
        }
    }
}

#[test]
fn engine_basic() {

    let mut engine: Engine<RemoteEntity> = Engine::new();
    
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.push(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(2, EntityMessage::InsertComponent(entity, comp));
    engine.push(3, EntityMessage::RemoveComponent(entity, comp));
    engine.push(4, EntityMessage::DespawnEntity(entity));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));
    asserts.push(EntityMessage::RemoveComponent(entity, comp));
    asserts.push(EntityMessage::DespawnEntity(entity));

    asserts.check(&mut engine);
}

#[test]
fn engine_invalidate_spawn_event() {

    let mut engine: Engine<RemoteEntity> = Engine::new();
    
    let entity = RemoteEntity::new(1);

    engine.push(2, EntityMessage::DespawnEntity(entity));
    engine.push(1, EntityMessage::SpawnEntity(entity, Vec::new()));

    let asserts = AssertList::new();
    asserts.check(&mut engine);
} 

#[test]
fn engine_invalidate_insert_event() {

    let mut engine: Engine<RemoteEntity> = Engine::new();
    
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.push(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(3, EntityMessage::RemoveComponent(entity, comp));
    engine.push(2, EntityMessage::InsertComponent(entity, comp));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn engine_entity_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::new();

    let entityA = RemoteEntity::new(1);
    let entityB = RemoteEntity::new(2);
    let entityC = RemoteEntity::new(2);

    engine.push(3, EntityMessage::SpawnEntity(entityA, Vec::new()));
    engine.push(2, EntityMessage::SpawnEntity(entityB, Vec::new()));
    engine.push(1, EntityMessage::SpawnEntity(entityC, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entityA, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entityB, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entityC, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn engine_component_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::new();

    let entity = RemoteEntity::new(1);
    let compA = ComponentKind::from(std::any::TypeId::of::<u8>());
    let compB = ComponentKind::from(std::any::TypeId::of::<u8>());
    let compC = ComponentKind::from(std::any::TypeId::of::<u8>());

    engine.push(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.push(4, EntityMessage::InsertComponent(entity, compA));
    engine.push(3, EntityMessage::InsertComponent(entity, compB));
    engine.push(2, EntityMessage::InsertComponent(entity, compC));

    // Check order
    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, compA));
    asserts.push(EntityMessage::InsertComponent(entity, compB));
    asserts.push(EntityMessage::InsertComponent(entity, compC));

    asserts.check(&mut engine);
}