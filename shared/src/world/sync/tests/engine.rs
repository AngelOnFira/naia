#![cfg(test)]

use crate::EntityAuthStatus;
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

struct ComponentType<const T: u8>;

fn component_kind<const T: u8>() -> ComponentKind {
    ComponentKind::from(std::any::TypeId::of::<ComponentType<T>>())
}

#[test]
fn engine_basic() {

    let mut engine: Engine<RemoteEntity> = Engine::default();
    
    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();

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
fn engine_entity_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity_a = RemoteEntity::new(1);
    let entity_b = RemoteEntity::new(2);
    let entity_c = RemoteEntity::new(3);

    engine.accept_message(3, EntityMessage::SpawnEntity(entity_a, Vec::new()));
    engine.accept_message(2, EntityMessage::SpawnEntity(entity_b, Vec::new()));
    engine.accept_message(1, EntityMessage::SpawnEntity(entity_c, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity_a, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entity_b, Vec::new()));
    asserts.push(EntityMessage::SpawnEntity(entity_c, Vec::new()));

    asserts.check(&mut engine);
}

#[test]
fn engine_component_channels_do_not_block() {

    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let comp_a = component_kind::<1>();
    let comp_b = component_kind::<2>();
    let comp_c = component_kind::<3>();

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(4, EntityMessage::InsertComponent(entity, comp_a));
    engine.accept_message(3, EntityMessage::InsertComponent(entity, comp_b));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp_c));

    // Check order
    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp_a));
    asserts.push(EntityMessage::InsertComponent(entity, comp_b));
    asserts.push(EntityMessage::InsertComponent(entity, comp_c));

    asserts.check(&mut engine);
}

#[test]
fn wrap_ordering_simple() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();

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
fn backlog_drains_on_prereq_arrival() {
    let mut engine: Engine<RemoteEntity> = Engine::default();
    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();

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
fn entity_despawn_before_spawn() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();

    // Despawn before spawn
    engine.accept_message(3, EntityMessage::DespawnEntity(entity));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp));
    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));
    asserts.push(EntityMessage::DespawnEntity(entity));
    asserts.check(&mut engine);
}

#[test]
fn component_remove_before_insert() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);
    let comp = component_kind::<1>();

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(3, EntityMessage::RemoveComponent(entity, comp));
    engine.accept_message(2, EntityMessage::InsertComponent(entity, comp));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::InsertComponent(entity, comp));
    asserts.push(EntityMessage::RemoveComponent(entity, comp));
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

#[test]
fn entity_auth_basic() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);

    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(2, EntityMessage::PublishEntity(entity));
    engine.accept_message(3, EntityMessage::EnableDelegationEntity(entity));
    engine.accept_message(4, EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Granted));
    engine.accept_message(5, EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Available));
    engine.accept_message(6, EntityMessage::DisableDelegationEntity(entity));
    engine.accept_message(7, EntityMessage::UnpublishEntity(entity));
    engine.accept_message(8, EntityMessage::DespawnEntity(entity));

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::PublishEntity(entity));
    asserts.push(EntityMessage::EnableDelegationEntity(entity));
    asserts.push(EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Granted));
    asserts.push(EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Available));
    asserts.push(EntityMessage::DisableDelegationEntity(entity));
    asserts.push(EntityMessage::UnpublishEntity(entity));
    asserts.push(EntityMessage::DespawnEntity(entity));
    asserts.check(&mut engine);
}

#[test]
fn entity_auth_scrambled() {
    let mut engine: Engine<RemoteEntity> = Engine::default();

    let entity = RemoteEntity::new(1);

    engine.accept_message(8, EntityMessage::DespawnEntity(entity));
    engine.accept_message(6, EntityMessage::DisableDelegationEntity(entity)); // this will never be received
    engine.accept_message(4, EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Granted)); // this will never be received
    engine.accept_message(2, EntityMessage::PublishEntity(entity));
    engine.accept_message(1, EntityMessage::SpawnEntity(entity, Vec::new()));
    engine.accept_message(3, EntityMessage::EnableDelegationEntity(entity)); // this will never be received
    engine.accept_message(5, EntityMessage::EntityUpdateAuthority(entity, EntityAuthStatus::Available)); // this will never be received
    engine.accept_message(7, EntityMessage::UnpublishEntity(entity)); // this will never be received

    let mut asserts = AssertList::new();
    asserts.push(EntityMessage::SpawnEntity(entity, Vec::new()));
    asserts.push(EntityMessage::PublishEntity(entity));
    asserts.push(EntityMessage::DespawnEntity(entity));
    asserts.check(&mut engine);
}