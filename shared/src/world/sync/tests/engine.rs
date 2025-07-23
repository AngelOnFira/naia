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

        let out = engine.receive_messages();

        assert_eq!(self.asserts.len(), out.len(), "Expected {} messages, got {}", self.asserts.len(), out.len());

        for (i, assert_msg) in self.asserts.iter().enumerate() {
            assert_eq!(assert_msg, &out[i], "At index {}, output message: {:?} not equal to expected message: {:?}", i, &out[i], assert_msg);
        }
    }
}

#[test]
fn engine_basic() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp));
    engine.accept_message(3, EntityMessage::RemoveComponent(entity, comp));
    engine.accept_message(4, EntityMessage::DespawnEntity(entity));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));
    asserts.push(EntityMessage::RemoveComponent(entity, comp));
    asserts.push(EntityMessage::DespawnEntity(entity));

    asserts.check(&mut engine);
}

#[test]
fn engine_invalidate_spawn_event() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    
    let entity = RemoteEntity::new(1);

    engine.accept_message(2, EntityMessage::DespawnEntity(entity));
    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));

    let asserts = AssertList::new();
    asserts.check(&mut engine);
} 

#[test]
fn engine_invalidate_insert_event() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u32>());

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(3, EntityMessage::RemoveComponent(entity, comp));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn engine_entity_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entityA = RemoteEntity::new(1);
    let entityB = RemoteEntity::new(2);
    let entityC = RemoteEntity::new(2);

    engine.accept_message(3, EntityMessage::SpawnEntity(entityA, Vec::new()));
    engine.accept_message(2, EntityMessage::SpawnEntity(entityB, Vec::new()));
    engine.accept_message(1, EntityMessage::SpawnEntity(entityC, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entityA, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entityB, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entityC, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn engine_component_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let compA = ComponentKind::from(std::any::TypeId::of::<u8>());
    let compB = ComponentKind::from(std::any::TypeId::of::<u8>());
    let compC = ComponentKind::from(std::any::TypeId::of::<u8>());

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(4, EntityMessage::InsertComponent(entity, compA));
    engine.accept_message(3, EntityMessage::InsertComponent(entity, compB));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, compC));

    // Check order
    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, compA));
    asserts.push(EntityMessage::InsertComponent(entity, compB));
    asserts.push(EntityMessage::InsertComponent(entity, compC));

    asserts.check(&mut engine);
}

#[test]
fn wrap_ordering_simple() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u8>());

    // Pre-wrap packet (high seq)
    engine.accept_message(65_534, EntityMessage::SpawnEntity(entity, Vec::new()));
    // Post-wrap packet (low seq)
    engine.accept_message(0, EntityMessage::InsertComponent(entity, comp));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));

    asserts.check(&mut engine);
}

#[test]
fn backlog_window_cap() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    // Reduced max_in_flight for testing
    engine.config.max_in_flight = 4;

    // Push 5 out-of-order packets within half-range window size 4
    for seq in 1..=5 {
        let entity = RemoteEntity::new(seq);
        engine.accept_message(seq, EntityMessage::SpawnEntity(entity, Vec::new()));
    }

    let out = engine.receive_messages();
    assert_eq!(out.len(), engine.config.max_in_flight as usize, "5th packet should be dropped due to window cap");
}

#[test]
fn guard_band_flush() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    let entity = RemoteEntity::new(1);

    let near_flush_seq = engine.config.flush_threshold - 2;
    let wrap_beyond_seq = engine.config.flush_threshold + 1;

    engine.accept_message(near_flush_seq, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(wrap_beyond_seq, EntityMessage::SpawnEntity(entity, Vec::new()));

    // We expect only the later packet to be delivered
    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.check(&mut engine);
}

#[test]
fn noop_safe() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    engine.accept_message(10, EntityMessage::Noop);

    let asserts = AssertList::new();
    asserts.check(&mut engine);
}

#[test]
fn generation_gate_reuse() {
    let mut engine: Engine<RemoteEntity> = Engine::default();
    let entity = RemoteEntity::new(1);

    // First lifetime
    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(2, EntityMessage::DespawnEntity(entity));

    // Wrap…
    engine.accept_message(65_535, EntityMessage::Noop); // filler

    // Second lifetime after wrap
    engine.accept_message(0, EntityMessage::SpawnEntity(entity, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn backlog_drains_on_prereq_arrival() {
    let mut engine: Engine<RemoteEntity> = Engine::default();
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u16>());

    // Insert arrives first, should backlog
    engine.accept_message(5, EntityMessage::InsertComponent(entity, comp));
    // Spawn arrives second
    engine.accept_message(6, EntityMessage::SpawnEntity(entity, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));

    asserts.check(&mut engine);
}

#[test]
fn component_remove_before_insert() {
    let mut engine: Engine<RemoteEntity> = Engine::default();
    let entity = RemoteEntity::new(1);
    let comp = ComponentKind::from(std::any::TypeId::of::<u8>());

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(3, EntityMessage::RemoveComponent(entity, comp));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.check(&mut engine);
}

#[test]
fn empty_drain_safe() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    // Drain when empty
    let out1 = engine.receive_messages();
    assert!(out1.is_empty());

    // After guard-band purge scenario – no panic even if drain again
    let out2 = engine.receive_messages();
    assert!(out2.is_empty());
}